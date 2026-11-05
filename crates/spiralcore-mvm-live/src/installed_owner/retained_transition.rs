//! Actual closed debt transfer. This module exports NO native issuer or grant.
use super::super::transfer::{TransferIntent, TransferKind};
use super::*;
use std::sync::Mutex;

pub(super) struct CompiledRetainedMigration {
    target_profile: [u8; 32],
}
impl CompiledRetainedMigration {
    pub(super) fn current() -> RecoveryResult<Self> {
        Ok(Self {
            target_profile: RetainedRecoveryCompatibility::current()?.profile,
        })
    }
}
struct Transaction {
    recovery: Option<RetainedRecovery>,
    intent: Option<TransferIntent>,
    target_bytes: Vec<u8>,
    pending_storage: Option<Arc<Storage>>,
    old_state: Option<File>,
    current_state_observation: Option<File>,
    intent_staging: Option<File>,
    catalog_staging: Option<File>,
    state_intent: Option<File>,
    state_staging: Option<File>,
    acquisition: Option<Box<TransferFiles>>,
    resuming: Option<Arc<ClosedTransferCurrent>>,
}
pub(super) struct PreparedRetainedRestart {
    held: Option<Box<Transaction>>,
}
impl Drop for PreparedRetainedRestart {
    fn drop(&mut self) {
        if let Some(held) = self.held.take() {
            std::mem::forget(held);
        }
    }
}
pub(super) struct RetainedRestartFailure {
    _error: SupervisorError,
    _original: PreparedRetainedRestart,
}
impl fmt::Debug for RetainedRestartFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(test)]
        {
            f.debug_tuple("retained transaction refused; synthetic test diagnostics")
                .field(&self._error)
                .finish()
        }
        #[cfg(not(test))]
        f.write_str("retained transaction refused; same original closed custody retained")
    }
}
pub(super) struct CommittedRetainedRestart {
    closed: ClosedRetainedDebt,
}

/// One owned historical table and its ACTUAL closed ledger, owner, exclusions
/// and original File pins. No second metadata reservation or row table is
/// constructed here. Opening a new authority is a separate consuming step.
pub(super) struct PreparedRetainedInventory {
    original: Mutex<ClosedRetainedDebt>,
}
pub(super) struct PreparedRetainedInventorySnapshot {
    pub(super) historical_rows: usize,
    pub(super) new_rows: usize,
    pub(super) remaining_slots: usize,
    pub(super) managed: ManagedSnapshot,
}
pub(super) struct RetainedInventoryPreparationFailure {
    _error: SupervisorError,
    _original: CommittedRetainedRestart,
}
impl fmt::Debug for RetainedInventoryPreparationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("historical inventory preparation refused; original custody retained")
    }
}
impl PreparedRetainedInventory {
    #[cfg(test)]
    pub(super) fn original_custody_identity(&self) -> (usize, usize) {
        let original = self.original.lock().unwrap();
        let held = original.recovery.held.as_ref().unwrap();
        (
            std::ptr::from_ref(&held.entries) as usize,
            held.funding().unwrap().original_ledger_address(),
        )
    }
    fn check_locked(original: &mut ClosedRetainedDebt) -> RecoveryResult<()> {
        let checked = (|| {
            original.revalidate()?;
            let held = original
                .recovery
                .held
                .as_ref()
                .ok_or(SupervisorError::State)?;
            if held
                .entries
                .iter()
                .flatten()
                .any(|row| row.record.phase != 4)
            {
                return Err(SupervisorError::State);
            }
            held.funding()?.can_rebind(held.epoch)?;
            Ok(())
        })();
        if checked.is_err()
            && let Some(held) = original.recovery.held.as_mut()
        {
            held.failed = true;
        }
        checked
    }
    pub(super) fn check_current(&self) -> RecoveryResult<()> {
        let mut original = self.original.lock().map_err(|_| SupervisorError::State)?;
        Self::check_locked(&mut original)
    }
    pub(super) fn snapshot(&self) -> RecoveryResult<PreparedRetainedInventorySnapshot> {
        let mut original = self.original.lock().map_err(|_| SupervisorError::State)?;
        Self::check_locked(&mut original)?;
        let snapshot = original.snapshot()?;
        Ok(PreparedRetainedInventorySnapshot {
            historical_rows: snapshot.entries,
            // No new authority or ordinary reserve is available while CLOSED.
            new_rows: 0,
            remaining_slots: MAX_ENTRIES
                .checked_sub(snapshot.entries)
                .ok_or(SupervisorError::State)?,
            managed: snapshot.managed,
        })
    }
}
impl CommittedRetainedRestart {
    #[cfg(test)]
    pub(super) fn original_custody_identity(&self) -> (usize, usize) {
        let held = self.closed.recovery.held.as_ref().unwrap();
        (
            std::ptr::from_ref(&held.entries) as usize,
            held.funding().unwrap().original_ledger_address(),
        )
    }
    pub(super) fn prepare_live_inventory(
        mut self,
    ) -> std::result::Result<PreparedRetainedInventory, RetainedInventoryPreparationFailure> {
        if let Err(error) = PreparedRetainedInventory::check_locked(&mut self.closed) {
            return Err(RetainedInventoryPreparationFailure {
                _error: error,
                _original: self,
            });
        }
        Ok(PreparedRetainedInventory {
            original: Mutex::new(self.closed),
        })
    }
    pub(super) fn snapshot(&self) -> RecoveryResult<RetainedDebtSnapshot> {
        self.closed.snapshot()
    }
    pub(super) fn binding(&self) -> RecoveryResult<InstallationBinding> {
        let held = self
            .closed
            .recovery
            .held
            .as_ref()
            .ok_or(SupervisorError::State)?;
        held.funding()?
            .authority()
            .with_disk_owner(|owner, epoch| owner.active_binding(epoch))
    }
}
impl ClosedRetainedDebt {
    fn into_prepared(self) -> PreparedRetainedRestart {
        PreparedRetainedRestart {
            held: Some(Box::new(Transaction {
                recovery: Some(self.recovery),
                intent: None,
                target_bytes: Vec::new(),
                pending_storage: None,
                old_state: None,
                current_state_observation: None,
                intent_staging: None,
                catalog_staging: None,
                state_intent: None,
                state_staging: None,
                acquisition: None,
                resuming: None,
            })),
        }
    }

    pub(super) fn prepare_restart(
        self,
        migration: &CompiledRetainedMigration,
    ) -> std::result::Result<PreparedRetainedRestart, RetainedRestartFailure> {
        let mut prepared = self.into_prepared();
        let result = prepared
            .held
            .as_mut()
            .expect("armed transaction")
            .prepare(migration);
        match result {
            Ok(()) => Ok(prepared),
            Err(error) => Err(RetainedRestartFailure {
                _error: error,
                _original: prepared,
            }),
        }
    }
}
impl PreparedRetainedRestart {
    pub(super) fn commit_closed(
        mut self,
    ) -> std::result::Result<CommittedRetainedRestart, RetainedRestartFailure> {
        let result = self.held.as_mut().expect("armed transaction").commit();
        match result {
            Err(error) => Err(RetainedRestartFailure {
                _error: error,
                _original: self,
            }),
            Ok(()) => {
                let mut done = self.held.take().expect("committed transaction");
                let recovery = done.recovery.take().expect("actual committed custody");
                Ok(CommittedRetainedRestart {
                    closed: ClosedRetainedDebt { recovery },
                })
            }
        }
    }
}
fn create_observed(slot: &mut Option<File>, directory: &File, name: &str) -> Result<()> {
    if slot.is_some() {
        return Err(OwnerError::Fenced);
    }
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: original directory and fixed NUL-terminated name; exclusive
    // creation, explicit0600 mode, immediately owned before fallible checks.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(storage_open_error(io::Error::last_os_error()));
    }
    // SAFETY: fresh successful openat descriptor, installed into empty custody.
    *slot = Some(unsafe { File::from_raw_fd(fd) });
    require_private_retained_file(slot.as_ref().ok_or(OwnerError::Fenced)?)
}
fn named_is(directory: &File, name: &str, actual: &File) -> Result<()> {
    let named = open_at(directory, name, libc::O_RDONLY)?;
    if object_id(&named)? != object_id(actual)? {
        return Err(OwnerError::Fenced);
    }
    Ok(())
}
fn rename_exact(directory: &File, from: &str, to: &str, actual: &File) -> Result<()> {
    named_is(directory, from, actual)?;
    let from = CString::new(from).map_err(|_| OwnerError::InvalidStorage)?;
    let to = CString::new(to).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: fixed names under same original held directory.
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}
// Eligibility is denial only; it grants no disposable cleanup authority.
// Fresh preparation and interrupted transfer share this EXACT excluded set.
fn require_eligible_namespace(directory: &File) -> Result<()> {
    for name in [
        CATALOG_INTENT,
        ".endpoint-disks.bin",
        ".endpoint-disks.intent",
        ".endpoint-disks.next",
    ] {
        if present(directory, name)? {
            return Err(OwnerError::Fenced);
        }
    }
    super::super::super::endpoint_disks::no_namespaces(directory)
}
fn publish_immutable(directory: &File, from: &str, to: &str, actual: &File) -> Result<()> {
    named_is(directory, from, actual)?;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // No replacing-rename fallback. Unsupported filesystems retain the
        // armed staging File and all original CLOSED transaction custody.
        rustix::fs::renameat_with(
            directory,
            from,
            directory,
            to,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(|error| OwnerError::from(io::Error::from_raw_os_error(error.raw_os_error())))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "exclusive retained intent publication unsupported",
        )
        .into())
    }
}
fn unlink_exact(directory: &File, name: &str, actual: &File) -> Result<()> {
    named_is(directory, name, actual)?;
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: fixed private metadata name after exact held-inode comparison.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(io::Error::last_os_error().into());
    }
    directory.sync_all()?;
    Ok(())
}
impl Transaction {
    fn prepare(&mut self, migration: &CompiledRetainedMigration) -> RecoveryResult<()> {
        self.prepare_kind(migration, TransferKind::Restart)
    }
    fn prepare_kind(
        &mut self,
        migration: &CompiledRetainedMigration,
        kind: TransferKind,
    ) -> RecoveryResult<()> {
        let held = self
            .recovery
            .as_mut()
            .and_then(|value| value.held.as_mut())
            .ok_or(SupervisorError::State)?;
        held.revalidate()?;
        if held
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.record.phase != 4)
        {
            return Err(SupervisorError::State);
        }
        held.funding()?.can_rebind(held.epoch)?;
        let old_epoch = held.epoch;
        let target_epoch = match kind {
            TransferKind::Restart => Epoch {
                key: InstallationKey {
                    installation: old_epoch.key.installation,
                    profile: migration.target_profile,
                },
                generation: old_epoch
                    .generation
                    .checked_add(1)
                    .ok_or(OwnerError::EpochExhausted)?,
            },
            TransferKind::RetainedStop => old_epoch,
        };
        if !supported_profile(old_epoch.key.profile)
            || target_epoch.key.profile != RetainedRecoveryCompatibility::current()?.profile
        {
            return Err(SupervisorError::Profile);
        }
        let source = decode_catalog(&held.catalog_bytes, old_epoch)?;
        let source_mode = source.mode;
        let target = CatalogV2 {
            epoch: target_epoch,
            mode: match kind {
                TransferKind::Restart => CatalogMode::Active,
                TransferKind::RetainedStop => CatalogMode::StoppedRetained,
            },
            entries: source.entries,
        };
        self.target_bytes = target.encode()?;
        let authority = held.funding()?.authority();
        let (pending, intent) = authority.with_disk_owner(|owner, epoch| {
            owner.active_binding(epoch)?;
            require_eligible_namespace(&owner.storage.directory)?;
            owner
                .storage
                .absent_names(&[TRANSFER_INTENT, TRANSFER_NEXT, CATALOG_NEXT])?;
            observe_at(
                &mut self.old_state,
                &owner.storage.directory,
                STATE,
                libc::O_RDONLY,
            )?;
            if bounded_bytes(
                self.old_state.as_ref().ok_or(OwnerError::Fenced)?,
                RECORD_BYTES,
            )? != owner.record.encode()
            {
                return Err(OwnerError::Fenced);
            }
            let mut next = owner.record;
            next.key = target_epoch.key;
            next.generation = target_epoch.generation;
            let pending = Arc::new(Storage {
                root: owner.storage.root.clone(),
                directory: owner.storage.directory.try_clone()?,
                key: next.key,
                ids: owner.storage.ids,
            });
            pending.check()?;
            let ids = owner.storage.ids;
            let intent = TransferIntent {
                source_version: if held.catalog_bytes.starts_with(b"SCRETN01") {
                    1
                } else {
                    2
                },
                kind,
                old_epoch,
                target_epoch,
                objects: ids.map(|id| (id.device, id.inode)),
                old_catalog_sha256: Sha256::digest(&held.catalog_bytes).into(),
                target_catalog_sha256: Sha256::digest(&self.target_bytes).into(),
                beacon_sha256: Sha256::digest(bounded_bytes(&owner._owner, 112)?).into(),
                old_record: owner.record.encode(),
                target_record: next.encode(),
                old_catalog_len: held.catalog_bytes.len() as u64,
                target_catalog_len: self.target_bytes.len() as u64,
                entries: target.entries.len() as u16,
                source_mode,
                target_mode: target.mode,
            };
            intent.verify_reverse(&self.target_bytes)?;
            Ok((pending, intent))
        })?;
        self.pending_storage = Some(pending);
        self.intent = Some(intent);
        Ok(())
    }
    fn commit(&mut self) -> RecoveryResult<()> {
        let held = self
            .recovery
            .as_mut()
            .and_then(|value| value.held.as_mut())
            .ok_or(SupervisorError::State)?;
        if self.resuming.is_some() {
            held.funding()?.check()?;
            held.revalidate_rows()?;
        } else {
            held.revalidate()?;
            held.funding()?.can_rebind(held.epoch)?;
        }
        let intent = self.intent.as_ref().ok_or(SupervisorError::State)?;
        intent.verify_reverse(&self.target_bytes)?;
        let pending = self
            .pending_storage
            .as_ref()
            .ok_or(SupervisorError::State)?;
        let directory = &pending.directory;
        pending.check()?;
        require_eligible_namespace(directory)?;
        if self.resuming.is_none() {
            pending.absent_names(&[TRANSFER_INTENT, TRANSFER_NEXT, CATALOG_NEXT])?;
        }
        if self.resuming.is_some() {
            observe_at(
                &mut self.intent_staging,
                directory,
                TRANSFER_INTENT,
                libc::O_RDONLY,
            )?;
            if fixed_bytes::<768>(self.intent_staging.as_ref().ok_or(SupervisorError::State)?)?
                != intent.encode()?
            {
                return Err(SupervisorError::State);
            }
        } else {
            create_observed(&mut self.intent_staging, directory, TRANSFER_NEXT)?;
            let file = self.intent_staging.as_mut().ok_or(SupervisorError::State)?;
            file.write_all(&intent.encode()?)?;
            file.sync_all()?;
            #[cfg(test)]
            tests::transaction_barrier("publish_intent");
            publish_immutable(directory, TRANSFER_NEXT, TRANSFER_INTENT, file)?;
            directory.sync_all()?;
        }
        #[cfg(test)]
        tests::transaction_barrier("intent");
        // Immutable canonical intent is durable before catalog/state effects.
        if bounded_hash(
            held.catalog.as_ref().ok_or(SupervisorError::State)?,
            MAX_BYTES,
        )? == intent.target_catalog_sha256
        {
            named_is(
                directory,
                CATALOG,
                held.catalog.as_ref().ok_or(SupervisorError::State)?,
            )?;
            self.catalog_staging = Some(
                held.catalog
                    .as_ref()
                    .ok_or(SupervisorError::State)?
                    .try_clone()?,
            );
        } else {
            named_is(
                directory,
                CATALOG,
                held.catalog.as_ref().ok_or(SupervisorError::State)?,
            )?;
            if present(directory, CATALOG_NEXT)? {
                observe_at(
                    &mut self.catalog_staging,
                    directory,
                    CATALOG_NEXT,
                    libc::O_RDONLY,
                )?;
                if bounded_hash(
                    self.catalog_staging
                        .as_ref()
                        .ok_or(SupervisorError::State)?,
                    MAX_BYTES,
                )? != intent.target_catalog_sha256
                {
                    return Err(SupervisorError::State);
                }
            } else {
                create_observed(&mut self.catalog_staging, directory, CATALOG_NEXT)?;
                let file = self
                    .catalog_staging
                    .as_mut()
                    .ok_or(SupervisorError::State)?;
                file.write_all(&self.target_bytes)?;
                file.sync_all()?;
            }
            rename_exact(
                directory,
                CATALOG_NEXT,
                CATALOG,
                self.catalog_staging
                    .as_ref()
                    .ok_or(SupervisorError::State)?,
            )?;
            directory.sync_all()?;
        }
        #[cfg(test)]
        tests::transaction_barrier("catalog");
        if present(directory, INTENT)? {
            observe_at(&mut self.state_intent, directory, INTENT, libc::O_RDONLY)?;
            if fixed_bytes::<48>(self.state_intent.as_ref().ok_or(SupervisorError::State)?)?
                != state_intent_bytes(intent)
            {
                return Err(SupervisorError::State);
            }
        } else {
            create_observed(&mut self.state_intent, directory, INTENT)?;
            let file = self.state_intent.as_mut().ok_or(SupervisorError::State)?;
            file.write_all(&state_intent_bytes(intent))?;
            file.sync_all()?;
            directory.sync_all()?;
        }
        #[cfg(test)]
        tests::transaction_barrier("state_intent");
        observe_at(
            &mut self.current_state_observation,
            directory,
            STATE,
            libc::O_RDONLY,
        )?;
        let current_bytes = fixed_bytes::<176>(
            self.current_state_observation
                .as_ref()
                .ok_or(SupervisorError::State)?,
        )?;
        if current_bytes == intent.target_record {
            self.state_staging = self.current_state_observation.take();
        } else if current_bytes == intent.old_record {
            // Retain actual source record until the paired commit is durable.
            if self.old_state.is_none() {
                self.old_state = self.current_state_observation.take();
            } else {
                named_is(
                    directory,
                    STATE,
                    self.old_state.as_ref().ok_or(SupervisorError::State)?,
                )?;
                // Only a positively matched duplicate may be dropped.
                self.current_state_observation.take();
            }
            if present(directory, NEXT)? {
                observe_at(&mut self.state_staging, directory, NEXT, libc::O_RDONLY)?;
                if fixed_bytes::<176>(self.state_staging.as_ref().ok_or(SupervisorError::State)?)?
                    != intent.target_record
                {
                    return Err(SupervisorError::State);
                }
            } else {
                create_observed(&mut self.state_staging, directory, NEXT)?;
                let state = self.state_staging.as_mut().ok_or(SupervisorError::State)?;
                state.write_all(&intent.target_record)?;
                state.sync_all()?;
            }
            rename_exact(
                directory,
                NEXT,
                STATE,
                self.state_staging.as_ref().ok_or(SupervisorError::State)?,
            )?;
            directory.sync_all()?;
        } else {
            return Err(SupervisorError::State);
        }
        #[cfg(test)]
        tests::transaction_barrier("state");
        unlink_exact(
            directory,
            INTENT,
            self.state_intent.as_ref().ok_or(SupervisorError::State)?,
        )?;
        if fixed_bytes::<176>(self.state_staging.as_ref().ok_or(SupervisorError::State)?)?
            != intent.target_record
            || bounded_bytes(
                self.catalog_staging
                    .as_ref()
                    .ok_or(SupervisorError::State)?,
                MAX_BYTES,
            )? != self.target_bytes
        {
            return Err(SupervisorError::State);
        }
        let authority = held.funding()?.authority();
        authority.with_disk_owner(|owner, _| {
            if bounded_hash(&owner._owner, 112)? != intent.beacon_sha256 {
                return Err(OwnerError::Fenced);
            }
            owner.storage = pending.clone();
            owner.record = Record::decode(&intent.target_record)?;
            Ok(())
        })?;
        authority.clear_closed_transfer(intent.target_epoch)?;
        drop(authority);
        let original = held.funding.take().ok_or(SupervisorError::State)?;
        match original.rebind_closed(intent.target_epoch) {
            Ok(rebound) => held.funding = Some(rebound),
            Err(original) => {
                held.rebind_failure = Some(original);
                return Err(SupervisorError::State);
            }
        }
        held.epoch = intent.target_epoch;
        held.catalog = self.catalog_staging.take();
        held.catalog_bytes = std::mem::take(&mut self.target_bytes);
        held.revalidate()?;
        unlink_exact(
            directory,
            TRANSFER_INTENT,
            self.intent_staging.as_ref().ok_or(SupervisorError::State)?,
        )?;
        pending.check()?;
        Ok(())
    }
}
// Fixed pre-funding observations. No catalog allocation or disk mutation.
#[derive(Default)]
struct TransferFiles {
    directory: Option<File>,
    owner: Option<File>,
    holders: Option<File>,
    intent: Option<File>,
    state: Option<File>,
    catalog: Option<File>,
    staging: [Option<File>; 4],
}
pub(crate) struct ClosedTransferCurrent {
    intent: TransferIntent,
    original: TransferFiles,
    observed: std::sync::Mutex<TransferFiles>,
}
impl ClosedTransferCurrent {
    pub(crate) fn target(&self) -> Epoch {
        self.intent.target_epoch
    }
    pub(crate) fn check(&self, owner: &InstallationOwner) -> Result<()> {
        owner.storage.check()?;
        if owner.supervisor.is_some()
            || owner.fenced.load(Ordering::Acquire)
            || owner.storage.ids.map(|id| (id.device, id.inode)) != self.intent.objects
        {
            return Err(OwnerError::Fenced);
        }
        let mut observed = self.observed.lock().map_err(|_| OwnerError::Fenced)?;
        self.observe(&owner.storage.directory, &mut observed)?;
        *observed = TransferFiles::default();
        Ok(())
    }
    fn observe(&self, directory: &File, files: &mut TransferFiles) -> Result<()> {
        observe_at(
            &mut files.intent,
            directory,
            TRANSFER_INTENT,
            libc::O_RDONLY,
        )?;
        let intent = files.intent.as_ref().ok_or(OwnerError::Fenced)?;
        require_private_retained_file(intent)?;
        if object_id(intent)?
            != object_id(self.original.intent.as_ref().ok_or(OwnerError::Fenced)?)?
            || fixed_bytes::<768>(intent)? != self.intent.encode()?
        {
            return Err(OwnerError::Fenced);
        }
        observe_at(&mut files.state, directory, STATE, libc::O_RDONLY)?;
        let state = files.state.as_ref().ok_or(OwnerError::Fenced)?;
        require_private_retained_file(state)?;
        let record = fixed_bytes::<176>(state)?;
        if record != self.intent.old_record && record != self.intent.target_record {
            return Err(OwnerError::Fenced);
        }
        observe_at(&mut files.catalog, directory, CATALOG, libc::O_RDONLY)?;
        let catalog = files.catalog.as_ref().ok_or(OwnerError::Fenced)?;
        require_private_retained_file(catalog)?;
        let digest = bounded_hash(catalog, MAX_BYTES)?;
        if digest != self.intent.target_catalog_sha256
            && (digest != self.intent.old_catalog_sha256 || record != self.intent.old_record)
        {
            return Err(OwnerError::Fenced);
        }
        for (at, name) in [TRANSFER_NEXT, CATALOG_NEXT, INTENT, NEXT]
            .iter()
            .enumerate()
        {
            if present(directory, name)? {
                observe_at(&mut files.staging[at], directory, name, libc::O_RDONLY)?;
                let file = files.staging[at].as_ref().ok_or(OwnerError::Fenced)?;
                require_private_retained_file(file)?;
                match at {
                    0 => return Err(OwnerError::Fenced), // canonical intent already committed
                    1 if bounded_hash(file, MAX_BYTES)? == self.intent.target_catalog_sha256 => {}
                    2 if fixed_bytes::<48>(file)? == state_intent_bytes(&self.intent) => {}
                    3 if fixed_bytes::<176>(file)? == self.intent.target_record => {}
                    _ => return Err(OwnerError::Fenced),
                }
            }
        }
        if bounded_hash(self.original.owner.as_ref().ok_or(OwnerError::Fenced)?, 112)?
            != self.intent.beacon_sha256
        {
            return Err(OwnerError::Fenced);
        }
        Ok(())
    }
}
fn fixed_bytes<const N: usize>(file: &File) -> Result<[u8; N]> {
    if file.metadata()?.len() != N as u64 {
        return Err(OwnerError::Fenced);
    }
    let mut bytes = [0; N];
    file.read_exact_at(&mut bytes, 0)?;
    if file.metadata()?.len() != N as u64 {
        return Err(OwnerError::Fenced);
    }
    Ok(bytes)
}
fn bounded_hash(file: &File, cap: usize) -> Result<[u8; 32]> {
    let length = file.metadata()?.len();
    if length > cap as u64 {
        return Err(OwnerError::Fenced);
    }
    let mut hash = Sha256::new();
    let mut scratch = [0u8; 8192];
    let mut offset = 0;
    while offset < length {
        let take = (length - offset).min(scratch.len() as u64) as usize;
        file.read_exact_at(&mut scratch[..take], offset)?;
        hash.update(&scratch[..take]);
        offset += take as u64;
    }
    if file.metadata()?.len() != length {
        return Err(OwnerError::Fenced);
    }
    Ok(hash.finalize().into())
}
fn state_intent_bytes(intent: &TransferIntent) -> [u8; 48] {
    let mut bytes = [0; 48];
    bytes[..8].copy_from_slice(b"SCINT001");
    bytes[8..16].copy_from_slice(&intent.target_epoch.generation.to_be_bytes());
    bytes[16..].copy_from_slice(&intent.target_record[BODY_BYTES..]);
    bytes
}
pub(super) fn resume_retained_restart(
    root: &Path,
    old_key: InstallationKey,
    layout: &CanonicalRecoveryLayout,
    migration: &CompiledRetainedMigration,
) -> std::result::Result<PreparedRetainedRestart, RetainedRestartFailure> {
    let mut prepared = PreparedRetainedRestart {
        held: Some(Box::new(Transaction {
            recovery: None,
            intent: None,
            target_bytes: Vec::new(),
            pending_storage: None,
            old_state: None,
            current_state_observation: None,
            intent_staging: None,
            catalog_staging: None,
            state_intent: None,
            state_staging: None,
            acquisition: Some(Box::new(TransferFiles::default())),
            resuming: None,
        })),
    };
    let result = prepared
        .held
        .as_mut()
        .expect("armed resume")
        .resume(root, old_key, layout, migration);
    match result {
        Ok(()) => Ok(prepared),
        Err(error) => Err(RetainedRestartFailure {
            _error: error,
            _original: prepared,
        }),
    }
}
impl Transaction {
    fn resume(
        &mut self,
        root: &Path,
        old_key: InstallationKey,
        layout: &CanonicalRecoveryLayout,
        migration: &CompiledRetainedMigration,
    ) -> RecoveryResult<()> {
        if root.as_os_str().len() > 4096
            || !supported_profile(old_key.profile)
            || old_key.installation == [0; 32]
        {
            return Err(SupervisorError::Profile);
        }
        layout.check()?;
        let observations = self.acquisition.as_mut().ok_or(SupervisorError::State)?;
        observe_state(&mut observations.directory, root)?;
        let directory = observations
            .directory
            .as_ref()
            .ok_or(SupervisorError::State)?;
        require_private_retained_directory(directory)?;
        if object_id(&open_root(root)?)? != object_id(directory)? {
            return Err(SupervisorError::State);
        }
        observe_at(&mut observations.owner, directory, OWNER, libc::O_RDWR)?;
        let owner = observations.owner.as_ref().ok_or(SupervisorError::State)?;
        require_private_retained_file(owner)?;
        lock(owner, false)?;
        observe_at(&mut observations.holders, directory, HOLDERS, libc::O_RDWR)?;
        let holders = observations
            .holders
            .as_ref()
            .ok_or(SupervisorError::State)?;
        require_private_retained_file(holders)?;
        lock(holders, false)?;
        let ids = [
            object_id(directory)?,
            object_id(owner)?,
            object_id(holders)?,
        ];
        observe_at(
            &mut observations.intent,
            directory,
            TRANSFER_INTENT,
            libc::O_RDONLY,
        )?;
        let intent_file = observations.intent.as_ref().ok_or(SupervisorError::State)?;
        require_private_retained_file(intent_file)?;
        let intent = TransferIntent::decode(&fixed_bytes::<768>(intent_file)?)?;
        if intent.kind != TransferKind::Restart
            || intent.old_epoch.key != old_key
            || intent.target_epoch.key.profile != migration.target_profile
            || intent.target_epoch.key.profile != RetainedRecoveryCompatibility::current()?.profile
            || ids.map(|id| (id.device, id.inode)) != intent.objects
        {
            return Err(SupervisorError::Profile);
        }
        observe_at(&mut observations.state, directory, STATE, libc::O_RDONLY)?;
        let state = observations.state.as_ref().ok_or(SupervisorError::State)?;
        require_private_retained_file(state)?;
        let bytes = fixed_bytes::<176>(state)?;
        if bytes != intent.old_record && bytes != intent.target_record {
            return Err(SupervisorError::State);
        }
        let record = Record::decode(&bytes)?;
        // All fallible duplication happens while original acquisition remains
        // armed. No file/lock is dropped if validation or allocation unwinds.
        let owner_copy = owner.try_clone()?;
        let directory_copy = directory.try_clone()?;
        let pending_directory = directory.try_clone()?;
        let layout = layout.duplicate()?;
        let mut original = *self.acquisition.take().ok_or(SupervisorError::State)?;
        let actual_owner = original.owner.take().ok_or(SupervisorError::State)?;
        let actual_holders = original.holders.take().ok_or(SupervisorError::State)?;
        original.owner = Some(owner_copy);
        let storage = Arc::new(Storage {
            root: root.to_path_buf(),
            directory: directory_copy,
            key: record.key,
            ids,
        });
        let pending = Arc::new(Storage {
            root: root.to_path_buf(),
            directory: pending_directory,
            key: intent.target_epoch.key,
            ids,
        });
        let epoch = Epoch {
            key: record.key,
            generation: record.generation,
        };
        let owner = InstallationOwner {
            storage,
            record,
            fenced: AtomicBool::new(false),
            supervisor: None,
            _owner: actual_owner,
        };
        let proof = Arc::new(ClosedTransferCurrent {
            intent,
            original,
            observed: std::sync::Mutex::new(TransferFiles::default()),
        });
        self.resuming = Some(proof.clone());
        let authority = InstalledAuthority::from_retained_transfer(
            RetainedExclusion {
                owner,
                holders: actual_holders,
                epoch,
            },
            proof.clone(),
        );
        self.recovery = Some(RetainedRecovery {
            held: Some(Box::new(Custody {
                funding: Some(ClosedRetainedFunding::new(authority)),
                rebind_failure: None,
                layout,
                epoch,
                protection: None,
                catalog: None,
                catalog_observation: None,
                catalog_bytes: Vec::new(),
                entries: std::array::from_fn(|_| None),
                observations: std::array::from_fn(|_| Observations::default()),
                failed: false,
            })),
        });
        let held = self
            .recovery
            .as_mut()
            .and_then(|value| value.held.as_mut())
            .ok_or(SupervisorError::State)?;
        held.funding()?.fund_metadata()?; // fixed observations only before this
        held.catalog = Some(proof.observed_catalog()?);
        let actual_bytes = bounded_bytes(
            held.catalog.as_ref().ok_or(SupervisorError::State)?,
            MAX_BYTES,
        )?;
        let old = Sha256::digest(&actual_bytes).as_slice() == proof.intent.old_catalog_sha256;
        let source = decode_catalog(
            &actual_bytes,
            if old {
                proof.intent.old_epoch
            } else {
                proof.intent.target_epoch
            },
        )?;
        let target = CatalogV2 {
            epoch: proof.intent.target_epoch,
            mode: proof.intent.target_mode,
            entries: source.entries,
        };
        self.target_bytes = target.encode()?;
        proof.intent.verify_reverse(&self.target_bytes)?;
        if target
            .entries
            .iter()
            .any(|entry| entry.record.phase != 4 || !supported_profile(entry.origin.key.profile))
        {
            return Err(SupervisorError::State);
        }
        let origin = decode_beacon(&fixed_bytes::<112>(
            proof
                .original
                .owner
                .as_ref()
                .ok_or(SupervisorError::State)?,
        )?)?;
        if !target
            .entries
            .first()
            .is_some_and(|row| row.origin == origin)
            || origin.key.installation != proof.intent.old_epoch.key.installation
            || origin.generation > proof.intent.old_epoch.generation
        {
            return Err(SupervisorError::State);
        }
        let mut tenants = [[0; 32]; MAX_ENTRIES];
        for (at, row) in target.entries.iter().enumerate() {
            tenants[at] = row.record.tenant;
        }
        held.funding
            .as_mut()
            .ok_or(SupervisorError::State)?
            .fund_rows(&tenants[..target.entries.len()])?;
        // Metadata funded heap decode; row debt now funded before row-data
        // acquisition and before any rollforward mutation.
        require_eligible_namespace(&pending.directory)?;
        held.protection = Some(RuntimeStateProtection::acquire(
            &held.layout.path,
            &held.layout.directory,
        )?);
        let protection = held.protection.as_ref().ok_or(SupervisorError::State)?;
        for (at, row) in target.entries.into_iter().enumerate() {
            let observed = &mut held.observations[at];
            Custody::capture_state(&held.layout, protection, &row.record, observed)?;
            held.entries[at] = Some(PinnedEntry {
                origin: row.origin,
                record: row.record,
                directory: observed.state.take().ok_or(SupervisorError::State)?,
                marker: None,
                records: None,
                key: None,
                status: None,
            });
            let entry = held.entries[at].as_mut().ok_or(SupervisorError::State)?;
            Custody::capture_files(
                protection,
                entry.origin,
                &entry.record,
                &entry.directory,
                &mut observed.files,
            )?;
            entry.marker = observed.files.marker.take();
            entry.records = observed.files.records.take();
            entry.key = observed.files.key.take();
            entry.status = observed.files.status.take();
        }
        held.catalog_bytes = actual_bytes;
        held.funding()?.check()?;
        self.pending_storage = Some(pending);
        self.intent = Some(TransferIntent::decode(&proof.intent.encode()?)?);
        Ok(())
    }
}
impl ClosedTransferCurrent {
    fn observed_catalog(&self) -> Result<File> {
        // Caller has already funded metadata; duplicate only the last freshly
        // checked named catalog. Original observed File stays held by proof.
        let directory = self.original.directory.as_ref().ok_or(OwnerError::Fenced)?;
        let mut observed = self.observed.lock().map_err(|_| OwnerError::Fenced)?;
        self.observe(directory, &mut observed)?;
        let catalog = observed.catalog.take().ok_or(OwnerError::Fenced)?;
        *observed = TransferFiles::default();
        Ok(catalog)
    }
}

/// Metadata-only durable handoff. This is NOT an actual backend-stop receipt.
pub(super) struct StoppedRetainedCustody {
    committed: CommittedRetainedRestart,
}
impl CommittedRetainedRestart {
    pub(super) fn prepare_retained_stop(
        self,
    ) -> std::result::Result<PreparedRetainedRestart, RetainedRestartFailure> {
        let mut prepared = self.closed.into_prepared();
        let result = CompiledRetainedMigration::current().and_then(|migration| {
            prepared
                .held
                .as_mut()
                .expect("armed retained stop")
                .prepare_kind(&migration, TransferKind::RetainedStop)
        });
        match result {
            Ok(()) => Ok(prepared),
            Err(error) => Err(RetainedRestartFailure {
                _error: error,
                _original: prepared,
            }),
        }
    }
}
impl PreparedRetainedRestart {
    pub(super) fn commit_retained_stop(
        self,
    ) -> std::result::Result<StoppedRetainedCustody, RetainedRestartFailure> {
        if !self
            .held
            .as_ref()
            .and_then(|value| value.intent.as_ref())
            .is_some_and(|intent| intent.kind == TransferKind::RetainedStop)
        {
            return Err(RetainedRestartFailure {
                _error: SupervisorError::State,
                _original: self,
            });
        }
        Ok(StoppedRetainedCustody {
            committed: self.commit_closed()?,
        })
    }
}
impl StoppedRetainedCustody {
    pub(super) fn snapshot(&self) -> RecoveryResult<RetainedDebtSnapshot> {
        self.committed.snapshot()
    }
    pub(super) fn binding(&self) -> RecoveryResult<InstallationBinding> {
        self.committed.binding()
    }
    pub(super) fn prepare_restart(
        self,
        migration: &CompiledRetainedMigration,
    ) -> std::result::Result<PreparedRetainedRestart, RetainedRestartFailure> {
        self.committed.closed.prepare_restart(migration)
    }
}
#[cfg(test)]
impl PreparedRetainedRestart {
    pub(super) fn ordinary_reserve_refused(&self) -> bool {
        self.held
            .as_ref()
            .and_then(|value| value.recovery.as_ref())
            .and_then(|value| value.held.as_ref())
            .is_some_and(|held| {
                held.funding()
                    .is_ok_and(|funding| funding.test_ordinary_reserve_refused())
            })
    }
}
#[cfg(test)]
impl RetainedRestartFailure {
    pub(super) fn observed_intent_id(&self) -> (u64, u64) {
        let transaction = self._original.held.as_ref().unwrap();
        let file = transaction
            .acquisition
            .as_ref()
            .and_then(|value| value.intent.as_ref())
            .or_else(|| {
                transaction
                    .resuming
                    .as_ref()
                    .and_then(|value| value.original.intent.as_ref())
            })
            .unwrap();
        let id = object_id(file).unwrap();
        (id.device, id.inode)
    }
}
#[cfg(test)]
impl PreparedRetainedRestart {
    pub(super) fn prepared_intent_bytes(&self) -> [u8; 768] {
        self.held
            .as_ref()
            .unwrap()
            .intent
            .as_ref()
            .unwrap()
            .encode()
            .unwrap()
    }
}

#[cfg(test)]
impl CommittedRetainedRestart {
    pub(super) fn attempt_v1_publication_for_format_regression(&self) -> RecoveryResult<()> {
        let held = self
            .closed
            .recovery
            .held
            .as_ref()
            .ok_or(SupervisorError::State)?;
        let rows: Vec<_> = held
            .entries
            .iter()
            .flatten()
            .map(|entry| entry.record.clone())
            .collect();
        held.funding()?.authority().with_disk_owner(|owner, epoch| {
            owner.storage.publish_retained(&Catalog {
                epoch,
                entries: rows,
            })
        })
    }
}
#[cfg(test)]
mod size_tests {
    use super::*;
    #[test]
    fn fixed_transfer_observer_fits_assigned_metadata_envelope() {
        // Fixed Rust objects + conservatively assigned Arc/Box allocation
        // headers; no hidden catalog Vec capacity in the observer.
        let fixed = std::mem::size_of::<ClosedTransferCurrent>()
            + std::mem::size_of::<TransferFiles>()
            + std::mem::size_of::<Transaction>()
            + 8 * 128;
        assert!(fixed <= 16 * 1024, "fixed transfer metadata {fixed}");
        assert!(std::mem::size_of::<TransferIntent>() <= 1024);
        assert!(ClosedRetainedFunding::test_ledger_box_bytes() <= 24 * 1024);
    }
}
