//! In-place transfer of the immutable original signer chain into the same
//! installation's finite archive. Its conservative full disk charge survives.
use super::*;
use mvm_hostd::audit_signer::verify::{WorkloadChainExpectation, verify_original_workload_chain};
use mvm_vmm::host::host_agent_spawn::RegisteredAuditFile;
use std::{fs::File, num::NonZeroU64};

struct Debt {
    serial: u64,
    scope: [u8; 32],
    name: [u8; 32],
    cap: NonZeroU64,
    reservation: ManagedReservation,
    record: Option<File>,
    generation: Option<([u8; 32], [u8; 32])>,
}

struct Archive {
    debt: Debt,
    statement: Vec<u8>,
    _file: File,
}

#[derive(Default)]
pub(crate) struct Inventory {
    active: [Option<Debt>; 2],
    archive: [Option<Archive>; crate::installed_owner::retained_telemetry::MAX_ENTRIES],
}

impl Drop for Inventory {
    fn drop(&mut self) {
        for debt in self.active.iter_mut().filter_map(Option::take).chain(
            self.archive
                .iter_mut()
                .filter_map(Option::take)
                .map(|a| a.debt),
        ) {
            std::mem::forget(debt.reservation);
        }
    }
}

fn scope_name(grant: &VmGrant) -> Result<[u8; 32]> {
    Ok(Sha256::digest(serde_json::to_vec(&(
        &grant.tenant,
        &grant.binding.vm_name,
    ))?)
    .into())
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
        let name = scope_name(grant)?;
        ensure!(
            !slots.archive.iter().flatten().any(|a| a.debt.name == name),
            "original signer chain already archived"
        );
        let scope = facts.commitment();
        let serial = grant.binding.grant_serial;
        if let Some(existing) = slots
            .active
            .iter()
            .flatten()
            .find(|d| d.serial == serial || d.name == name)
        {
            ensure!(
                existing.serial == serial
                    && existing.scope == scope
                    && existing.cap == cap
                    && existing.record.is_some(),
                "original audit reservation differs or is incomplete"
            );
            return existing.reservation.check_current().map_err(Into::into);
        }
        let index = slots
            .active
            .iter()
            .position(Option::is_none)
            .ok_or_else(|| anyhow!("original retained audit slots full"))?;
        let reservation = self.pool.reserve(
            tenant_tag(&grant.tenant),
            ManagedClaim {
                cache_disk_bytes: cap
                    .get()
                    .checked_add(2 * 4096)
                    .ok_or_else(|| anyhow!("audit debt overflow"))?,
                host_metadata_bytes: BOOKKEEPING_BYTES,
                ..Default::default()
            },
        )?;
        let debt = slots.active[index].insert(Debt {
            serial,
            scope,
            name,
            cap,
            reservation,
            record: None,
            generation: None,
        });
        match self
            .issuer
            .record_audit_debt(index, serial, scope, cap.get())
        {
            Ok(record) => {
                debt.record = Some(record);
                Ok(())
            }
            Err(e) => {
                self.issuer.close_admission();
                Err(e.into())
            }
        }
    }

    pub(super) fn enroll_audit(&self, grant: &VmGrant, file: &RegisteredAuditFile) -> Result<()> {
        let opened = file.registered();
        ensure!(
            opened.tenant == grant.tenant && opened.vm == grant.binding.vm_name,
            "original audit registration scope differs"
        );
        let mut inventory = self
            .audit_debts
            .lock()
            .map_err(|_| anyhow!("audit debt inventory poisoned"))?;
        let debt = inventory
            .active
            .iter_mut()
            .flatten()
            .find(|d| d.serial == grant.binding.grant_serial)
            .ok_or_else(|| anyhow!("original audit debit missing"))?;
        let identity = (opened.generation, file.verifying_key());
        ensure!(
            debt.generation.is_none_or(|present| present == identity) && debt.record.is_some(),
            "original signed audit generation differs"
        );
        debt.generation = Some(identity);
        Ok(())
    }

    pub(super) fn adopt_audit(&self, grant: &VmGrant, file: &RegisteredAuditFile) -> Result<()> {
        let terminal = file
            .terminal()
            .ok_or_else(|| anyhow!("signed terminal missing"))?;
        ensure!(
            file.registered().same_registration(terminal)
                && terminal.tenant == grant.tenant
                && terminal.vm == grant.binding.vm_name,
            "foreign terminal registration"
        );
        let identity = (terminal.generation, file.verifying_key());
        let serial = grant.binding.grant_serial;
        let mut inventory = self
            .audit_debts
            .lock()
            .map_err(|_| anyhow!("audit debt inventory poisoned"))?;
        let debt = inventory
            .active
            .iter()
            .flatten()
            .chain(inventory.archive.iter().flatten().map(|a| &a.debt))
            .find(|d| d.serial == serial)
            .ok_or_else(|| anyhow!("original audit debit missing"))?;
        ensure!(
            debt.generation == Some(identity),
            "terminal signer or generation differs"
        );
        let cap = debt.cap;
        debt.reservation.check_retained_identity()?;
        drop(inventory);
        // Check the signed chain without holding the shared inventory through I/O.
        let key = ed25519_dalek::VerifyingKey::from_bytes(&file.verifying_key())?;
        let result = verify_original_workload_chain(
            file.file(),
            &key,
            &WorkloadChainExpectation {
                tenant: &terminal.tenant,
                workload: &terminal.workload,
                head: &terminal.head,
                length: terminal.length,
                cap: cap.get(),
                device: terminal.dev,
                inode: terminal.ino,
            },
        )?;
        let statement = serde_json::to_vec(&(
            terminal,
            file.verifying_key(),
            result.entries,
            result.sha256,
        ))?;
        let original = file.file().try_clone()?;
        inventory = self
            .audit_debts
            .lock()
            .map_err(|_| anyhow!("audit debt inventory poisoned"))?;
        if let Some(a) = inventory
            .archive
            .iter()
            .flatten()
            .find(|a| a.debt.serial == serial)
        {
            ensure!(
                a.statement == statement,
                "archived original terminal differs"
            );
            return a
                .debt
                .reservation
                .check_retained_identity()
                .map_err(Into::into);
        }
        let index = inventory
            .active
            .iter()
            .position(|d| {
                d.as_ref()
                    .is_some_and(|d| d.serial == serial && d.generation == Some(identity))
            })
            .ok_or_else(|| anyhow!("original audit debt changed"))?;
        let slot = inventory
            .archive
            .iter()
            .position(Option::is_none)
            .ok_or_else(|| anyhow!("retained audit archive full"))?;
        let debt = inventory.active[index]
            .as_ref()
            .expect("matched active debt");
        self.issuer.archive_audit(
            index,
            slot,
            debt.record
                .as_ref()
                .ok_or_else(|| anyhow!("original debt record missing"))?,
            &statement,
        )?;
        let debt = inventory.active[index]
            .take()
            .expect("same original debt held under inventory guard");
        inventory.archive[slot] = Some(Archive {
            debt,
            statement,
            _file: original,
        });
        Ok(())
    }
}
