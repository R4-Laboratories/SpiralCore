//! The original tenant daemon and signer share the supervisor's resource ledger.
use super::*;
use mvm_core::host_lifetime::PreparedHostLifetimeLease;
use mvm_vmm::host::{
    host_agent_spawn::{
        HostAgentParentPermit, HostAgentServicesParams, OwnedHostAgent, OwnedHostAgentInputs,
        OwnedHostAgentRegistrar, OwnedHostAgentSpawnError,
    },
    network_endpoint_spawn::{EgressProxySpawnConfig, InstalledEndpointSettings},
};
use std::num::NonZeroU64;

// Maximum serialized register journal in mvm-hostd::broker::daemon::owned.
// Its original and replacement coexist until the atomic rename.
const JOURNAL_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy)]
pub struct TenantHelperLimits {
    /// Complete per-process virtual address space, inherited by the only signer.
    pub process_address_space_bytes: NonZeroU64,
    /// Cumulative logical bytes in each original persistent VM audit chain.
    pub audit_chain_bytes: NonZeroU64,
}

impl TenantHelperLimits {
    pub(super) fn claim(&self) -> Result<ManagedClaim> {
        Ok(ManagedClaim {
            host_carrier_bytes: self
                .process_address_space_bytes
                .get()
                .checked_mul(2)
                .ok_or_else(|| anyhow!("tenant helper address-space claim overflow"))?,
            // Bound two parent-side control frames plus fixed ownership/paths.
            host_metadata_bytes: 2 * JOURNAL_BYTES + super::BOOKKEEPING_BYTES + 4 * 4096,
            staging_disk_bytes: 2 * JOURNAL_BYTES,
            // The pinned owned binaries each use their main thread plus Tokio's
            // two worker threads and at most two blocking threads.
            workers: u64::try_from(mvm_vmm::host::host_agent_spawn::OWNED_TENANT_MAX_THREADS)?,
            ..Default::default()
        })
    }
}

pub(super) struct FundedPermit {
    shared: Weak<Shared>,
    tenant: String,
    holder: PreparedHostLifetimeLease,
    reservation: ManagedReservation,
    limits: TenantHelperLimits,
    settings: InstalledEndpointSettings,
}

impl FundedPermit {
    pub(super) fn check_authority(&self, registrar: &OwnedHostAgentRegistrar) -> Result<()> {
        self.check_current(&self.tenant)?;
        registrar.check_current_for_tenant(&self.tenant)?;
        registrar.check_original_holder(&self.holder)?;
        ensure!(
            registrar
                .process_address_space_limit()
                .is_some_and(|n| n <= self.limits.process_address_space_bytes),
            "original tenant helper lacks its funded address-space limit"
        );
        ensure!(
            registrar.audit_chain_bytes() == Some(self.limits.audit_chain_bytes),
            "original signer audit limit differs"
        );
        Ok(())
    }
}

impl HostAgentParentPermit for FundedPermit {
    fn check_current(&self, tenant: &str) -> Result<()> {
        ensure!(self.tenant == tenant, "foreign tenant helper permit");
        self.reservation.check_current()?;
        self.holder.verify_current()?;
        let parent = self
            .shared
            .upgrade()
            .ok_or_else(|| anyhow!("original parent missing"))?;
        ensure!(
            !parent
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?
                .closed,
            "parent admission closed"
        );
        Ok(())
    }

    fn owned_process_address_space_limit(&self, tenant: &str) -> Result<Option<NonZeroU64>> {
        self.check_current(tenant)?;
        Ok(Some(self.limits.process_address_space_bytes))
    }

    fn owned_audit_chain_bytes(&self, tenant: &str) -> Result<Option<NonZeroU64>> {
        self.check_current(tenant)?;
        Ok(Some(self.limits.audit_chain_bytes))
    }

    fn check_registration(&self, params: &HostAgentServicesParams<'_>) -> Result<()> {
        self.check_current(
            params
                .tenant_id
                .ok_or_else(|| anyhow!("registration tenant missing"))?,
        )?;
        let parent = self
            .shared
            .upgrade()
            .ok_or_else(|| anyhow!("original parent missing"))?;
        let grant = parent
            .registry
            .lock()
            .map_err(|_| anyhow!("parent registry poisoned"))?
            .slots
            .iter()
            .filter_map(Weak::upgrade)
            .find(|g| g.binding.vm_name == params.vm_name && g.tenant == self.tenant)
            .ok_or_else(|| anyhow!("original funded VM missing"))?;
        grant.check_current(&grant.binding)?;
        let facts = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .registration
            .clone()
            .ok_or_else(|| anyhow!("original registration facts missing"))?;
        facts.check_request(params)?;
        parent.reserve_audit_debt(&grant, &facts, self.limits.audit_chain_bytes)
    }

    fn installed_endpoint_settings(
        &self,
        params: &HostAgentServicesParams<'_>,
    ) -> Result<&InstalledEndpointSettings> {
        self.check_registration(params)?;
        Ok(&self.settings)
    }
}

pub(super) enum TenantSlot {
    Vacant,
    Ready {
        original: OwnedHostAgent,
        permit: Arc<FundedPermit>,
    },
    // An error after spawn carries the original child in OwnedHostAgentSpawnError.
    Uncertain {
        error: anyhow::Error,
        permit: Arc<FundedPermit>,
    },
    Retired,
}

pub(super) struct AuditDebt {
    grant_serial: u64,
    scope: [u8; 32],
    cap: NonZeroU64,
    reservation: ManagedReservation,
}

impl Shared {
    pub(super) fn reserve_audit_debt(
        &self,
        grant: &VmGrant,
        facts: &host_agent::RegistrationBinding,
        cap: NonZeroU64,
    ) -> Result<()> {
        let mut slots = self
            .audit_debts
            .lock()
            .map_err(|_| anyhow!("audit debt inventory poisoned"))?;
        let scope = facts.commitment();
        if let Some(debt) = slots
            .iter()
            .flatten()
            .find(|d| d.grant_serial == grant.binding.grant_serial)
        {
            ensure!(
                debt.cap == cap && debt.scope == scope,
                "original retained audit claim differs"
            );
            return debt.reservation.check_current().map_err(Into::into);
        }
        let index = slots
            .iter()
            .position(Option::is_none)
            .ok_or_else(|| anyhow!("original retained audit slots full"))?;
        let reservation = self.pool.reserve(
            tenant_tag(&grant.tenant),
            ManagedClaim {
                // Audit data survives both native actors. Bound the original and
                // temporary secondary chain head plus the one private inventory row.
                cache_disk_bytes: cap
                    .get()
                    .checked_add(2 * 4096)
                    .ok_or_else(|| anyhow!("audit debt overflow"))?,
                ..Default::default()
            },
        )?;
        slots[index] = Some(AuditDebt {
            grant_serial: grant.binding.grant_serial,
            scope,
            cap,
            reservation,
        });
        let result =
            self.issuer
                .record_audit_debt(index, grant.binding.grant_serial, scope, cap.get());
        if result.is_err() {
            // Any partial publication is ambiguous. A retry cannot infer a committed record.
            self.issuer.close_admission();
        }
        result.map_err(Into::into)
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        for debt in self
            .audit_debts
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .iter_mut()
            .filter_map(Option::take)
        {
            std::mem::forget(debt.reservation);
        }
        let slot = self
            .tenant_helper
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner());
        // The canonical owner can still positively shut down on drop. If its
        // original retirement fails, MVM keeps the live processes quarantined;
        // retain the corresponding reservation too.
        if retire_processes(slot).is_err() {
            match slot {
                TenantSlot::Ready { permit, .. } | TenantSlot::Uncertain { permit, .. } => {
                    std::mem::forget(permit.reservation.clone());
                }
                TenantSlot::Vacant | TenantSlot::Retired => {}
            }
        }
    }
}
fn retire_processes(slot: &mut TenantSlot) -> Result<()> {
    match slot {
        TenantSlot::Ready { original, permit } => {
            permit.reservation.check_retained_identity()?;
            original.shutdown()
        }
        TenantSlot::Uncertain { error, permit } => {
            permit.reservation.check_retained_identity()?;
            error
                .downcast_mut::<OwnedHostAgentSpawnError>()
                .ok_or_else(|| anyhow!("original tenant startup disposal unproved"))?
                .agent_mut()
                .shutdown()
        }
        TenantSlot::Retired => Ok(()),
        TenantSlot::Vacant => anyhow::bail!("original tenant helper was not started"),
    }
}
impl Shared {
    pub(super) fn tenant_registrar(
        &self,
        tenant: &str,
    ) -> Result<(OwnedHostAgentRegistrar, Arc<FundedPermit>)> {
        let slot = self
            .tenant_helper
            .lock()
            .map_err(|_| anyhow!("tenant helper custody poisoned"))?;
        match &*slot {
            TenantSlot::Ready { original, permit } if permit.tenant == tenant => {
                Ok((original.registrar(), permit.clone()))
            }
            _ => Err(anyhow!("original registered tenant daemon missing")),
        }
    }
}

impl InstalledParentProvider {
    /// Fund the existing MVM owned-host-agent launch, once, before any VM admission.
    pub fn start_tenant_helper(&mut self, tenant: &str, limits: TenantHelperLimits) -> Result<()> {
        mvm_core::naming::validate_vm_name(tenant)?;
        ensure!(
            OwnedHostAgent::supports_process_address_space_limit(),
            "tenant process memory enforcement unavailable"
        );
        let parent = &self.shared;
        {
            let registry = parent
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?;
            ensure!(
                !registry.closed && registry.slots.iter().all(|s| s.strong_count() == 0),
                "tenant helper requires an unopened VM registry"
            );
        }
        let mut slot = parent
            .tenant_helper
            .lock()
            .map_err(|_| anyhow!("tenant helper custody poisoned"))?;
        ensure!(
            matches!(*slot, TenantSlot::Vacant),
            "original tenant helper already issued"
        );
        let reservation = parent.pool.reserve(tenant_tag(tenant), limits.claim()?)?;
        let settings = match EgressProxySpawnConfig::from_host_config()? {
            None => InstalledEndpointSettings::ResolvedDirect,
            Some(_) => anyhow::bail!("configured proxy outside funded direct endpoint profile"),
        };
        let holder = || -> Result<PreparedHostLifetimeLease> {
            let (_, h) = parent.issuer.prepare()?;
            PreparedHostLifetimeLease::bind(
                h.directory,
                h.holder,
                HostLeaseExpectation {
                    state_name: "state.bin".into(),
                    state_bytes: h.state_bytes,
                    state_sha256: h.state_sha256,
                    absent_names: [".state.next".into(), ".state.intent".into()],
                },
            )
        };
        let permit = Arc::new(FundedPermit {
            shared: Arc::downgrade(parent),
            tenant: tenant.into(),
            holder: holder()?,
            reservation,
            limits,
            settings,
        });
        let inputs = OwnedHostAgentInputs {
            tenant: tenant.into(),
            holder: holder()?,
            parent: permit.clone(),
        };
        permit.check_current(tenant)?;
        match OwnedHostAgent::spawn(inputs) {
            Ok(original) => {
                let registrar = original.registrar();
                *slot = TenantSlot::Ready {
                    original,
                    permit: permit.clone(),
                };
                permit.check_authority(&registrar)
            }
            Err(error) => {
                *slot = TenantSlot::Uncertain { error, permit };
                anyhow::bail!("original tenant startup failed; cleanup custody retained")
            }
        }
    }

    /// Keep both process debits until MVM confirms original signer and daemon cleanup.
    pub fn stop_tenant_helper(&mut self) -> Result<()> {
        let mut slot = self
            .shared
            .tenant_helper
            .lock()
            .map_err(|_| anyhow!("tenant helper custody poisoned"))?;
        retire_processes(&mut slot)?;
        // Disarm the conservative Drop disposition only after canonical disposal.
        *slot = TenantSlot::Retired;
        Ok(())
    }
}
