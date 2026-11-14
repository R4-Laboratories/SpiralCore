//! Original installed collector custody for the clean paired journal witness.
//! Enrollment, transfer and byte meters remain in the existing paired modules.
use super::*;
use mvm_client::launch::persistent::journal::{OwnedJournalStart, OwnedJournalStartFailure};
use mvm_core::{
    host_lifetime::EndpointPartitionLimits,
    vm_backend::{BackendKind, VmStartConfig},
};
use mvm_hostd::mailbox_process::original_journal::{JournalOwner, JournalStopOutcome};
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::{
        InstalledParentProvider, ParentLaunchOwner, PortableStartupQuote, TenantHelperLimits,
    },
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{io::Read, num::NonZeroU64};

pub(super) const HOST_RESIDENT_BYTES: u64 = 3 << 30;
pub(super) const START_MEMORY_BYTES: u64 = 1 << 30;
pub(super) const START_DISK_BYTES: u64 = 1 << 30;
pub(super) const STAGING_DISK_BYTES: u64 = 2400 << 20;
pub(super) const TENANT_PROCESS_AS: u64 = 192 << 20;

struct Role {
    pin: mvm_core::plan::PlanArtifact,
    archive: NonZeroU64,
}

pub struct Operator {
    roles: [Role; 2],
}

impl Operator {
    pub fn quote(&self, name: &str) -> Result<PortableStartupQuote> {
        let role = &self.roles[index(name)?];
        let additional = |ceiling: u64| {
            ceiling
                .checked_sub(role.archive.get())
                .and_then(NonZeroU64::new)
                .context("operator archive exceeds startup ceiling")
        };
        PortableStartupQuote::new(
            role.pin.clone(),
            role.archive,
            additional(START_MEMORY_BYTES)?,
            additional(START_DISK_BYTES)?,
        )
    }
}

/// Bounded private input; the canonical preparer authenticates the supplied pin.
pub fn operator(home: &Path) -> Result<(CollectorPair, Operator)> {
    const CAP: u64 = 256 << 10;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(home.join("collector-operator.json"))?;
    let meta = file.metadata()?;
    mvm_core::private_fs::require_current_owner(&meta)?;
    ensure!(
        meta.is_file() && meta.nlink() == 1 && meta.mode() & 0o077 == 0 && meta.len() <= CAP,
        "expected private bounded collector operator input"
    );
    let mut bytes = Vec::new();
    file.take(CAP + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 == meta.len(),
        "collector operator length changed"
    );
    let mut value: Value = serde_json::from_slice(&bytes)?;
    let object = value
        .as_object_mut()
        .context("collector operator must be an object")?;
    let roles = object.remove("roles").context("operator roles missing")?;
    let roles = roles.as_array().context("operator roles must be [A, B]")?;
    ensure!(
        roles.len() == 2,
        "operator requires exactly two role bundles"
    );
    let parse = |value: &Value| -> Result<Role> {
        let mut role = value.as_object().context("role must be an object")?.clone();
        let parsed = Role {
            pin: serde_json::from_value(role.remove("bundle_pin").context("bundle pin missing")?)?,
            archive: NonZeroU64::new(
                role.remove("decoded_archive_bytes")
                    .and_then(|n| n.as_u64())
                    .context("archive bytes missing")?,
            )
            .context("empty operator archive")?,
        };
        ensure!(role.is_empty(), "unknown collector role fields");
        Ok(parsed)
    };
    let packet = Operator {
        roles: [parse(&roles[0])?, parse(&roles[1])?],
    };
    ensure!(
        packet.roles[0].pin.bundle_sha256 != packet.roles[1].pin.bundle_sha256,
        "collector roles require distinct signed bundles"
    );
    ensure!(object.is_empty(), "unknown collector operator fields");
    // Validate both quote ceilings before creating installation/tenant effects.
    packet.quote(A)?;
    packet.quote(B)?;
    let (mut host, mut tenant) = InstalledConfig::two_endpoint_witness_profile().managed_limits();
    host.host_resident_bytes = HOST_RESIDENT_BYTES;
    tenant.host_resident_bytes = HOST_RESIDENT_BYTES;
    host.staging_disk_bytes = STAGING_DISK_BYTES;
    tenant.staging_disk_bytes = STAGING_DISK_BYTES;
    let config = InstalledConfig::with_funded_pair_limits(host, tenant)?;
    let partition = EndpointPartitionLimits {
        host_plaintext_bytes: 0,
        host_codec_bytes: 0,
        host_carrier_bytes: 0,
        host_metadata_bytes: 96 << 20,
        host_stack_bytes: 24 << 20,
        cache_disk_bytes: (8 << 20) + (32 << 10),
        staging_disk_bytes: 0,
        workers: 4,
        max_streams: 1,
    };
    let helper = TenantHelperLimits {
        process_address_space_bytes: NonZeroU64::new(TENANT_PROCESS_AS).unwrap(),
        audit_chain_bytes: NonZeroU64::new(1 << 20).unwrap(),
    };
    Ok((CollectorPair::new(home, config, partition, helper)?, packet))
}

/// Try both original journal owners once; all attempted outcomes are synced.
pub fn retire(pair: &mut CollectorPair, state: &mut State, home: &Path) -> Result<Value> {
    let history = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(home.join("paired-collector-retirement.jsonl"));
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut history = match history {
        Ok(file) => Some(file),
        Err(error) => {
            errors.push(format!("opening retirement evidence: {error}"));
            None
        }
    };
    for (i, name) in [A, B].into_iter().enumerate() {
        let until = Instant::now() + Duration::from_secs(20);
        let result = if !state.attempted[i] {
            Err(anyhow::anyhow!(
                "original journal was not attempted; preserve definition"
            ))
        } else if i == 0 {
            state
                .a
                .as_mut()
                .context("original source journal missing")
                .and_then(|owner| pair.stop(name, &mut JournalOwner::Source(owner), until))
        } else if let Some(owner) = &state.b {
            match owner.lock() {
                Ok(mut held) => pair.stop(name, &mut JournalOwner::Destination(&mut held), until),
                Err(_) => Err(anyhow::anyhow!("original destination journal poisoned")),
            }
        } else {
            state
                .b_pending
                .as_mut()
                .context("original destination journal missing")
                .and_then(|owner| pair.stop(name, &mut JournalOwner::Destination(owner), until))
        };
        let row = match result {
            Ok(outcome) => {
                if outcome.assisted || !outcome.helper_status.success() {
                    errors.push(format!(
                        "{name}: assisted or unsuccessful original helper collection"
                    ));
                }
                json!({"name":name,"collected":true,"assisted":outcome.assisted,
                    "success":outcome.helper_status.success(),"exit_code":outcome.helper_status.code()})
            }
            Err(error) => {
                errors.push(format!("{name}: {error:#}"));
                json!({"name":name,"collected":false,"error":format!("{error:#}")})
            }
        };
        // A recording error must not prevent the other owner being attempted.
        if let Err(error) = (|| -> Result<()> {
            let history = history
                .as_mut()
                .context("retirement evidence unavailable")?;
            serde_json::to_writer(&mut *history, &row)?;
            history.write_all(b"\n")?;
            history.sync_all()?;
            Ok(())
        })() {
            errors.push(format!("recording {name} retirement: {error:#}"));
        }
        rows.push(row);
    }
    ensure!(
        errors.is_empty(),
        "installed collector retirement: {}",
        errors.join("; ")
    );
    drop(state.a.take());
    drop(state.b_pending.take());
    drop(state.b.take());
    let retained = pair.close()?;
    Ok(
        json!({"original_helpers":rows,"retained_resources":retained,
        "runtime_state_removed":false,"definitions_removed":false}),
    )
}

#[derive(Default)]
struct Slot {
    parent: Option<ParentLaunchOwner>,
    started: Option<OwnedJournalStart>,
    failed: Option<OwnedJournalStartFailure>,
    endpoint: Option<mvm_client::EndpointRetirement>,
    endpoint_transferred: bool,
    stopped: bool,
}

/// One shared host/tenant ledger and one actual backend for both original names.
/// The caller supplies funded ceilings and authenticated canonical-startup quotes.
pub struct CollectorPair {
    supervisor: Option<InstalledSupervisor>,
    provider: Option<Arc<InstalledParentProvider>>,
    backend: Option<Arc<LocalBackend>>,
    slots: [Slot; 2],
}

fn index(name: &str) -> Result<usize> {
    match name {
        A => Ok(0),
        B => Ok(1),
        _ => anyhow::bail!("unexpected paired collector name"),
    }
}

impl CollectorPair {
    pub fn new(
        home: &Path,
        config: InstalledConfig,
        partition: EndpointPartitionLimits,
        helper: TenantHelperLimits,
    ) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "installed collector witness requires Linux"
        );
        let owner = InstallationOwner::initialize(
            &home.join("installation"),
            InstallationKey {
                installation: rand::random(),
                profile: config.profile_hash(),
            },
        )?;
        let supervisor = InstalledSupervisor::start(owner, config)?;
        let mut provider = InstalledParentProvider::new(&supervisor)?;
        provider.configure_telemetry_only(&supervisor, partition)?;
        provider.start_tenant_helper("local", helper)?;
        let backend = Arc::new(LocalBackend::with_hypervisor("firecracker"));
        provider.install_sdk_registrar(&backend, "local")?;
        let provider = Arc::new(provider);
        mvm_hostd::host_lifetime_parent::install(provider.clone())?;
        Ok(Self {
            supervisor: Some(supervisor),
            provider: Some(provider),
            backend: Some(backend),
            slots: std::array::from_fn(|_| Slot::default()),
        })
    }

    pub fn backend(&self) -> &Arc<LocalBackend> {
        self.backend.as_ref().expect("original backend retained")
    }

    /// Definitions are created by the existing paired LaunchRequest path first.
    /// No ordinary persistent-start fallback is permitted after this invocation.
    pub async fn start(
        &mut self,
        name: &str,
        quote: PortableStartupQuote,
        rootfs: &Path,
        grant: mvm_contract::protocol::mailbox::MailboxGrant,
        journal: &mut InstalledJournal<'_>,
    ) -> Result<()> {
        let i = index(name)?;
        ensure!(
            self.slots[i].parent.is_none(),
            "original collector start already attempted"
        );
        let provider = self
            .provider
            .as_ref()
            .context("original provider missing")?;
        // prepare_request intentionally serves transient launches; persistent
        // journal startup instead supplies its exact VM binding directly.
        let prepared = provider.prepare_vm(
            &VmStartConfig {
                name: name.into(),
                tenant_id: Some("local".into()),
                cpus: 1,
                memory_mib: 256,
                ..Default::default()
            },
            "local",
            BackendKind::Firecracker,
        )?;
        let (prepared, kernel) = prepared.prepare_canonical_startup(quote)?;
        require_signed_root(rootfs, kernel.workload_root_declaration()?)?;
        let (mut lifetime, parent) = prepared.begin_launch()?;
        self.slots[i].parent = Some(parent);
        lifetime.select_canonical_kernel(kernel)?;
        let result = self
            .backend()
            .start_persistent_with_journal_and_host_lifetime(name, grant, journal, lifetime)
            .await;
        match result {
            Ok(started) => {
                self.slots[i].started = Some(started);
                Ok(())
            }
            Err(failed) => {
                let error = anyhow::anyhow!("original owned journal startup: {:#}", failed.error());
                self.slots[i].failed = Some(failed);
                Err(error)
            }
        }
    }

    /// Borrow the actual SDK admission for the unchanged enrollment/posture checks.
    pub fn machine(&self, name: &str) -> Result<&mvm_client::launch::machine_start::MachineStart> {
        Ok(self.slots[index(name)?]
            .started
            .as_ref()
            .context("original owned journal did not start")?
            .machine())
    }

    /// One bounded attempt; an error leaves the same native/journal/parent custody
    /// available for explicit retry. The caller retains the original helper too.
    pub fn stop(
        &mut self,
        name: &str,
        journal: &mut JournalOwner<'_>,
        until: Instant,
    ) -> Result<JournalStopOutcome> {
        let slot = &mut self.slots[index(name)?];
        ensure!(!slot.stopped, "original journal already collected");
        let Slot {
            parent,
            started,
            failed,
            endpoint,
            endpoint_transferred,
            stopped,
        } = slot;
        let parent = parent.as_mut().context("original parent owner missing")?;
        let finish = |_: &mvm_core::plan::ExecutionPlan,
                      _: bool,
                      receipt: &mut mvm_client::OwnedRetirement|
         -> Result<()> {
            if *endpoint_transferred {
                parent.retry_endpoint_cleanup()?;
            } else {
                if endpoint.is_none() {
                    *endpoint = receipt.take_endpoint_retirement();
                }
                // Keep any returned receipt even if storage publication fails.
                let had_receipt = endpoint.is_some();
                let result = parent.retire_endpoint(endpoint);
                *endpoint_transferred = had_receipt && endpoint.is_none();
                result?;
            }
            parent.finish_pending_retired(receipt)
        };
        let outcome = if let Some(started) = started {
            started.stop_and_collect(journal, until, finish)?
        } else if let Some(failed) = failed {
            failed.stop_and_collect(journal, until, finish)?
        } else {
            anyhow::bail!("original SDK startup custody missing")
        };
        *stopped = true;
        Ok(outcome)
    }

    pub fn resources(&self) -> Result<Value> {
        Ok(serde_json::to_value(
            self.supervisor
                .as_ref()
                .context("original supervisor missing")?
                .managed_resources()
                .snapshot()?,
        )?)
    }

    /// Stop only the original tenant service after both journals were collected.
    /// Retained telemetry/startup debt remains booked; no definition or state reaping.
    pub fn close(&mut self) -> Result<Value> {
        ensure!(
            self.slots.iter().all(|slot| slot.stopped),
            "original pair retirement incomplete"
        );
        for slot in &mut self.slots {
            drop(slot.started.take());
            drop(slot.failed.take());
            drop(slot.parent.take());
        }
        drop(self.backend.take());
        let provider = self.provider.take().context("original provider missing")?;
        provider.close_admission();
        // Global parent registration is weak. try_unwrap permits that original
        // weak identity; get_mut would incorrectly reject it.
        let mut provider = match Arc::try_unwrap(provider) {
            Ok(provider) => provider,
            Err(provider) => {
                self.provider = Some(provider);
                anyhow::bail!("original parent provider still shared");
            }
        };
        let result = provider.stop_tenant_helper();
        self.provider = Some(Arc::new(provider));
        result?;
        self.supervisor
            .as_mut()
            .context("original supervisor missing")?
            .close_admission();
        self.resources()
    }
}

impl Drop for CollectorPair {
    fn drop(&mut self) {
        if self
            .slots
            .iter()
            .any(|slot| slot.parent.is_some() && !slot.stopped)
        {
            // Abandonment is never positive retirement or a budget refund.
            std::mem::forget(std::mem::replace(
                &mut self.slots,
                std::array::from_fn(|_| Slot::default()),
            ));
            std::mem::forget((
                self.backend.take(),
                self.provider.take(),
                self.supervisor.take(),
            ));
        }
    }
}

impl workload::CallBoundary for CollectorPair {
    fn start(
        &mut self,
        name: &str,
        payload: &Value,
        until: Instant,
    ) -> Result<Box<dyn workload::PendingCall>> {
        let input = serde_json::to_vec(payload)?;
        ensure!(
            input.len() <= fixture::CALL_INPUT_LIMIT,
            "oversized fixture Call"
        );
        let owner = self.slots[index(name)?]
            .started
            .as_mut()
            .context("original owned journal call custody missing")?;
        let mut output = mvm_client::entrypoint::dispatch::CapturedOutput::default();
        let outcome = owner.run_entrypoint(&input, until, &mut output)?;
        ensure!(
            outcome.exit_code() == 0
                && !output.truncated
                && output.stdout.len() <= workload::CALL_OUTPUT_LIMIT,
            "original owned workload failed: {:?}",
            outcome.terminal
        );
        let value = serde_json::from_slice(&output.stdout)?;
        Ok(Box::new(workload::CompletedCall {
            value: Some(value),
            input_bytes: input.len(),
            output_bytes: output.stdout.len(),
        }))
    }
}

fn require_signed_root(rootfs: &Path, root: &mvm_core::plan::bundle::BundleArtifact) -> Result<()> {
    ensure!(
        root.role == mvm_core::plan::ArtifactRole::Rootfs
            && fs::metadata(rootfs)?.len() == root.size_bytes
            && mvm_core::crypto::image_verify::sha256_file(rootfs)? == root.sha256,
        "original role root differs from signed canonical bundle"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_root_requires_signed_length_and_exact_digest() {
        let home = tempfile::tempdir().unwrap();
        let image = home.path().join("root.ext4");
        fs::write(&image, b"role-a").unwrap();
        let mut root = mvm_core::plan::bundle::BundleArtifact {
            name: "rootfs".into(),
            role: mvm_core::plan::ArtifactRole::Rootfs,
            path: "rootfs.ext4".into(),
            sha256: mvm_core::crypto::image_verify::sha256_file(&image).unwrap(),
            size_bytes: 6,
        };
        require_signed_root(&image, &root).unwrap();
        fs::write(&image, b"role-b").unwrap();
        assert!(require_signed_root(&image, &root).is_err());
        fs::write(&image, b"role-a\0").unwrap();
        assert!(require_signed_root(&image, &root).is_err());
        fs::write(&image, b"role-a").unwrap();
        root.size_bytes += 1;
        assert!(require_signed_root(&image, &root).is_err());
    }
}
