//! Concrete dependency injection into the canonical MVM host. MVM depends on
//! this trait contract, never on this embedding or a second copy of its types.
use super::*;
use mvm_agentd::vsock::{ExecEvent, GuestRequest, GuestResponse};
use mvm_core::vm_backend::VmStartConfig;
use mvm_hostd::owner_access::{
    AuthenticatedOwner, CurrentAccessBoot, FactoryResult, OwnerAccessEndpoint, OwnerAccessFactory,
    OwnerPin,
};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct InstalledAccessFactory {
    pins: Vec<OwnerPin>,
    policy: LocalPolicy,
    profile: AgentProfile,
    budget: SharedBudget,
    clock: Arc<dyn Clock>,
    audit: Arc<AuditEmitter>,
    #[cfg(feature = "object-v1")]
    object_root: Option<object::StoreRoot>,
}
impl InstalledAccessFactory {
    pub fn new(
        pins: Vec<OwnerPin>,
        policy: LocalPolicy,
        profile: AgentProfile,
        budget: SharedBudget,
        clock: Arc<dyn Clock>,
        audit: Arc<AuditEmitter>,
    ) -> Result<Self> {
        if !matches!(profile, AgentProfile::Dev | AgentProfile::SealedProd)
            || pins.is_empty()
            || pins.len() > 32
        {
            return Err(Error::Profile);
        }
        budget.limits()?;
        if pins.iter().any(|pin| !policy.owners.contains(&pin.owner)) {
            return Err(Error::Owner);
        }
        Ok(Self {
            pins,
            policy,
            profile,
            budget,
            clock,
            audit,
            #[cfg(feature = "object-v1")]
            object_root: None,
        })
    }
    /// Local capacity is subtractive admission, never additional permission.
    /// Signed object selection still supplies the operation/owner/schema grant.
    #[cfg(feature = "object-v1")]
    pub fn with_object_store(mut self, root: std::path::PathBuf) -> Result<Self> {
        let root = object::StoreRoot::open(root)?;
        let limits = self.budget.limits()?;
        let codec = spiralcore_transfer::resource_profile::ResourceProfile::SmallV1
            .operation_charge()
            .map_err(|_| Error::Unsupported)?;
        if limits.frames_per_request < 32
            || (limits.codec_bytes as u64) < codec
            || limits.wire_bytes < object::WIRE_RESERVATION
        {
            return Err(Error::Limit);
        }
        self.object_root = Some(root);
        Ok(self)
    }
}
struct InstalledEndpoint {
    access: Mutex<NativeAccess>,
    control: AccessControl,
    key: ed25519_dalek::SigningKey,
    next: AtomicU64,
}
impl OwnerAccessFactory for InstalledAccessFactory {
    fn owner_pins(&self) -> &[OwnerPin] {
        &self.pins
    }
    fn preflight(&self, admitted: &AdmittedPlan, config: &VmStartConfig) -> FactoryResult<()> {
        mvm_core::naming::validate_vm_name(&config.name)?;
        let plan = admitted.plan();
        if !self.pins.iter().any(|pin| pin.tenant == plan.tenant.0) {
            return Err(Error::Owner.into());
        }
        if plan
            .grants
            .as_ref()
            .is_some_and(|grants| grants.owner_access.is_some())
        {
            let binding = mvm_contract::grants::owner_access::OwnerAccessBinding::from_plan(
                plan,
                &config.name,
            )
            .map_err(|_| Error::Policy)?
            .ok_or(Error::Policy)?;
            if let Some(profile) = binding.grant.object_v1 {
                if profile.cache_reuse_within_execution {
                    return Err(Error::Unsupported.into());
                }
                #[cfg(feature = "object-v1")]
                self.object_root
                    .as_ref()
                    .ok_or(Error::Unsupported)?
                    .check()?;
                #[cfg(not(feature = "object-v1"))]
                return Err(Error::Unsupported.into());
            }
        } else if self.profile != AgentProfile::Dev
            || !plan
                .agent_verbs
                .as_ref()
                .is_some_and(|verbs| verbs.iter().any(|verb| verb.as_str() == "exec"))
        {
            return Err(Error::Profile.into());
        }
        Ok(())
    }
    fn activate(
        &self,
        admitted: &AdmittedPlan,
        config: &VmStartConfig,
        boot: CurrentAccessBoot,
    ) -> FactoryResult<Arc<dyn OwnerAccessEndpoint>> {
        self.preflight(admitted, config)?;
        boot.check()?;
        let signer = mvm_hostd::audit::host_keypair::load_or_init()?;
        if &signer.verifying != boot.host() {
            return Err(Error::Signature.into());
        }
        let fresh = boot.clone();
        let control = AccessControl::with_current_check(
            LifecycleBinding {
                instance: config.name.clone(),
                tenant: admitted.plan().tenant.0.clone(),
                image_sha256: admitted.plan().image.sha256.clone(),
                revocation_generation: 1,
                profile: self.profile,
            },
            Arc::new(move || fresh.check().map_err(|_| Error::Revoked)),
        );
        let access = NativeAccess::from_admitted(
            admitted,
            Admission {
                signed: admitted.signed(),
                keys: &[],
                grant: boot.grant().clone(),
                host: boot.host(),
                control: control.clone(),
                policy: self.policy.clone(),
                budget: self.budget.clone(),
                clock: self.clock.clone(),
            },
            self.audit.clone(),
        )?;
        #[cfg(feature = "object-v1")]
        let access = {
            let mut access = access;
            access.object_root = self.object_root.clone();
            access
        };
        Ok(Arc::new(InstalledEndpoint {
            access: Mutex::new(access),
            control,
            key: signer.signing,
            next: AtomicU64::new(1),
        }))
    }
}
impl OwnerAccessEndpoint for InstalledEndpoint {
    fn dispatch(
        &self,
        owner: &AuthenticatedOwner,
        request: GuestRequest,
        boot: &CurrentAccessBoot,
    ) -> FactoryResult<Vec<GuestResponse>> {
        boot.check()?;
        let mut access = self.access.try_lock().map_err(|_| Error::Busy)?;
        let connect = |deadline| {
            let stream = boot
                .connect_before(deadline)
                .map_err(|_| Error::Transport)?;
            Ok((stream, self.key.clone(), *boot.peer()))
        };
        match request {
            GuestRequest::DeliverDeclaredOperation(request) => {
                if request.owner != owner.owner() {
                    return Err(Error::Owner.into());
                }
                Ok(vec![GuestResponse::OwnerAccessReply(
                    access.call_typed_connect(owner.owner(), request, connect)?,
                )])
            }
            GuestRequest::Exec {
                command,
                stdin,
                timeout_secs,
                deadline_unix_ms: Some(mut deadline),
            } => {
                let now = self_clock_millis(&*access.service.clock)?;
                if let Some(seconds) = timeout_secs {
                    deadline = deadline.min(
                        now.checked_add(seconds.checked_mul(1000).ok_or(Error::Expired)?)
                            .ok_or(Error::Expired)?,
                    );
                }
                let sequence = self
                    .next
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |next| {
                        next.checked_add(1)
                    })
                    .map_err(|_| Error::Limit)?;
                let mut identity = [0; 32];
                identity[..8].copy_from_slice(&sequence.to_be_bytes());
                identity[8..].copy_from_slice(&owner.key().as_bytes()[..24]);
                let reply = access.call_dev_connect(
                    owner.owner(),
                    DevCall {
                        id: identity,
                        command: &command,
                        stdin: stdin.as_deref(),
                        deadline_unix_ms: deadline,
                    },
                    connect,
                )?;
                let mut frames = Vec::with_capacity(3);
                if !reply.stdout.is_empty() {
                    frames.push(GuestResponse::ExecEvent(ExecEvent::Stdout {
                        chunk: reply.stdout,
                    }));
                }
                if !reply.stderr.is_empty() {
                    frames.push(GuestResponse::ExecEvent(ExecEvent::Stderr {
                        chunk: reply.stderr,
                    }));
                }
                frames.push(GuestResponse::ExecEvent(ExecEvent::Exit {
                    code: reply.code,
                }));
                Ok(frames)
            }
            _ => Err(Error::Unsupported.into()),
        }
    }
    fn revoke(&self) {
        let _ = self.control.revoke();
        if let Ok(mut access) = self.access.try_lock() {
            let _ = access.service.revoke();
        }
    }
}
fn self_clock_millis(clock: &dyn Clock) -> Result<u64> {
    u64::try_from(clock.now().unix.timestamp_millis()).map_err(|_| Error::Expired)
}
