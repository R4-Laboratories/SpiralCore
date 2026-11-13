//! Original installed portable workspace and storage custody.
use super::*;

use mvm_core::{host_lifetime::HostLifetimeAnchor, plan::PlanArtifact};
use std::{fs::File, num::NonZeroU64};

/// Installer-supplied bounds for one verified portable bundle. Workspace
/// excludes the archive input buffer. Storage excludes the retained archive
/// copy but includes expanded files, scratch, snapshots, and cache overlap.
pub struct PortableStartupQuote {
    pin: PlanArtifact,
    archive_bytes: NonZeroU64,
    claim: ManagedClaim,
    #[cfg(target_os = "linux")]
    workspace_disk: u64,
}

#[cfg(target_os = "linux")]
struct CanonicalWorkspace {
    kernel: Arc<mvm_core::canonical_linux::PreparedCanonicalLinux>,
    bounds: super::startup_bounds::StartupBounds,
    root_bytes: NonZeroU64,
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
            #[cfg(target_os = "linux")]
            workspace_disk: storage_bytes.get(),
            claim: ManagedClaim {
                host_metadata_bytes: sum(workspace_bytes)?,
                staging_disk_bytes: sum(storage_bytes)?
                    .checked_add(4096)
                    .ok_or_else(|| anyhow!("portable inventory block overflow"))?,
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
    #[cfg(target_os = "linux")]
    canonical: Option<CanonicalWorkspace>,
}

// Scope, original archive pin and all charged dimensions are committed before
// the caller may touch materialized files. Each slot survives provider rotation.
struct Debt {
    serial: u64,
    commitment: [u8; 32],
    credit: ManagedReservation,
    marker: Option<File>,
}

#[derive(Default)]
pub(crate) struct Inventory(
    [Option<Debt>; crate::installed_owner::retained_telemetry::MAX_ENTRIES],
);

impl Drop for Inventory {
    fn drop(&mut self) {
        // The installation denial latch survives the controller; a process
        // dropping its last supervisor cannot declare the artifacts disposed.
        for debt in self.0.iter_mut().filter_map(Option::take) {
            std::mem::forget(debt.credit);
        }
    }
}

impl PreparedParentVm {
    /// Reserve a complete process and filesystem envelope before asking the
    /// canonical SDK to authenticate an already installed bundle. The returned
    /// kernel must be passed unchanged to try_launch_with_canonical_direct_policy.
    #[cfg(target_os = "linux")]
    pub fn prepare_canonical_startup(
        self,
        quote: PortableStartupQuote,
    ) -> Result<(Self, Arc<mvm_core::canonical_linux::PreparedCanonicalLinux>)> {
        ensure!(
            self.grant.binding.backend == BackendKind::Firecracker,
            "canonical startup requires the original Firecracker binding"
        );
        let home = mvm_core::config::mvm_home_strict()?;
        let scratch = std::env::temp_dir();
        let state = mvm_core::config::vm_state_dir_at(&home, &self.grant.binding.vm_name);
        let tenant = mvm_core::config::host_agent_dir_at(&home, &self.grant.tenant);
        ensure!(
            mvm_core::config::vm_socket_dir_at(&state) == state
                && mvm_core::config::host_agent_socket_dir_in(&tenant) == tenant,
            "startup sockets must remain in the funded filesystem"
        );
        let pin = quote.pin.clone();
        let archive_bytes = quote.archive_bytes;
        let claim = quote.claim;
        let storage = quote.workspace_disk;
        let disk = archive_bytes.get() + storage; // checked by PortableStartupQuote::new
        let prepared = self.with_portable_startup(quote)?;
        let bounds = super::startup_bounds::StartupBounds::capture(
            &home,
            &scratch,
            claim.host_metadata_bytes,
            disk,
        )?;
        let registry = mvm_core::plan::bundle::BundleRegistry::default_path()?;
        let archive = registry.archive_path(&pin.bundle_sha256);
        // Preparation may leave authenticated files even when it later refuses.
        prepared.grant.retain_portable_debt(&pin, archive_bytes)?;
        let kernel = Arc::new(
            mvm_client::launch::canonical_linux::prepare_funded_canonical_linux(
                &archive,
                &pin,
                BackendKind::Firecracker,
                archive_bytes,
            )?,
        );
        ensure!(
            kernel.pin() == &pin
                && kernel.arch() == mvm_core::arch::GuestArch::host()
                && kernel.backend() == BackendKind::Firecracker,
            "prepared kernel differs from the original installed bundle"
        );
        let root_bytes = NonZeroU64::new(kernel.workload_root_declaration()?.size_bytes)
            .ok_or_else(|| anyhow!("empty original workload root"))?;
        ensure!(
            root_bytes
                .get()
                .checked_add(kernel.kernel_declaration().size)
                .is_some_and(|both| both <= storage),
            "signed root and kernel exceed charged storage"
        );
        bounds.check(
            &mvm_core::config::mvm_home_strict()?,
            &std::env::temp_dir(),
            claim.host_metadata_bytes,
            disk,
        )?;
        bounds.check_generation(kernel.generation_directory_file())?;
        {
            let mut lifecycle = prepared
                .grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            lifecycle.check_launch_open()?;
            lifecycle
                .startup
                .as_mut()
                .ok_or_else(|| anyhow!("original startup reservation missing"))?
                .canonical = Some(CanonicalWorkspace {
                kernel: kernel.clone(),
                bounds,
                root_bytes,
            });
        }
        Ok((prepared, kernel))
    }

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
                #[cfg(target_os = "linux")]
                canonical: None,
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

    #[cfg(target_os = "linux")]
    pub(super) fn check_original_kernel(
        &self,
        kernel: &Arc<mvm_core::canonical_linux::PreparedCanonicalLinux>,
        copy: Option<(&File, NonZeroU64)>,
    ) -> Result<()> {
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        let startup = lifecycle
            .startup
            .as_ref()
            .ok_or_else(|| anyhow!("original startup absent"))?;
        let required_phase = if copy.is_some() {
            Phase::Active
        } else {
            Phase::Held
        };
        ensure!(
            startup.phase == required_phase,
            "original startup phase differs"
        );
        let prepared = startup
            .canonical
            .as_ref()
            .ok_or_else(|| anyhow!("prepared canonical startup absent"))?;
        ensure!(
            Arc::ptr_eq(&prepared.kernel, kernel),
            "foreign original canonical kernel"
        );
        startup.credit.check_current()?;
        prepared.bounds.check(
            &mvm_core::config::mvm_home_strict()?,
            &std::env::temp_dir(),
            startup.quote.claim.host_metadata_bytes,
            startup.quote.archive_bytes.get() + startup.quote.workspace_disk,
        )?;
        prepared
            .bounds
            .check_generation(kernel.generation_directory_file())?;
        if let Some((directory, bytes)) = copy {
            ensure!(
                bytes == prepared.root_bytes
                    && super::host_agent::same_directory(
                        directory,
                        kernel.generation_directory_file()
                    )?,
                "root copy differs from the originally paid file extent or generation"
            );
        }
        Ok(())
    }

    pub(super) fn check_portable_quote(
        &self,
        owner: &Arc<HostLifetimeAnchor>,
        pin: &PlanArtifact,
    ) -> Result<()> {
        self.check_open()?;
        self.retain_portable_debt(pin, owner.portable_archive_bytes(pin)?)
    }

    fn retain_portable_debt(&self, pin: &PlanArtifact, bytes: NonZeroU64) -> Result<()> {
        self.check_open()?;
        let (credit, commitment) = {
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
            let commitment = Sha256::digest(serde_json::to_vec(&(
                "SpiralCore/installed-portable-debt/v1",
                &self.tenant,
                &self.binding,
                pin,
                bytes.get(),
                startup.quote.claim.host_metadata_bytes,
                startup.quote.claim.staging_disk_bytes,
                startup.credit.serial(),
            ))?)
            .into();
            (startup.credit.clone(), commitment)
        };
        credit.check_current()?;
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let mut inventory = parent
            .portable_debts
            .lock()
            .map_err(|_| anyhow!("portable inventory poisoned"))?;
        let serial = credit.serial();
        let index = inventory
            .0
            .iter()
            .position(|d| d.as_ref().is_some_and(|d| d.serial == serial));
        if let Some(index) = index {
            let debt = inventory.0[index]
                .as_ref()
                .expect("matched original portable debit");
            ensure!(
                debt.commitment == commitment,
                "original portable commitment differs"
            );
            let marker = debt
                .marker
                .as_ref()
                .ok_or_else(|| anyhow!("original portable record uncertain"))?;
            parent
                .issuer
                .portable_debt(index, &commitment, Some(marker))?;
        } else {
            let index = inventory
                .0
                .iter()
                .position(Option::is_none)
                .ok_or_else(|| anyhow!("portable inventory full"))?;
            let debt = inventory.0[index].insert(Debt {
                serial,
                commitment,
                credit,
                marker: None,
            });
            match parent.issuer.portable_debt(index, &commitment, None) {
                Ok(file) => debt.marker = Some(file),
                Err(error) => {
                    parent.issuer.close_admission();
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    pub(super) fn check_retired_portable_debt(&self) -> Result<()> {
        let serial = {
            let lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            let Some(startup) = &lifecycle.startup else {
                return Ok(());
            };
            startup.credit.check_retained_identity()?;
            startup.credit.serial()
        };
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let inventory = parent
            .portable_debts
            .lock()
            .map_err(|_| anyhow!("portable inventory poisoned"))?;
        let (index, debt) = inventory
            .0
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                item.as_ref()
                    .filter(|d| d.serial == serial)
                    .map(|d| (index, d))
            })
            .ok_or_else(|| anyhow!("original portable debit not retained"))?;
        let marker = debt
            .marker
            .as_ref()
            .ok_or_else(|| anyhow!("original portable record uncertain"))?;
        parent
            .issuer
            .portable_debt(index, &debt.commitment, Some(marker))?;
        debt.credit.check_retained_identity()?;
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
