//! Two real local guests, two separately admitted own-mailbox messages, one installed coordinator.
use super::*;
use crate::paired_body;
use mvm_client::launch::LaunchOutcome;
use mvm_core::protocol::mailbox_worker_control::{MailboxWorkerExit, MailboxWorkerState};
use mvm_hostd::mailbox_process::owned_client::InstalledMailboxClient;
use mvm_hostd::mailbox_service::guest_io::{self, private_channel::take_reader};
use serde_json::{Value, json};
use std::os::unix::net::UnixDatagram;

struct Lane<'a> {
    cfg: &'a Config,
    body: Vec<u8>,
    option: &'static str,
    metrics: Report,
    original: Option<LaunchOutcome>,
    definition: Option<DefinitionGuard>,
    signed: Option<InstalledMailboxClient>,
    reader: Option<UnixDatagram>,
    admission: Option<Value>,
    errors: Vec<String>,
    cleanup_wall_ns: Option<u128>,
}
fn hex_id(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl<'a> Lane<'a> {
    fn new(cfg: &'a Config, body: Vec<u8>, option: &'static str) -> Self {
        let metrics = new_report(&body);
        Self {
            cfg,
            body,
            option,
            metrics,
            original: None,
            definition: None,
            signed: None,
            reader: None,
            admission: None,
            errors: Vec::new(),
            cleanup_wall_ns: None,
        }
    }

    async fn launch(
        &mut self,
        backend: &LocalBackend,
        installed: &InstalledWitness,
        executable: &HeldExecutable,
        prior_helper: Option<u32>,
    ) -> Result<()> {
        let start = Instant::now();
        let request =
            request_with_command(self.cfg, vec![PRODUCER_PATH.into(), self.option.into()])?;
        self.original = Some(backend.launch(request).await?);
        let launched = self.original.as_ref().expect("original launch recorded");
        self.definition = Some(DefinitionGuard::capture(
            &mvm_core::config::machine_spec_path(&launched.machine.name),
        )?);
        self.metrics.plan_id = Some(launched.plan_id.clone());
        let client = installed.original_owner(self.cfg)?;
        let r = client.acquire_request();
        let original = client.owner().context("original helper owner missing")?;
        let grant = launched
            .admitted
            .plan()
            .mailbox
            .context("signed plan missing mailbox")?;
        ensure!(
            r.vm_id == self.cfg.name
                && r.workload_id == self.cfg.name
                && r.mailbox_grant == grant
                && hex_id(grant.mailbox.as_bytes()).eq_ignore_ascii_case(&self.cfg.mailbox_hex)
                && launched.admitted.plan_id().0 == launched.plan_id
                && launched.plan_id == format!("sha256:{}", hex_id(&r.plan_commitment)),
            "original signed stop locator differs from admitted plan"
        );
        self.admission = Some(json!({"plan_id": launched.plan_id,
            "vm": r.vm_id, "workload": r.workload_id, "mailbox": hex_id(grant.mailbox.as_bytes()),
            "authority_verifying_key": hex_id(&grant.authority_key), "execution_epoch": grant.epoch.get(),
            "owner_generation": original.generation, "owner_context": hex_id(&original.context_commitment)}));
        self.signed = Some(client);
        self.require_serving()?;
        // Read this offer before the other worker can be delegated. Save only
        // this privately tagged reader until the matching signed original exit.
        self.reader = Some(take_reader(
            &installed.report_socket,
            self.signed.as_ref().unwrap().owner().unwrap(),
        )?);
        let pidfile =
            mvm_backends::fc::fc_pid_path(&self.cfg.name).context("Firecracker record missing")?;
        self.metrics
            .processes
            .push(ProcessObservation::capture(&pidfile, None)?);
        self.metrics
            .processes
            .push(installed.observe_new_helper(executable, prior_helper)?);
        self.metrics.setup_wall_ns = Some(start.elapsed().as_nanos());
        Ok(())
    }

    fn require_serving(&self) -> Result<()> {
        let client = self.signed.as_ref().context("signed worker unavailable")?;
        ensure!(
            client.query(Instant::now() + Duration::from_secs(2))? == MailboxWorkerState::Serving,
            "original worker no longer serving"
        );
        Ok(())
    }

    fn sample_live(&mut self) -> Result<()> {
        ensure!(
            self.metrics.processes.len() == 2,
            "expected original VMM and helper"
        );
        self.require_serving()?;
        for original in &mut self.metrics.processes {
            original.sample_before_stop()?;
            ensure!(
                original.before_stop.is_some(),
                "original process exited early"
            );
        }
        Ok(())
    }

    fn receive(&mut self) -> Result<()> {
        receive_payload(self.cfg, &self.body, Instant::now(), &mut self.metrics)
    }

    async fn retire(&mut self, backend: &LocalBackend) {
        let start = Instant::now();
        let clean = clean_original(backend, &self.original, &self.definition).await;
        self.metrics.stop = clean.stop.into();
        self.metrics.cleanup = clean.cleanup.into();
        if let Some(e) = clean.error {
            self.errors.push(e);
        }
        if self.original.is_none() {
            self.errors
                .push("successful original launch absent; retained pending state".into());
            return;
        }
        let until = Instant::now() + Duration::from_secs(5);
        for process in &mut self.metrics.processes {
            if let Err(e) = process.observe_exit(until) {
                self.errors.push(format!("original process exit: {e:#}"));
            }
        }
        let terminal = (|| -> Result<()> {
            let original = self.signed.as_ref().context("signed original missing")?;
            ensure!(
                matches!(
                    original.query(Instant::now() + Duration::from_secs(2))?,
                    MailboxWorkerState::Collected {
                        exit: MailboxWorkerExit::Code { code: 0 }
                    }
                ),
                "signed original helper exit unconfirmed"
            );
            self.metrics.installed_helper = Some("signed original exit 0".into());
            let owner = original.owner().context("missing signed original")?;
            let counts = guest_io::receive_original(
                self.reader
                    .as_ref()
                    .context("private original reader missing")?,
                owner,
            )?;
            if self.metrics.exact_readback {
                ensure!(
                    counts.guest_to_host_read_bytes >= self.body.len() as u64
                        && counts.host_to_guest_write_accepted_bytes > 0,
                    "guest stream measurement inconsistent with verified delivery"
                );
            }
            self.metrics.guest_mailbox_stream_counts = Some(counts);
            Ok(())
        })();
        if let Err(e) = terminal {
            self.errors.push(format!("original receipt: {e:#}"));
        }
        for path in [
            mvm_core::config::vm_state_dir_at(&self.cfg.home, &self.cfg.name),
            mvm_core::config::machine_state_dir(&self.cfg.name),
        ] {
            match fs::symlink_metadata(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Ok(_) => self
                    .errors
                    .push("original runtime or persistent definition remains".into()),
                Err(e) => self.errors.push(format!("state removal unconfirmed: {e}")),
            }
        }
        self.cleanup_wall_ns = Some(start.elapsed().as_nanos());
    }

    fn record(&self) -> Value {
        let m = &self.metrics;
        json!({"name": self.cfg.name,"signed_admission": self.admission,
          "expected_bytes": self.body.len(),"expected_sha256":m.expected_sha256,
          "received_bytes":m.received_bytes,"received_sha256":m.received_sha256,"exact_readback":m.exact_readback,
          "setup_wall_ns":m.setup_wall_ns,"receive_wall_ns":m.receive_wall_ns,"receive_probes":m.receive_probes,
          "stop_and_receipt_wall_ns":self.cleanup_wall_ns,"stop":m.stop,"cleanup":m.cleanup,
          "signed_helper_exit":m.installed_helper,"guest_stream_scope":guest_io::SCOPE,
          "guest_stream_counts":m.guest_mailbox_stream_counts,"processes":m.processes,"errors":self.errors})
    }
}

pub async fn run_pair(first: &Config, second: &Config) -> Result<Value> {
    first.validate()?;
    second.validate()?;
    ensure!(
        first.home == second.home
            && first.rootfs == second.rootfs
            && first.name != second.name
            && !first.mailbox_hex.eq_ignore_ascii_case(&second.mailbox_hex),
        "pair requires shared fresh home and rootfs, two different VM names and mailbox IDs"
    );
    mvm_client::validate_vm_name(&second.name)?;
    let (helper, image, claim, _claim) = claim_fresh_home(first)?;
    let start = Instant::now();
    let cpu = ProcessTime::now();
    let mut installed =
        InstalledWitness::start_profile(first, &helper, Some([&first.name, &second.name]))?;
    let backend = LocalBackend::with_hypervisor("firecracker")
        .with_installed_mailbox(installed.provider.clone());
    let mut a = Lane::new(first, paired_body(1), "--pair-alpha");
    let mut b = Lane::new(second, paired_body(2), "--pair-beta");
    let mut errors = Vec::new();
    let mut both_serving = false;
    let mut sibling_served_after_stop = false;
    let launch = async {
        a.launch(&backend, &installed, &image, None).await?;
        let first_pid = a.metrics.processes[1].pid;
        b.launch(&backend, &installed, &image, Some(first_pid))
            .await?;
        a.sample_live()?;
        b.sample_live()?;
        let ca = a.signed.as_ref().unwrap();
        let cb = b.signed.as_ref().unwrap();
        let oa = ca.owner().unwrap();
        let ob = cb.owner().unwrap();
        ensure!(
            oa.daemon_boot_nonce == ob.daemon_boot_nonce
                && oa != ob
                && oa.context_commitment != ob.context_commitment
                && oa.generation != ob.generation
                && ca.acquire_request().mailbox_grant.authority_key
                    != cb.acquire_request().mailbox_grant.authority_key
                && ca.acquire_request().request_nonce != cb.acquire_request().request_nonce,
            "two signed admissions did not bind independent grants on one original"
        );
        both_serving = true;
        a.receive()?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if let Err(e) = launch {
        errors.push(format!("paired startup/first receive: {e:#}"));
    }
    a.retire(&backend).await;
    if both_serving {
        let sibling = (|| -> Result<()> {
            b.sample_live()?;
            b.receive()?;
            Ok(())
        })();
        if let Err(e) = sibling {
            errors.push(format!("sibling receive: {e:#}"));
        } else {
            sibling_served_after_stop =
                a.metrics.installed_helper.is_some() && a.metrics.cleanup == "canonical_cleanup_ok";
        }
    }
    b.retire(&backend).await;
    let coordinator = match installed.finish() {
        Ok(status) => Some(status.to_string()),
        Err(e) => {
            errors.push(format!("coordinator exit: {e:#}"));
            None
        }
    };
    let good = errors.is_empty()
        && a.errors.is_empty()
        && b.errors.is_empty()
        && both_serving
        && sibling_served_after_stop
        && a.metrics.exact_readback
        && b.metrics.exact_readback;
    if good {
        if let Err(e) = fs::remove_file(&claim) {
            errors.push(format!("claim release: {e}"));
        }
    }
    Ok(
        json!({"source":MVM_SOURCE,"status": if good && errors.is_empty() {"passed"} else {"failed"},
        "scope":"Two Linux Firecracker guests on one host; each producer addresses its own signed mailbox; host is the consumer.",
        "both_originals_serving":both_serving,"second_served_after_first_retirement":sibling_served_after_stop,
        "first":a.record(),"second":b.record(),"original_coordinator_exit":coordinator,
        "guest_to_guest_cast":"NOT_RUN: signed cross-address grants, destination journal and guest receive endpoint required",
        "guest_cpu":null,"wan_bytes":null,"send_only_ns":null,
        "host_orchestrator_cpu_ns":cpu.elapsed().as_nanos(),"lifecycle_wall_ns":start.elapsed().as_nanos(),"errors":errors}),
    )
}
