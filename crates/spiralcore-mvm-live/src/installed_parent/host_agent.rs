//! Actual original helper custody. No shipping profile is activated here.
use super::*;
use crate::installed_resources::ManagedValue;
use mvm_core::host_lifetime::{HostLifetimeAnchor, PreparedHostLifetimeLease};
use mvm_core::user_config::{load_strict_bounded, owned_config_workspace_requirement};
use mvm_vmm::host::host_agent_spawn::{
    HostAgentParentPermit, HostAgentServicesParams, OwnedHostAgent, OwnedHostAgentInputs,
    OwnedHostAgentRegistrar, OwnedHostAgentSpawnError,
};
use mvm_vmm::host::network_endpoint_spawn::{EgressProxySpawnConfig, InstalledEndpointSettings};
use std::{
    fmt,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Unused,
    Preparing,
    Active,
    Uncertain,
    Retired,
}
pub(super) struct QualifiedHostAgentProfile {
    claim: ManagedClaim,
    cold_operator: Option<ColdOperatorInput>,
}

struct ColdOperatorInput {
    config_dir: PathBuf,
    telemetry_collect: Option<String>,
}
impl QualifiedHostAgentProfile {
    #[cfg(test)]
    pub(super) fn fixture(claim: ManagedClaim) -> Self {
        Self {
            claim,
            cold_operator: None,
        }
    }

    #[cfg(test)]
    pub(super) fn fixture_with_cold_operator(
        claim: ManagedClaim,
        config_dir: PathBuf,
        telemetry_collect: Option<String>,
    ) -> Self {
        Self {
            claim,
            cold_operator: Some(ColdOperatorInput {
                config_dir,
                telemetry_collect,
            }),
        }
    }
}
// No production constructor until the complete selected helper model is accepted.
#[derive(Default)]
pub(super) struct HostAgentSlot {
    profile: Option<QualifiedHostAgentProfile>,
    tenant: Option<String>,
    phase: Phase,
}
struct Permit {
    parent: Weak<Shared>,
    tenant: String,
    reservation: ManagedReservation,
    settings: Option<ManagedValue<InstalledEndpointSettings>>,
}

/// Linear resident custody, separate from the clonable original registrar.
pub(super) struct RootHostAgent {
    permit: Arc<Permit>,
    agent: Option<Agent>,
    stopped: bool,
}
enum Agent {
    Ready(OwnedHostAgent),
    Failed(anyhow::Error),
}
struct StartFailure {
    owner: RootHostAgent,
}
impl fmt::Debug for StartFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StartFailure { original_owned_helper_retained: true }")
    }
}
impl fmt::Display for StartFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("original host-agent startup failed; cleanup custody retained")
    }
}
impl std::error::Error for StartFailure {}

#[cfg(test)]
pub(super) fn synthetic_start_failure_reason(error: &anyhow::Error) -> String {
    // Only the isolated synthetic fixture uses this diagnostic. Product error
    // formatting stays static and never emits private configuration or keys.
    use std::fmt::Write as _;
    struct Bounded(String);
    impl fmt::Write for Bounded {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            if text.len() > 2048_usize.saturating_sub(self.0.len()) {
                return Err(fmt::Error);
            }
            self.0.push_str(text);
            Ok(())
        }
    }
    let reason = error.downcast_ref::<StartFailure>().and_then(|failure| {
        match failure.owner.agent.as_ref() {
            Some(Agent::Failed(reason)) => Some(reason),
            _ => None,
        }
    });
    let Some(reason) = reason else {
        return "non-startup helper refusal".into();
    };
    let mut text = Bounded(String::with_capacity(2048));
    for (index, cause) in reason.chain().take(8).enumerate() {
        if write!(&mut text, "[{index}] {cause}; ").is_err() {
            return "owned helper detector exceeded diagnostic bound".into();
        }
    }
    text.0
}

impl InstalledParentProvider {
    pub(super) fn capture_cold_operator_settings(
        &self,
        tenant: &str,
        config_dir: &Path,
        telemetry_collect: Option<&str>,
    ) -> Result<ManagedValue<InstalledEndpointSettings>> {
        const CONFIG_BYTES: usize = 8 * 1024;
        ensure!(
            telemetry_collect.is_none_or(|value| value.len() <= 64),
            "installed collection setting exceeds input limit"
        );
        ensure!(
            telemetry_collect.is_some_and(|value| {
                ["1", "true", "on", "yes"]
                    .iter()
                    .any(|enabled| value.trim().eq_ignore_ascii_case(enabled))
            }),
            "installed endpoint requires explicit telemetry collection"
        );
        mvm_core::naming::validate_vm_name(tenant)?;
        let tenant = tenant_tag(tenant);
        let workspace_bytes = owned_config_workspace_requirement(CONFIG_BYTES, CONFIG_BYTES)?;
        let workspace = self.shared.pool.reserve(
            tenant,
            ManagedClaim {
                host_metadata_bytes: workspace_bytes,
                ..Default::default()
            },
        )?;
        workspace.check_current()?;
        {
            let config = load_strict_bounded(Some(config_dir), CONFIG_BYTES, workspace_bytes)?;
            let proxy = config.egress_proxy.as_ref().ok_or_else(|| {
                anyhow!("installed endpoint requires explicit egress proxy table")
            })?;
            ensure!(
                proxy.https.is_none() && proxy.http.is_none() && proxy.no_proxy.is_none(),
                "installed ObjectTCP proxy service unsupported"
            );
        }
        drop(workspace);
        let retained = self.shared.pool.reserve(
            tenant,
            ManagedClaim {
                host_metadata_bytes: u64::try_from(std::mem::size_of::<
                    ManagedValue<InstalledEndpointSettings>,
                >())?,
                ..Default::default()
            },
        )?;
        Ok(
            retained.attach(InstalledEndpointSettings::ResolvedConfigured(
                EgressProxySpawnConfig::default(),
            )),
        )
    }

    // This observation only recognizes the private no-attempt phase after a
    // refusal. Poison or any attempted handoff is conservatively uncertain.
    pub(super) fn host_agent_unused(&self) -> bool {
        self.shared
            .host_agent
            .lock()
            .is_ok_and(|slot| slot.phase == Phase::Unused)
    }

    #[cfg(test)]
    pub(super) fn configure_host_agent(
        &self,
        supervisor: &InstalledSupervisor,
        tenant: &str,
        profile: QualifiedHostAgentProfile,
    ) -> Result<()> {
        mvm_core::naming::validate_vm_name(tenant)?;
        ensure!(
            self.shared.pool.same_pool(supervisor.managed_resources()),
            "helper supervisor differs from original parent"
        );
        {
            let registry = self
                .shared
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?;
            ensure!(
                !registry.closed && registry.slots.iter().all(|slot| slot.upgrade().is_none()),
                "parent already issued or closed"
            );
        }
        let mut slot = self
            .shared
            .host_agent
            .lock()
            .map_err(|_| anyhow!("helper state poisoned"))?;
        ensure!(
            slot.profile.is_none() && slot.phase == Phase::Unused,
            "original helper already configured"
        );
        slot.tenant = Some(tenant.into());
        slot.profile = Some(profile);
        Ok(())
    }

    pub(super) fn prepare_owned_host_agent(&self, tenant: &str) -> Result<RootHostAgent> {
        mvm_core::naming::validate_vm_name(tenant)?;
        let (claim, cold_operator) = {
            let mut slot = self
                .shared
                .host_agent
                .lock()
                .map_err(|_| anyhow!("helper state poisoned"))?;
            let profile = slot
                .profile
                .as_ref()
                .ok_or_else(|| anyhow!("owned helper resource model not qualified"))?;
            ensure!(
                slot.tenant.as_deref() == Some(tenant),
                "original helper tenant differs"
            );
            ensure!(
                slot.phase == Phase::Unused,
                "original helper already issued"
            );
            let claim = profile.claim;
            ensure!(
                claim.workers == 2 && claim.guests == 0 && claim.host_metadata_bytes >= 65536,
                "owned helper model incomplete"
            );
            let cold_operator = slot.profile.as_mut().unwrap().cold_operator.take();
            slot.phase = Phase::Preparing;
            (claim, cold_operator)
        };
        // Reserve before copying identity or handing off any original holder.
        // All later failures fence this one attempt; no reset/reacquire path.
        let result = (|| -> Result<RootHostAgent> {
            let reservation = self.shared.pool.reserve(tenant_tag(tenant), claim)?;
            let settings = cold_operator
                .as_ref()
                .map(|input| {
                    self.capture_cold_operator_settings(
                        tenant,
                        &input.config_dir,
                        input.telemetry_collect.as_deref(),
                    )
                })
                .transpose()?;
            let permit = Arc::new(Permit {
                parent: Arc::downgrade(&self.shared),
                tenant: tenant.into(),
                reservation,
                settings,
            });
            permit.check_current(tenant)?;
            let (_, handoff) = self.shared.issuer.prepare()?;
            let holder = PreparedHostLifetimeLease::bind(
                handoff.directory,
                handoff.holder,
                HostLeaseExpectation {
                    state_name: "state.bin".into(),
                    state_bytes: handoff.state_bytes,
                    state_sha256: handoff.state_sha256,
                    absent_names: [".state.next".into(), ".state.intent".into()],
                },
            )?;
            let outcome = OwnedHostAgent::spawn(OwnedHostAgentInputs {
                tenant: tenant.into(),
                holder,
                parent: permit.clone(),
            });
            let owner = RootHostAgent {
                permit,
                agent: Some(match outcome {
                    Ok(agent) => Agent::Ready(agent),
                    Err(reason) => Agent::Failed(reason),
                }),
                stopped: false,
            };
            if matches!(owner.agent, Some(Agent::Failed(_))) {
                return Err(StartFailure { owner }.into());
            }
            let activated = (|| -> Result<()> {
                let mut slot = self
                    .shared
                    .host_agent
                    .lock()
                    .map_err(|_| anyhow!("helper state poisoned"))?;
                ensure!(
                    slot.phase == Phase::Preparing,
                    "original helper admission changed"
                );
                slot.phase = Phase::Active;
                Ok(())
            })();
            if activated.is_err() {
                // Startup already produced an actual owner. A late state
                // failure must return that owner, not erase it through Drop.
                return Err(StartFailure { owner }.into());
            }
            Ok(owner)
        })();
        if result.is_err() {
            if let Ok(mut slot) = self.shared.host_agent.lock() {
                slot.phase = Phase::Uncertain;
            }
            if let Ok(mut registry) = self.shared.registry.lock() {
                registry.closed = true;
            }
        }
        result
    }
}
impl RootHostAgent {
    pub(super) fn from_start_failure(error: anyhow::Error) -> Result<Self> {
        error
            .downcast::<StartFailure>()
            .map(|failure| failure.owner)
    }

    pub(super) fn registrar(&self) -> Result<OwnedHostAgentRegistrar> {
        self.permit.check_current(&self.permit.tenant)?;
        match self.agent.as_ref() {
            Some(Agent::Ready(agent)) if !self.stopped => Ok(agent.registrar()),
            _ => Err(anyhow!("original helper registrar unavailable")),
        }
    }
    pub(super) fn shutdown(&mut self) -> Result<()> {
        if self.stopped {
            return Ok(());
        }
        self.permit.reservation.check_retained_identity()?;
        match self.agent.as_mut() {
            Some(Agent::Ready(agent)) => agent.shutdown()?,
            Some(Agent::Failed(error)) => error
                .downcast_mut::<OwnedHostAgentSpawnError>()
                .ok_or_else(|| anyhow!("original helper cleanup proof unavailable"))?
                .agent_mut()
                .shutdown()?,
            None => return Err(anyhow!("original helper custody unavailable")),
        }
        let parent = self
            .permit
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        parent
            .host_agent
            .lock()
            .map_err(|_| anyhow!("helper state poisoned"))?
            .phase = Phase::Retired;
        self.stopped = true;
        Ok(())
    }
}
impl Drop for RootHostAgent {
    fn drop(&mut self) {
        if !self.stopped {
            if let Some(parent) = self.permit.parent.upgrade() {
                if let Ok(mut state) = parent.host_agent.lock() {
                    state.phase = Phase::Uncertain;
                }
                if let Ok(mut registry) = parent.registry.lock() {
                    registry.closed = true;
                }
            }
            if let Some(agent) = self.agent.take() {
                std::mem::forget(agent);
            }
            std::mem::forget(self.permit.clone());
        }
    }
}
impl HostAgentParentPermit for Permit {
    fn installed_endpoint_settings(
        &self,
        params: &HostAgentServicesParams<'_>,
    ) -> Result<&InstalledEndpointSettings> {
        self.check_registration(params)?;
        self.settings
            .as_ref()
            .map(AsRef::as_ref)
            .ok_or_else(|| anyhow!("original installed endpoint settings unavailable"))
    }

    fn check_current(&self, tenant: &str) -> Result<()> {
        ensure!(tenant == self.tenant, "original helper tenant differs");
        self.reservation.check_current()?;
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        ensure!(
            !parent
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?
                .closed,
            "original parent admission closed"
        );
        let state = parent
            .host_agent
            .lock()
            .map_err(|_| anyhow!("helper state poisoned"))?;
        ensure!(
            state.tenant.as_deref() == Some(tenant)
                && matches!(state.phase, Phase::Preparing | Phase::Active),
            "original helper admission unavailable"
        );
        Ok(())
    }
    fn check_registration(&self, params: &HostAgentServicesParams<'_>) -> Result<()> {
        self.check_current(
            params
                .tenant_id
                .ok_or_else(|| anyhow!("canonical helper tenant missing"))?,
        )?;
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let grant = {
            let registry = parent
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?;
            registry
                .slots
                .iter()
                .filter_map(Weak::upgrade)
                .find(|grant| grant.binding.vm_name == params.vm_name)
                .ok_or_else(|| anyhow!("original VM grant unavailable"))?
        };
        grant.check_current(&grant.binding)?;
        let facts = {
            let lifecycle = grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle
                .registration
                .as_ref()
                .ok_or_else(|| anyhow!("original registration admission missing"))?
                .clone()
        };
        let anchor = facts
            .anchor
            .upgrade()
            .ok_or_else(|| anyhow!("original launch anchor unavailable"))?;
        let original: Arc<dyn CoreCurrentParentPermit> = grant.clone();
        ensure!(
            anchor.retains_parent_permit(&original),
            "original launch anchor differs"
        );
        anchor.verify_current()?;
        mvm_vmm::host::host_agent_spawn::validate_owned_host_agent_services(params)?;
        ensure!(
            facts.commitment == commitment(params)?,
            "canonical registration facts differ"
        );
        facts.check_paths(params)?;
        grant.check_current(&grant.binding)?;
        self.check_current(&self.tenant)
    }
}

pub(super) struct RegistrationBinding {
    anchor: Weak<HostLifetimeAnchor>,
    commitment: [u8; 32],
    // Retain actual copied paths and prepaid facts storage through VM cleanup.
    state: PathBuf,
    broker: PathBuf,
    directory: Mutex<Option<Arc<File>>>,
    socket_directory: Mutex<Option<Arc<File>>>,
    _storage: ManagedReservation,
}
struct HashWriter {
    hash: Sha256,
    bytes: usize,
}
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > 16384usize.saturating_sub(self.bytes) {
            return Err(std::io::Error::other("owned registration facts limit"));
        }
        self.hash.update(bytes);
        self.bytes += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn commitment(params: &HostAgentServicesParams<'_>) -> Result<[u8; 32]> {
    ensure!(
        params.workload_id.len() <= 63
            && params.vm_name.len() <= 63
            && params.tenant_id.is_some_and(|tenant| tenant.len() <= 63)
            && params.state_dir.as_os_str().len() <= 4096
            && params.broker_listen_socket.as_os_str().len() <= 4096,
        "owned registration identity/path profile exceeded"
    );
    let mut writer = HashWriter {
        hash: Sha256::new(),
        bytes: 0,
    };
    writer
        .hash
        .update(b"SpiralCore/installed-parent/registration/v1\0");
    serde_json::to_writer(
        &mut writer,
        &(
            params.workload_id,
            params.tenant_id,
            params.vm_name,
            params.state_dir,
            params.broker_listen_socket,
            params.services,
            params.capability_bindings,
            params.service_proxies,
        ),
    )?;
    Ok(writer.hash.finalize().into())
}

impl RegistrationBinding {
    pub(super) fn check_retained_state(
        &self,
        path: &std::path::Path,
        directory: &File,
    ) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        ensure!(path == self.state, "original retained state path differs");
        let original = self
            .directory
            .lock()
            .map_err(|_| anyhow!("registered directory state poisoned"))?
            .clone()
            .ok_or_else(|| anyhow!("original registered directory not pinned"))?;
        // This callback must never first-pin a replacement directory.
        check_directory(&self.state, &self.directory)?;
        let expected = original.metadata()?;
        let actual = directory.metadata()?;
        mvm_core::private_fs::require_current_owner(&actual)?;
        ensure!(
            actual.is_dir()
                && actual.mode() & 0o7777 == 0o700
                && (expected.dev(), expected.ino()) == (actual.dev(), actual.ino()),
            "original retained state directory differs"
        );
        self.anchor
            .upgrade()
            .ok_or_else(|| anyhow!("original launch anchor unavailable"))?
            .verify_current()
    }
    fn check_paths(&self, params: &HostAgentServicesParams<'_>) -> Result<()> {
        ensure!(
            params.state_dir == self.state && params.broker_listen_socket == self.broker,
            "original registered paths differ"
        );
        check_directory(&self.state, &self.directory)?;
        // Canonical socket shortening can place this directory outside state.
        // Its actual effect destination needs its OWN original descriptor pin.
        check_directory(
            self.broker
                .parent()
                .ok_or_else(|| anyhow!("original broker parent missing"))?,
            &self.socket_directory,
        )
    }
}

fn check_directory(path: &std::path::Path, pin: &Mutex<Option<Arc<File>>>) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let existing = pin
        .lock()
        .map_err(|_| anyhow!("registered directory state poisoned"))?
        .clone();
    let directory = match existing {
        Some(directory) => directory,
        None => {
            let directory = Arc::new(
                File::options()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_DIRECTORY)
                    .open(path)?,
            );
            let meta = directory.metadata()?;
            mvm_core::private_fs::require_current_owner(&meta)?;
            ensure!(
                meta.is_dir() && meta.mode() & 0o7777 == 0o700,
                "registered state directory is not private"
            );
            let original = {
                let mut slot = pin
                    .lock()
                    .map_err(|_| anyhow!("registered directory state poisoned"))?;
                match slot.as_ref() {
                    Some(original) => original.clone(),
                    None => {
                        *slot = Some(directory.clone());
                        directory.clone()
                    }
                }
            };
            // A concurrent successful pin may win. Close this candidate
            // outside the mutex, preserving the original winner.
            drop(directory);
            original
        }
    };
    let original = directory.metadata()?;
    let named = std::fs::symlink_metadata(path)?;
    mvm_core::private_fs::require_current_owner(&named)?;
    ensure!(
        named.is_dir()
            && named.mode() & 0o7777 == 0o700
            && (original.dev(), original.ino()) == (named.dev(), named.ino()),
        "original registered directory changed"
    );
    Ok(())
}

impl VmGrant {
    pub(super) fn prepare_original_registration(
        &self,
        admitted: &AdmittedPlan,
        config: &mvm_core::vm_backend::VmStartConfig,
        backend: BackendKind,
    ) -> Result<Arc<RegistrationBinding>> {
        let anchor = config
            .host_lifetime
            .as_ref()
            .ok_or_else(|| anyhow!("original launch anchor missing"))?;
        anchor.verify_config(config, backend)?;
        ensure!(
            *anchor.binding() == self.binding,
            "original helper launch differs"
        );
        let plan = admitted.plan();
        let count: usize = plan
            .extensions
            .iter()
            .map(|ext| ext.capabilities.len())
            .sum();
        ensure!(
            plan.services.len() <= 2 && count <= 16 && config.service_proxies.is_empty(),
            "owned helper service profile unsupported"
        );
        ensure!(
            plan.extensions
                .iter()
                .flat_map(|ext| &ext.capabilities)
                .all(|binding| binding
                    .capability
                    .service
                    .as_str()
                    .len()
                    .checked_add(binding.capability.verb.len())
                    .is_some_and(|bytes| bytes <= 128)),
            "owned capability identity profile exceeded"
        );
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let storage = parent.pool.reserve(
            tenant_tag(&self.tenant),
            ManagedClaim {
                host_metadata_bytes: 32768,
                ..Default::default()
            },
        )?;
        let state = mvm_core::config::vm_state_dir(&self.binding.vm_name);
        let broker = mvm_core::config::vm_vsock_port_socket_at(
            &state,
            mvm_net::channel::GuestService::Broker.port(),
        );
        ensure!(
            state.as_os_str().len() <= 4096 && broker.as_os_str().len() <= 4096,
            "original helper path profile unsupported"
        );
        let capabilities: Vec<_> = plan
            .extensions
            .iter()
            .flat_map(|ext| ext.capabilities.iter().cloned())
            .collect();
        let params = HostAgentServicesParams {
            // Preserve the canonical Runtime BrokerRegisterRequest mapping.
            // The separately signed plan workload remains the authorizer.
            workload_id: &config.name,
            tenant_id: Some(&plan.tenant.0),
            vm_name: &self.binding.vm_name,
            state_dir: &state,
            broker_listen_socket: &broker,
            services: &plan.services,
            capability_bindings: &capabilities,
            service_proxies: &config.service_proxies,
        };
        mvm_vmm::host::host_agent_spawn::validate_owned_host_agent_services(&params)?;
        Ok(Arc::new(RegistrationBinding {
            anchor: Arc::downgrade(anchor),
            commitment: commitment(&params)?,
            state,
            broker,
            directory: Mutex::new(None),
            socket_directory: Mutex::new(None),
            _storage: storage,
        }))
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};

    #[test]
    fn replaced_canonical_short_socket_parent_is_not_original_directory_authority() {
        let (_installation, mut supervisor, provider) = super::super::endpoint_tests::fixture();
        let home = tempfile::tempdir_in("/tmp").unwrap();
        let state = home
            .path()
            .join("long-canonical-state-directory-".repeat(4));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&state)
            .unwrap();
        let broker = mvm_core::config::vm_vsock_port_socket_at(
            &state,
            mvm_net::channel::GuestService::Broker.port(),
        );
        let socket_parent = broker.parent().unwrap().to_path_buf();
        assert_ne!(
            socket_parent, state,
            "fixture did not select canonical short sockets"
        );
        // Create exclusively: never take ownership of another existing namespace.
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&socket_parent)
            .unwrap();
        let facts = RegistrationBinding {
            anchor: Weak::new(),
            commitment: [0; 32],
            state: state.clone(),
            broker: broker.clone(),
            directory: Mutex::new(None),
            socket_directory: Mutex::new(None),
            _storage: provider
                .shared
                .pool
                .reserve(
                    tenant_tag("local"),
                    ManagedClaim {
                        host_metadata_bytes: 32768,
                        ..Default::default()
                    },
                )
                .unwrap(),
        };
        let params = HostAgentServicesParams {
            workload_id: "path-fixture",
            tenant_id: Some("local"),
            vm_name: "path-fixture",
            state_dir: &state,
            broker_listen_socket: &broker,
            services: &[],
            capability_bindings: &[],
            service_proxies: &[],
        };
        facts.check_paths(&params).unwrap();
        std::fs::rename(&socket_parent, home.path().join("original-socket-parent")).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&socket_parent)
            .unwrap();
        let replacement = std::fs::symlink_metadata(&socket_parent).unwrap();
        let primary = facts.check_paths(&params);
        let named = std::fs::symlink_metadata(&socket_parent).unwrap();
        let cleanup = if (named.dev(), named.ino()) == (replacement.dev(), replacement.ino()) {
            std::fs::remove_dir(&socket_parent)
        } else {
            Err(std::io::Error::other(
                "fixture replacement changed before cleanup",
            ))
        };
        assert!(
            cleanup.is_ok(),
            "owned empty socket cleanup failed: {cleanup:?}"
        );
        assert!(
            primary.is_err(),
            "replaced canonical broker parent passed original directory check"
        );
        drop(facts);
        drop(provider);
        supervisor.close_admission();
        supervisor.try_finish().unwrap();
    }
}
