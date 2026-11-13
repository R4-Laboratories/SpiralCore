//! Original installed portable workspace and storage custody.
use super::*;

use mvm_core::{host_lifetime::HostLifetimeAnchor, plan::PlanArtifact};
use std::num::NonZeroU64;

/// Installer-supplied bounds for one verified portable bundle. Workspace
/// excludes the archive input buffer. Storage excludes the retained archive
/// copy but includes expanded files, scratch, snapshots, and cache overlap.
pub struct PortableStartupQuote {
    pin: PlanArtifact,
    archive_bytes: NonZeroU64,
    claim: ManagedClaim,
}

impl PortableStartupQuote {
    pub fn new(
        pin: PlanArtifact,
        archive_bytes: NonZeroU64,
        workspace_bytes: NonZeroU64,
        storage_bytes: NonZeroU64,
    ) -> Result<Self> {
        ensure!(
            mvm_core::manifest::is_slot_hash_dirname(&pin.bundle_sha256)
                && pin.signature_bytes().is_some(),
            "invalid original portable archive pin"
        );
        let sum = |additional: NonZeroU64| {
            archive_bytes
                .get()
                .checked_add(additional.get())
                .ok_or_else(|| anyhow!("portable startup charge overflow"))
        };
        Ok(Self {
            pin,
            archive_bytes,
            claim: ManagedClaim {
                host_metadata_bytes: sum(workspace_bytes)?,
                staging_disk_bytes: sum(storage_bytes)?,
                ..Default::default()
            },
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Held,
    Active,
    Complete,
}

pub(super) struct StartupWorkspace {
    quote: PortableStartupQuote,
    credit: ManagedReservation,
    pub(super) phase: Phase,
}

impl PreparedParentVm {
    pub fn with_portable_startup(self, quote: PortableStartupQuote) -> Result<Self> {
        self.grant.check_open()?;
        let parent = self
            .grant
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let credit = parent
            .pool
            .reserve(tenant_tag(&self.grant.tenant), quote.claim)?;
        {
            let mut lifecycle = self
                .grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle.check_launch_open()?;
            ensure!(
                lifecycle.admitted_plan.is_none()
                    && lifecycle.endpoint == EndpointIssuance::NeverIssued
                    && lifecycle.startup.is_none(),
                "original portable workspace already funded or launch advanced"
            );
            lifecycle.startup = Some(StartupWorkspace {
                quote,
                credit,
                phase: Phase::Held,
            });
        }
        Ok(self)
    }
}

impl VmGrant {
    pub(super) fn bind_portable_quote(
        &self,
        lifetime: PreparedHostLifetime,
    ) -> Result<PreparedHostLifetime> {
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        match &lifecycle.startup {
            Some(startup) => {
                ensure!(
                    startup.phase == Phase::Held,
                    "original portable workspace already started"
                );
                lifetime
                    .with_portable_archive(startup.quote.pin.clone(), startup.quote.archive_bytes)
            }
            None => Ok(lifetime),
        }
    }

    pub(super) fn check_portable_quote(
        &self,
        owner: &Arc<HostLifetimeAnchor>,
        pin: &PlanArtifact,
    ) -> Result<()> {
        self.check_open()?;
        let bytes = owner.portable_archive_bytes(pin)?;
        let credit = {
            let lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle.check_launch_open()?;
            let startup = lifecycle
                .startup
                .as_ref()
                .ok_or_else(|| anyhow!("original portable workspace unavailable"))?;
            ensure!(
                startup.phase == Phase::Held
                    && &startup.quote.pin == pin
                    && startup.quote.archive_bytes == bytes,
                "portable quote differs from the original held workspace"
            );
            startup.credit.clone()
        };
        credit.check_current()?;
        Ok(())
    }

    pub(super) fn transition_portable_workspace(
        &self,
        binding: &HostLaunchBinding,
        from: Phase,
        to: Phase,
    ) -> Result<()> {
        self.check_current(binding)?;
        let credit = {
            let lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle
                .startup
                .as_ref()
                .ok_or_else(|| anyhow!("original portable workspace unavailable"))?
                .credit
                .clone()
        };
        credit.check_current()?;
        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        let startup = lifecycle
            .startup
            .as_mut()
            .ok_or_else(|| anyhow!("original portable workspace unavailable"))?;
        ensure!(
            startup.phase == from,
            "original portable workspace phase differs"
        );
        startup.phase = to;
        Ok(())
    }
}
