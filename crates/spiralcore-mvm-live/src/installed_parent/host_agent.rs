//! Retain the existing signed VM registration under the original installed holder.
use super::*;
use mvm_core::host_lifetime::HostLifetimeAnchor;
use mvm_vmm::host::host_agent_spawn::{
    HostAgentServicesParams, OwnedHostAgentRegistrar, OwnedVmRegistration, RegisteredEndpointOwner,
};
use std::{
    fs::File,
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// One immutable original registrar can serve two reservations on the installed SDK runner.
/// The callback selects a grant only inside its original tenant controller;
/// each selected grant independently verifies the signed registration and original directories.
pub(super) struct TenantRegistrationOwner(Weak<Shared>);
impl TenantRegistrationOwner {
    pub(super) fn new(shared: &Arc<Shared>) -> Self {
        Self(Arc::downgrade(shared))
    }
    pub(super) fn original_grant(&self, tenant: &str, vm: &str) -> Result<Arc<VmGrant>> {
        let parent = self
            .0
            .upgrade()
            .ok_or_else(|| anyhow!("original installed parent missing"))?;
        let registry = parent
            .registry
            .lock()
            .map_err(|_| anyhow!("original VM registry poisoned"))?;
        registry
            .slots
            .iter()
            .filter_map(Weak::upgrade)
            .find(|g| g.tenant == tenant && g.binding.vm_name == vm)
            .ok_or_else(|| anyhow!("original tenant VM reservation missing"))
    }
}
impl RegisteredEndpointOwner for TenantRegistrationOwner {
    fn bind_registration(
        &self,
        registrar: &OwnedHostAgentRegistrar,
        registration: &OwnedVmRegistration,
        params: &HostAgentServicesParams<'_>,
    ) -> Result<()> {
        let grant = self.original_grant(
            params
                .tenant_id
                .ok_or_else(|| anyhow!("original tenant missing"))?,
            params.vm_name,
        )?;
        grant.bind_registration(registrar, registration, params)
    }
    fn retired_audit(
        &self,
        registration: &OwnedVmRegistration,
        file: &mvm_vmm::host::host_agent_spawn::RegisteredAuditFile,
    ) -> Result<()> {
        let opened = file.registered();
        self.original_grant(&opened.tenant, &opened.vm)?
            .retired_audit(registration, file)
    }
}
impl InstalledParentProvider {
    /// Install the original funded tenant registrar once on this SDK runner.
    /// The resident also retains and registers this provider with MVM host admission.
    pub fn install_sdk_registrar(
        &self,
        backend: &mvm_client::LocalBackend,
        tenant: &str,
    ) -> Result<()> {
        let (registrar, permit) = self.shared.tenant_registrar(tenant)?;
        permit.check_authority(&registrar)?;
        let registrar = registrar
            .with_registration_owner(Arc::new(TenantRegistrationOwner::new(&self.shared)))?;
        backend.install_owned_host_agent(registrar)?;
        Ok(())
    }
}

impl ParentLaunchOwner {
    /// Register this launch through its original tenant daemon. Only the signed
    /// registration callback can pin state and socket directories for telemetry.
    pub fn original_registrar(&self) -> Result<OwnedHostAgentRegistrar> {
        let grant = &self.grant;
        grant.check_open()?;
        {
            let lifecycle = grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle.check_launch_open()?;
            ensure!(lifecycle.armed, "original VM launch is not armed");
        }
        let parent = grant
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original installed parent missing"))?;
        let (original, permit) = parent.tenant_registrar(&grant.tenant)?;
        permit.check_authority(&original)?;
        original.with_registration_owner(grant.clone())
    }
}

impl RegisteredEndpointOwner for VmGrant {
    fn bind_registration(
        &self,
        registrar: &OwnedHostAgentRegistrar,
        registration: &OwnedVmRegistration,
        params: &HostAgentServicesParams<'_>,
    ) -> Result<()> {
        let grant = self
            .original
            .upgrade()
            .ok_or_else(|| anyhow!("original VM grant missing"))?;
        grant.check_current(&grant.binding)?;
        let parent = grant
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original installed parent missing"))?;
        let (original, funded) = parent.tenant_registrar(&grant.tenant)?;
        ensure!(
            registrar.same_original(&original),
            "original registrar differs"
        );
        funded.check_authority(registrar)?;
        let facts = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .registration
            .clone()
            .ok_or_else(|| anyhow!("original signed registration facts missing"))?;
        let anchor = facts
            .anchor
            .upgrade()
            .ok_or_else(|| anyhow!("original launch anchor missing"))?;
        let permit: Arc<dyn CoreCurrentParentPermit> = grant.clone();
        ensure!(
            anchor.retains_parent_permit(&permit),
            "original registration launch differs"
        );
        anchor.verify_current()?;
        facts.check_request(params)?;
        let (state, sockets) = registration.original_directories(registrar, params)?;
        registration.check_current_for(registrar, params)?;
        grant.check_current(&grant.binding)?;
        facts.pin_original(state, sockets)?;
        parent.enroll_audit(
            self,
            registration
                .registered_audit()
                .ok_or_else(|| anyhow!("original signer inventory missing"))?,
        )
    }

    fn retired_audit(
        &self,
        registration: &OwnedVmRegistration,
        file: &mvm_vmm::host::host_agent_spawn::RegisteredAuditFile,
    ) -> Result<()> {
        ensure!(
            registration
                .registered_audit()
                .is_some_and(|held| std::ptr::eq(file, held)),
            "foreign original registered archive"
        );
        self.parent
            .upgrade()
            .ok_or_else(|| anyhow!("original installed parent missing"))?
            .adopt_audit(self, file)
    }
}
pub(super) struct RegistrationBinding {
    anchor: Weak<HostLifetimeAnchor>,
    commitment: [u8; 32],
    // Retain actual copied paths and prepaid facts storage through VM cleanup.
    state: PathBuf,
    broker: PathBuf,
    directories: Mutex<Option<(File, File)>>,
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
    pub(super) fn commitment(&self) -> [u8; 32] {
        self.commitment
    }
    pub(super) fn check_request(&self, params: &HostAgentServicesParams<'_>) -> Result<()> {
        ensure!(
            params.state_dir == self.state
                && params.broker_listen_socket == self.broker
                && commitment(params)? == self.commitment,
            "signed registration facts differ"
        );
        self.anchor
            .upgrade()
            .ok_or_else(|| anyhow!("original launch anchor missing"))?
            .verify_current()
    }

    fn pin_original(&self, state: &File, sockets: &File) -> Result<()> {
        let parent = self
            .broker
            .parent()
            .ok_or_else(|| anyhow!("original broker parent absent"))?;
        match_named_directory(&self.state, state)?;
        match_named_directory(parent, sockets)?;
        let mut held = self
            .directories
            .lock()
            .map_err(|_| anyhow!("original registration pin poisoned"))?;
        match held.as_ref() {
            Some((before_state, before_sockets)) => {
                ensure!(
                    same_directory(before_state, state)?
                        && same_directory(before_sockets, sockets)?,
                    "original registration incarnation differs"
                );
            }
            None => *held = Some((state.try_clone()?, sockets.try_clone()?)),
        }
        Ok(())
    }
    pub(super) fn check_retained_state(&self, path: &Path, directory: &File) -> Result<()> {
        ensure!(path == self.state, "retained VM state path differs");
        let held = self
            .directories
            .lock()
            .map_err(|_| anyhow!("original registration pin poisoned"))?;
        let (state, sockets) = held
            .as_ref()
            .ok_or_else(|| anyhow!("original registration directories absent"))?;
        match_named_directory(&self.state, state)?;
        match_named_directory(
            self.broker
                .parent()
                .ok_or_else(|| anyhow!("original broker parent absent"))?,
            sockets,
        )?;
        ensure!(
            same_directory(directory, state)?,
            "retained state directory differs from original registration"
        );
        self.anchor
            .upgrade()
            .ok_or_else(|| anyhow!("original anchor unavailable"))?
            .verify_current()
    }
}
pub(super) fn same_directory(left: &File, right: &File) -> Result<bool> {
    let a = left.metadata()?;
    let b = right.metadata()?;
    Ok(a.is_dir() && b.is_dir() && (a.dev(), a.ino()) == (b.dev(), b.ino()))
}
fn match_named_directory(path: &Path, held: &File) -> Result<()> {
    let named = std::fs::symlink_metadata(path)?;
    let meta = held.metadata()?;
    mvm_core::private_fs::require_current_owner(&meta)?;
    ensure!(
        named.is_dir()
            && meta.is_dir()
            && meta.mode() & 0o7777 == 0o700
            && (meta.dev(), meta.ino()) == (named.dev(), named.ino()),
        "registered directory path or permissions differ"
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
            directories: Mutex::new(None),
            _storage: storage,
        }))
    }
}
