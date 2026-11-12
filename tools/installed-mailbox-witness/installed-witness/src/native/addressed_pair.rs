//! Launch two independent SDK originals and observe one addressed Cast.
use super::{
    cast_readback::{OriginalParticipant, verify_original_pair},
    *,
};
use mvm_client::launch::{
    LaunchOutcome,
    cast_rendezvous::{OriginalCastRole, PrepareBoth},
};
use mvm_core::protocol::mailbox_worker_control::{MailboxWorkerExit, MailboxWorkerState};
use mvm_hostd::{
    mailbox_launch::cast_fixture::{self, ReservedFixture, SignedFixtureDirection},
    mailbox_service::guest_io::{self, private_channel::take_pair_readers},
};
use serde_json::{Value, json};
use std::sync::{Arc, mpsc};

fn request(cfg: &Config) -> Result<LaunchRequest> {
    let mut grant = mvm_client::admission::mailbox_grant::resolve(Some(&cfg.mailbox_hex), None)?
        .context("missing requested mailbox")?;
    grant.local_service = Some(cast_fixture::local_service());
    // The image's installed workload receives ordinary entrypoint Calls.
    // An ad-hoc command here would select a different grant/entrypoint mode.
    Ok(LaunchRequest::builder(
        LifecycleMode::Persistent,
        RootfsSource::LocalPath(cfg.rootfs.clone()),
    )
    .name(&cfg.name)
    .backend("firecracker")
    .cpus(1)
    .memory_mib(256)
    .ttl_seconds(180)
    .mailbox(grant)
    .build()?)
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

/// The caller supplies the installed coordinator's opaque original-owner carrier.
/// A successful carrier returns only a transient disposition.
pub fn run(
    first: &Config,
    second: &Config,
    expected: &[u8],
    route: impl FnOnce(
        &SignedFixtureDirection,
        OriginalParticipant<'_>,
        OriginalParticipant<'_>,
    ) -> Result<()>,
) -> Result<Value> {
    first.validate()?;
    second.validate()?;
    mvm_client::validate_vm_name(&first.name)?;
    mvm_client::validate_vm_name(&second.name)?;
    validate_rootfs_input(&second.rootfs)?;
    ensure!(
        first.home == second.home
            && fs::canonicalize(&first.rootfs)? != fs::canonicalize(&second.rootfs)?
            && first.name != second.name
            && !first.mailbox_hex.eq_ignore_ascii_case(&second.mailbox_hex)
            && (1..=1024).contains(&expected.len()),
        "two original roles and a bounded addressed body required"
    );
    let (helper, _image, claim, held) = claim_fresh_home(first)?;
    let (tx, rx) = mpsc::sync_channel(1);
    let home = first.home.clone();
    let prepare: Arc<PrepareBoth> = Arc::new(move |a, b, deadline| {
        let pair = cast_fixture::prepare(
            &home,
            &held,
            ReservedFixture {
                original: a.participant(),
                config: a.config(),
            },
            ReservedFixture {
                original: b.participant(),
                config: b.config(),
            },
            deadline,
        )?;
        tx.try_send(pair.retained)
            .map_err(|_| anyhow::anyhow!("pair issuer already used or closed"))?;
        Ok([
            OriginalCastRole {
                boot: pair.source.0,
                role: pair.source.1,
            },
            OriginalCastRole {
                boot: pair.destination.0,
                role: pair.destination.1,
            },
        ])
    });
    let names = [first.name.as_str(), second.name.as_str()];
    let cfg = [first, second];
    let reqs = [request(first)?, request(second)?];
    let vm_runtime = runtime()?;
    let start = Instant::now();
    let cpu = ProcessTime::now();
    let mut installed = InstalledWitness::start_configured(first, &helper, Some(names), |p| {
        p.with_original_cast_pair(names[0], names[1], Duration::from_secs(30), prepare)
    })?;
    let backend = LocalBackend::with_hypervisor("firecracker")
        .with_installed_mailbox(installed.provider.clone());
    // Each synchronous SDK reservation must be able to meet its peer.
    let launches = thread::scope(|scope| {
        let jobs = reqs.map(|req| {
            let provider = installed.provider.clone();
            scope.spawn(move || -> Result<LaunchOutcome> {
                let local =
                    LocalBackend::with_hypervisor("firecracker").with_installed_mailbox(provider);
                Ok(runtime()?.block_on(local.launch(req))?)
            })
        });
        jobs.map(|job| {
            job.join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("original launch panicked")))
        })
    });
    let setup_ns = start.elapsed().as_nanos();
    let mut errors = Vec::new();
    let mut originals = [None, None];
    let mut defs = [None, None];
    let mut controls = [None, None];
    for (i, r) in launches.into_iter().enumerate() {
        match r {
            Err(e) => errors.push(format!("{} launch: {e:#}", names[i])),
            Ok(vm) => {
                originals[i] = Some(vm);
                match DefinitionGuard::capture(&mvm_core::config::machine_spec_path(names[i])) {
                    Ok(d) => defs[i] = Some(d),
                    Err(e) => errors.push(format!("{} definition: {e:#}", names[i])),
                }
                match installed.original_owner(cfg[i]) {
                    Ok(c) => controls[i] = Some(c),
                    Err(e) => errors.push(format!("{} signed owner: {e:#}", names[i])),
                }
            }
        }
    }
    let mut readers = None;
    let mut readback = None;
    if errors.is_empty() {
        let run = (|| -> Result<()> {
            let parts = [0, 1].map(|i| OriginalParticipant {
                launch: originals[i].as_ref().unwrap(),
                control: controls[i].as_ref().unwrap(),
            });
            let [a, b] = parts;
            readers = Some(take_pair_readers(
                &installed.report_socket,
                [
                    a.control.owner().context("missing source owner")?,
                    b.control.owner().context("missing destination owner")?,
                ],
            )?);
            let signed = rx
                .try_recv()
                .context("two original admissions did not issue one direction")?;
            readback = Some(verify_original_pair(a, b, expected, || {
                route(&signed, a, b)
            })?);
            Ok(())
        })();
        if let Err(e) = run {
            errors.push(format!("original A to B: {e:#}"));
        }
    }
    let mut rows = Vec::new();
    for i in 0..2 {
        let begin = Instant::now();
        let cleaned = vm_runtime.block_on(clean_original(&backend, &originals[i], &defs[i]));
        if let Some(e) = &cleaned.error {
            errors.push(format!("{} cleanup: {e}", names[i]));
        }
        let receipt = (|| -> Result<_> {
            let ctrl = controls[i]
                .as_ref()
                .context("original signed controller absent")?;
            let state = ctrl.query(Instant::now() + Duration::from_secs(2))?;
            ensure!(
                matches!(
                    state,
                    MailboxWorkerState::Collected {
                        exit: MailboxWorkerExit::Code { code: 0 }
                    }
                ),
                "missing successful signed original exit"
            );
            for path in [
                mvm_core::config::vm_state_dir_at(&cfg[i].home, names[i]),
                mvm_core::config::machine_state_dir(names[i]),
            ] {
                match fs::symlink_metadata(path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                    Ok(_) => anyhow::bail!("original persistent or volatile state still present"),
                }
            }
            let counts = guest_io::receive_original(
                &readers.as_ref().context("original meter missing")?[i],
                ctrl.owner().context("original owner missing")?,
            )?;
            if readback.is_some() {
                ensure!(
                    counts.guest_to_host_read_bytes > 0
                        && counts.host_to_guest_write_accepted_bytes > 0,
                    "empty guest stream measurement"
                );
            }
            Ok(counts)
        })();
        let counts = match receipt {
            Ok(x) => Some(x),
            Err(e) => {
                errors.push(format!("{} receipt: {e:#}", names[i]));
                None
            }
        };
        rows.push(
            json!({"name":names[i],"original_plan": originals[i].as_ref().map(|v|&v.plan_id),
            "stop":cleaned.stop,"cleanup":cleaned.cleanup,"guest_stream_counts":counts,
            "retirement_wall_ns":begin.elapsed().as_nanos()}),
        );
    }
    let coordinator = match installed.finish() {
        Ok(status) => Some(status.to_string()),
        Err(e) => {
            errors.push(format!("original coordinator drain: {e:#}"));
            None
        }
    };
    if errors.is_empty()
        && let Err(e) = fs::remove_file(&claim)
    {
        errors.push(format!("release fixture claim: {e}"));
    }
    Ok(
        json!({"status":if errors.is_empty() && readback.is_some(){"passed"}else{"failed"},
        "scope":"Linux Firecracker same-host addressed Cast, ephemeral Submit and exact destination workload readback",
        "readback":readback,"message_id_correlation_across_guests":null,"participants":rows,"coordinator_exit":coordinator,
        "setup_wall_ns":setup_ns,"lifecycle_wall_ns":start.elapsed().as_nanos(),
        "host_driver_process_cpu_ns":cpu.elapsed().as_nanos(),"guest_stream_scope":guest_io::SCOPE,
        "wan_bytes":null,"send_only_ns":null,"errors":errors}),
    )
}
