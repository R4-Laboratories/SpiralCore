//! Bounded native witness of one explicitly provisioned worker in the test home.
use super::{Config, HeldExecutable, ProcessObservation};
use anyhow::{Context, Result, bail, ensure};
use mvm_client::launch::{
    host_agent_mailbox::HostAgentMailboxProvisioner, mailbox_owner::InstalledMailboxProvisioner,
};
use mvm_core::protocol::broker_control::SignedControl;
use mvm_hostd::mailbox_process::owned_client::InstalledMailboxClient;
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    os::unix::fs::MetadataExt,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub(super) struct InstalledWitness {
    pub(super) provider: Arc<HostAgentMailboxProvisioner>,
    child: Arc<Mutex<Child>>,
}

impl InstalledWitness {
    pub(super) fn start(cfg: &Config, helper: &Path) -> Result<Self> {
        let install = helper
            .parent()
            .context("missing installed binary directory")?;
        let daemon = install.join("mvm-host-agent");
        for path in [&daemon, &install.join("mvm-signer-helper")] {
            HeldExecutable::open(path)?.require_protected()?;
        }
        mvm_vmm::host::host_agent_installation::descriptor(&cfg.home, "local")?;
        let signer = mvm_hostd::audit::host_keypair::load_or_init_at(&cfg.home.join("keys"))?;
        let socket = mvm_core::config::host_agent_control_socket("local");
        let agent_cfg = mvm_hostd::broker::daemon::HostAgentConfig {
            tenant_id: "local".into(),
            control_socket: socket.clone(),
            registration_journal_path: Some(
                mvm_core::config::host_agent_dir("local").join("registrations.json"),
            ),
            host_signer_public_key_path: signer.public_path,
            signer_helper_uds_path: mvm_core::config::host_agent_signer_helper_socket("local"),
            software_chain_key_path: signer.secret_path,
            max_frame_bytes: 65_536,
        };
        ensure!(
            !socket.try_exists()?,
            "fresh witness tenant socket required"
        );
        let mut child = Command::new(daemon)
            .arg("--owned-mailbox-v1")
            .env("MVM_HOST_AGENT_WORKER", "1")
            .env("MVM_HOST_AGENT_IDLE_TIMEOUT", "40")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        let startup = (|| -> Result<()> {
            let mut stdin = child
                .stdin
                .take()
                .context("original control configuration pipe missing")?;
            serde_json::to_writer(&mut stdin, &agent_cfg)?;
            stdin.flush()?;
            drop(stdin);
            let until = Instant::now() + Duration::from_secs(10);
            while !socket.try_exists()? {
                ensure!(
                    child.try_wait()?.is_none(),
                    "original installed coordinator exited during startup"
                );
                ensure!(
                    Instant::now() < until,
                    "installed control socket startup deadline"
                );
                thread::sleep(Duration::from_millis(15));
            }
            Ok(())
        })();
        // Even failed startup must observe the original child. Never terminate an
        // unrelated process from a filesystem PID or claim a successful drain.
        if let Err(error) = startup {
            if child.try_wait()?.is_none() {
                child.kill()?;
                child.wait()?;
            }
            return Err(error);
        }
        let child = Arc::new(Mutex::new(child));
        let provider = Arc::new(HostAgentMailboxProvisioner::from_original_worker(
            &cfg.home,
            child.clone(),
        )?);
        Ok(Self { provider, child })
    }

    pub(super) fn original_owner(&self, cfg: &Config) -> Result<InstalledMailboxClient> {
        let path = cfg
            .home
            .join("state/mailbox-owners-v1")
            .join(format!("{}.json", cfg.name));
        let file = fs::File::open(path)?;
        ensure!(file.metadata()?.len() <= 65_536, "oversize witness locator");
        let record: serde_json::Value = serde_json::from_reader(file)?;
        let locator: SignedControl = serde_json::from_value(
            record
                .get("stop")
                .context("persisted Stop locator absent")?
                .clone(),
        )?;
        let mut client = InstalledMailboxClient::from_stop_locator(&cfg.name, &locator)?;
        client.bind_peer(self.provider.resolve_original(&locator)?)?;
        Ok(client)
    }

    pub(super) fn observe_helper(&self, expected: &HeldExecutable) -> Result<ProcessObservation> {
        let parent = self
            .child
            .lock()
            .map_err(|_| anyhow::anyhow!("coordinator lost"))?
            .id();
        let mut pids = BTreeSet::new();
        for thread in fs::read_dir(format!("/proc/{parent}/task"))? {
            let path = thread?.path().join("children");
            match fs::read_to_string(path) {
                Ok(children) => {
                    for pid in children.split_whitespace() {
                        pids.insert(pid.parse::<u32>()?);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        let mut matched = None;
        for pid in pids {
            let path = format!("/proc/{pid}/exe");
            let Ok(m) = fs::metadata(&path) else { continue };
            if !expected.matches_inode(m.dev(), m.ino())? {
                continue;
            }
            ensure!(
                matched.is_none(),
                "more than one installed helper in single-guest witness"
            );
            matched = Some(ProcessObservation::capture_pid(pid, Some(expected))?);
        }
        matched.context("original installed daemon has no matching helper child")
    }

    pub(super) fn finish(&mut self) -> Result<ExitStatus> {
        let until = Instant::now() + Duration::from_secs(60);
        let mut child = self
            .child
            .lock()
            .map_err(|_| anyhow::anyhow!("coordinator child lost"))?;
        loop {
            if let Some(status) = child.try_wait()? {
                ensure!(
                    mvm_hostd::host_agent_idle::is_idle_shutdown(status.code()),
                    "original installed coordinator exited without completed idle drain: {status}"
                );
                return Ok(status);
            }
            if Instant::now() >= until {
                bail!("original coordinator idle retirement deadline");
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}
