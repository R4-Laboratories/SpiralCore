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
    mailbox_launch::cast_fixture::{self, ReservedFixture},
    mailbox_service::guest_io::{self, private_channel::take_pair_readers},
};
use serde_json::{Value, json};
use std::sync::{Arc, mpsc};

// The SDK treats a LocalPath directory as an unpacked image and rewrites its
// configuration. Only the operator's already-materialized role may run here.
fn materialized_role(path: &std::path::Path) -> Result<()> {
    validate_rootfs_input(path)?;
    ensure!(
        path.file_name() == Some(std::ffi::OsStr::new("rootfs.ext4"))
            && fs::symlink_metadata(path)?.is_file(),
        "addressed role requires a materialized rootfs.ext4"
    );
    let dir = path.parent().context("role image directory missing")?;
    use std::os::unix::fs::MetadataExt;
    ensure!(
        dir.is_absolute() && fs::canonicalize(dir)? == dir,
        "role image directory must be canonical"
    );
    let uid = rustix::process::geteuid().as_raw();
    for ancestor in dir.ancestors() {
        let m = fs::symlink_metadata(ancestor)?;
        let protected = m.mode() & 0o022 == 0;
        let system_tmp = matches!(ancestor.to_str(), Some("/tmp" | "/var/tmp"))
            && m.uid() == 0
            && m.mode() & 0o1000 != 0;
        ensure!(
            m.is_dir() && (m.uid() == 0 || m.uid() == uid) && (protected || system_tmp),
            "role image namespace writable by another principal"
        );
    }
    for name in [
        "rootfs.verity",
        "rootfs.roothash",
        mvm_core::guest_sidecar::SIDECAR_FILENAME,
    ] {
        let sidecar = dir.join(name);
        validate_rootfs_input(&sidecar)?;
        let m = fs::symlink_metadata(sidecar)?;
        ensure!(
            m.is_file() && m.len() > 0,
            "incomplete materialized role metadata"
        );
        match name {
            "rootfs.roothash" => ensure!(m.len() <= 65, "role verity root too large"),
            mvm_core::guest_sidecar::SIDECAR_FILENAME => {
                ensure!(m.len() <= 65536, "role metadata too large")
            }
            _ => (),
        }
    }
    ensure!(
        mvm_build::run_image::rootfs_build_is_complete(path),
        "role metadata failed MVM validation"
    );
    let hash = fs::read_to_string(dir.join("rootfs.roothash"))?;
    let hash = hash.trim();
    ensure!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid role verity root"
    );
    Ok(())
}

// Explicit local operator evidence. The private file is never included in a
// report or printed; it lives inside the already validated private test home.
struct PrivateLaunchDiagnostics(std::sync::Mutex<fs::File>);
impl PrivateLaunchDiagnostics {
    fn open(home: &std::path::Path) -> Result<Option<Arc<Self>>> {
        match std::env::var_os("SPIRALCORE_PRIVATE_LAUNCH_DIAGNOSTICS").as_deref() {
            None => Ok(None),
            Some(v) if v == "1" => Ok(Some(Arc::new(Self::create(home)?))),
            _ => anyhow::bail!("private diagnostics option must be 1 or absent"),
        }
    }
    fn create(home: &std::path::Path) -> Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(home.join("addressed-sdk.private.jsonl"))?;
        Ok(Self(std::sync::Mutex::new(f)))
    }
    fn record(&self, phase: &'static str, role: &'static str, detail: &str) -> Result<()> {
        use std::io::Write;
        let end = detail.floor_char_boundary(4096);
        let mut f = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("private diagnostic writer unavailable"))?;
        serde_json::to_writer(
            &mut *f,
            &json!({"phase":phase,"role":role,"cause":&detail[..end],"truncated":end<detail.len()}),
        )?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        Ok(())
    }
}
fn unstarted_diagnostics<T>(
    claim: &std::path::Path,
    original: &fs::File,
    prepare: impl FnOnce() -> Result<T>,
) -> Result<T> {
    match prepare() {
        Ok(value) => Ok(value),
        Err(error) => {
            use std::os::unix::fs::MetadataExt;
            let held = original.metadata()?;
            let current = fs::symlink_metadata(claim)?;
            ensure!(
                (held.dev(), held.ino()) == (current.dev(), current.ino()),
                "unstarted original home claim changed"
            );
            fs::remove_file(claim)?;
            Err(error)
        }
    }
}

fn observe_refusal(
    sink: &Option<Arc<PrivateLaunchDiagnostics>>,
    phase: &'static str,
    role: &'static str,
    detail: impl FnOnce() -> String,
) {
    if let Some(sink) = sink
        && sink.record(phase, role, &detail()).is_err()
    {
        eprintln!("addressed private diagnostic unavailable role={role} phase={phase}");
    }
}

fn launch_diagnostic(err: &mvm_core::client::MvmError) -> (&'static str, &'static str) {
    use mvm_core::client::MvmError;
    let detail = match err {
        MvmError::Backend { reason } if reason.contains("needs a verified workload kernel") => {
            "workload_kernel_unverified"
        }
        _ => "sdk_error",
    };
    (err.code(), detail)
}

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

/// Two normal entrypoint calls with one coordinator-owned, single-use turn.
/// The clear message identifier remains inside the authorized guest endpoint.
pub fn run(
    first: &Config,
    second: &Config,
    expected: &[u8],
    record_failed: impl FnOnce(&Value) -> Result<()>,
    record_late: impl FnOnce(std::process::ExitStatus) -> Result<()>,
) -> Result<Value> {
    first.validate()?;
    second.validate()?;
    mvm_client::validate_vm_name(&first.name)?;
    mvm_client::validate_vm_name(&second.name)?;
    materialized_role(&first.rootfs)?;
    materialized_role(&second.rootfs)?;
    ensure!(
        first.home == second.home
            && fs::canonicalize(&first.rootfs)? != fs::canonicalize(&second.rootfs)?
            && first.name != second.name
            && !first.mailbox_hex.eq_ignore_ascii_case(&second.mailbox_hex)
            && (1..=1024).contains(&expected.len()),
        "two original roles and a bounded addressed body required"
    );
    let (helper, _image, claim, held) = claim_fresh_home(first)?;
    let diagnostics = unstarted_diagnostics(&claim, &held, || {
        PrivateLaunchDiagnostics::open(&first.home)
    })?;
    let prepare_diagnostics = diagnostics.clone();
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
        )
        .inspect_err(|e| {
            observe_refusal(
                &prepare_diagnostics,
                "signed_direction_prepare",
                "pair",
                || format!("{e:#}"),
            )
        })?;
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
    let reqs = [
        ("source", request(first)?),
        ("destination", request(second)?),
    ];
    let vm_runtime = runtime()?;
    let start = Instant::now();
    let cpu = ProcessTime::now();
    let mut installed =
        InstalledWitness::start_configured(first, &helper, Some(names), true, |p| {
            p.with_original_cast_pair(names[0], names[1], Duration::from_secs(30), prepare)
        })?;
    let backend = LocalBackend::with_hypervisor("firecracker")
        .with_installed_mailbox(installed.provider.clone());
    // Each synchronous SDK reservation must be able to meet its peer.
    let launches = thread::scope(|scope| {
        let jobs = reqs.map(|(role, req)| {
            let provider = installed.provider.clone();
            let diagnostics = diagnostics.clone();
            scope.spawn(move || -> Result<LaunchOutcome> {
                let local =
                    LocalBackend::with_hypervisor("firecracker").with_installed_mailbox(provider);
                let rt = runtime().inspect_err(|_| {
                    eprintln!("addressed role={role} launch=failed category=runtime_init")
                })?;
                match rt.block_on(local.launch(req)) {
                    Ok(vm) => {
                        eprintln!("addressed role={role} launch=admitted");
                        Ok(vm)
                    }
                    Err(error) => {
                        observe_refusal(&diagnostics, "sdk_launch", role, || error.to_string());
                        let (code, detail) = launch_diagnostic(&error);
                        eprintln!(
                            "addressed role={role} launch=failed code={code} category={detail}"
                        );
                        anyhow::bail!("SDK launch refused ({code}, {detail})")
                    }
                }
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
                signed.forward_once(Instant::now() + Duration::from_secs(14))
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
    let drain_failed = coordinator.is_none();
    let record = json!({"status":if errors.is_empty() && readback.is_some(){"passed"}else{"failed"},
        "scope":"Linux Firecracker same-host addressed Cast, ephemeral Submit and exact destination workload readback",
        "expected_bytes":expected.len(),"expected_sha256":digest(expected),"readback":readback,"message_id_correlation_across_guests":null,"participants":rows,"coordinator_exit":coordinator,
        "setup_wall_ns":setup_ns,"lifecycle_wall_ns":start.elapsed().as_nanos(),
        "host_driver_process_cpu_ns":cpu.elapsed().as_nanos(),"guest_stream_scope":guest_io::SCOPE,
        "wan_bytes":null,"send_only_ns":null,"errors":errors});
    if drain_failed {
        // Persist the observation deadline before waiting on the original
        // Child. Even an output error must not relinquish the process handle.
        installed.record_and_collect_late(|| record_failed(&record), record_late)?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_complete_materialized_role_reaches_sdk() -> Result<()> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("sc-addressed-role-{}-{n}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&dir)?;
        struct Remove(std::path::PathBuf);
        impl Drop for Remove {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Remove(dir.clone());
        let image = dir.join("rootfs.ext4");
        assert!(materialized_role(&dir).is_err());
        for file in [
            "rootfs.ext4",
            "rootfs.verity",
            "rootfs.roothash",
            mvm_core::guest_sidecar::SIDECAR_FILENAME,
        ] {
            assert!(materialized_role(&image).is_err());
            let path = dir.join(file);
            match file {
                mvm_core::guest_sidecar::SIDECAR_FILENAME => {
                    mvm_build::builder_vm::GuestSidecar::for_oci_run("test", false, true)
                        .write_to_dir(&dir)?;
                }
                "rootfs.roothash" => fs::write(&path, "ab".repeat(32))?,
                _ => fs::write(&path, [1])?,
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        materialized_role(&image)?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o770))?;
        assert!(materialized_role(&image).is_err());
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        let hash = dir.join("rootfs.roothash");
        fs::write(&hash, "a".repeat(4096))?;
        assert!(materialized_role(&image).is_err());
        fs::write(hash, "ab".repeat(32))?;
        let tree = dir.join("rootfs.verity");
        fs::remove_file(&tree)?;
        symlink(&image, &tree)?;
        assert!(materialized_role(&image).is_err());
        Ok(())
    }

    #[test]
    fn private_cause_retains_phase_with_exclusive_private_bounded_records() -> Result<()> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let home =
            std::env::temp_dir().join(format!("sc-private-refusal-{}-{stamp}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&home)?;
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(home.clone());
        let sink = PrivateLaunchDiagnostics::create(&home)?;
        let path = home.join("addressed-sdk.private.jsonl");
        assert_eq!(fs::metadata(&path)?.mode() & 0o777, 0o600);
        let claim = home.join("original-claim");
        let held = fs::File::create_new(&claim)?;
        assert!(
            unstarted_diagnostics(&claim, &held, || PrivateLaunchDiagnostics::create(&home))
                .is_err()
        );
        assert!(!claim.exists());
        let replaced = fs::File::create_new(&claim)?;
        fs::rename(&claim, home.join("retained-prior"))?;
        fs::write(&claim, b"replacement")?;
        assert!(
            unstarted_diagnostics(&claim, &replaced, || PrivateLaunchDiagnostics::create(
                &home
            ))
            .is_err()
        );
        assert_eq!(fs::read(&claim)?, b"replacement");
        let cause = "x".repeat(4095)
            + "é
operator-only";
        sink.record("sdk_launch", "source", &cause)?;
        sink.record("signed_direction_prepare", "pair", "second-layer fallback")?;
        let bytes = fs::read_to_string(path)?;
        let rows = bytes
            .lines()
            .map(serde_json::from_str::<Value>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["truncated"], true);
        assert_eq!(rows[0]["cause"].as_str().unwrap().len(), 4095);
        assert_eq!(
            rows[1],
            json!({"phase":"signed_direction_prepare","role":"pair","cause":"second-layer fallback","truncated":false})
        );
        Ok(())
    }

    #[test]
    fn predrain_diagnostic_contains_only_stable_categories() {
        use mvm_core::client::MvmError;
        assert_eq!(
            launch_diagnostic(&MvmError::Backend {
                reason: "firecracker needs a verified workload kernel at /private/secret".into()
            }),
            ("BACKEND_ERROR", "workload_kernel_unverified")
        );
        assert_eq!(
            launch_diagnostic(&MvmError::Unauthorized {
                reason: "secret-do-not-emit".into()
            }),
            ("UNAUTHORIZED", "sdk_error")
        );
    }
}
