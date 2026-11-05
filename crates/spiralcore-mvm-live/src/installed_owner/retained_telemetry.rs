//! Bounded retained ciphertext catalog, separate from disposable namespaces.
//! These private I/O leaves require original parent admission and the canonical
//! state-protection guard in their caller. Catalog data grants no permission.
use super::*;
use std::os::unix::ffi::OsStringExt;

#[path = "retained_transfer.rs"]
mod transfer;

#[path = "retained_recovery.rs"]
pub(crate) mod recovery;
pub(crate) use recovery::{CanonicalRecoveryLayout, RetainedRecoveryCompatibility};

pub(crate) const CATALOG: &str = ".retained-telemetry.bin";
pub(crate) const CATALOG_INTENT: &str = ".retained-telemetry.intent";
pub(crate) const CATALOG_NEXT: &str = ".retained-telemetry.next";
pub(crate) const TRANSFER_INTENT: &str = ".retained-telemetry.transfer";
pub(crate) const TRANSFER_NEXT: &str = ".retained-telemetry.transfer.next";
const MAGIC: &[u8; 8] = b"SCRETN01";
const DOMAIN: &[u8] = b"spiralcore/retained-telemetry-catalog/v1\0";
const HEADER: usize = 88;
const ENTRY: usize = 4352;
pub(crate) const MAX_ENTRIES: usize = 16;
const MAX_BYTES: usize = HEADER + ENTRY * MAX_ENTRIES + 32;

pub(crate) fn require_private_retained_file(file: &File) -> Result<()> {
    check_private(file, false)?;
    check_filesystem(file)
}

pub(crate) fn require_private_retained_directory(directory: &File) -> Result<()> {
    check_private(directory, true)?;
    check_filesystem(directory)
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RetainedRecord {
    pub(crate) tenant: [u8; 32],
    pub(crate) session: [u8; 32],
    pub(crate) parent: u64,
    pub(crate) serial: u64,
    pub(crate) records: u64,
    pub(crate) sidecars: u64,
    pub(crate) state: PathBuf,
    pub(crate) state_device: u64,
    pub(crate) state_inode: u64,
    pub(crate) phase: u8,
    pub(crate) record_id: (u64, u64),
    pub(crate) key_id: (u64, u64),
    pub(crate) sealed_len: u64,
    pub(crate) sealed_sha256: [u8; 32],
}
impl RetainedRecord {
    fn valid(&self) -> bool {
        let path = self.state.as_os_str().as_bytes();
        self.tenant != [0; 32]
            && self.session != [0; 32]
            && self.parent != 0
            && self.serial != 0
            && self.records == 8 << 20
            && self.sidecars == 32 << 10
            && self.state.is_absolute()
            && path.len() <= 4096
            && !path.contains(&0)
            && self
                .state
                .components()
                .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
            && self.state_device != 0
            && self.state_inode != 0
            && match self.phase {
                1 | 2 => self.record_id == (0, 0) && self.key_id == (0, 0),
                3 | 4 => {
                    self.record_id.0 != 0
                        && self.record_id.1 != 0
                        && self.key_id.0 != 0
                        && self.key_id.1 != 0
                }
                _ => false,
            }
            && if self.phase == 4 {
                self.sealed_len <= self.records
            } else {
                self.sealed_len == 0 && self.sealed_sha256 == [0; 32]
            }
    }
    fn encode_base(&self) -> Result<[u8; ENTRY]> {
        if !self.valid() {
            return Err(OwnerError::Fenced);
        }
        let mut raw = [0_u8; ENTRY];
        raw[..32].copy_from_slice(&self.tenant);
        raw[32..64].copy_from_slice(&self.session);
        for (at, value) in [
            (64, self.parent),
            (72, self.serial),
            (80, self.records),
            (88, self.sidecars),
            (96, self.state_device),
            (104, self.state_inode),
            (4224, self.record_id.0),
            (4232, self.record_id.1),
            (4240, self.key_id.0),
            (4248, self.key_id.1),
            (4256, self.sealed_len),
        ] {
            raw[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        raw[112] = self.phase;
        let path = self.state.as_os_str().as_bytes();
        raw[120..122].copy_from_slice(&(path.len() as u16).to_le_bytes());
        raw[128..128 + path.len()].copy_from_slice(path);
        raw[4264..4296].copy_from_slice(&self.sealed_sha256);
        Ok(raw)
    }
    fn decode_base(raw: &[u8]) -> Result<Self> {
        if raw.len() != ENTRY {
            return Err(OwnerError::Fenced);
        }
        let number = |at| u64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
        let len = u16::from_le_bytes(raw[120..122].try_into().unwrap()) as usize;
        if len > 4096
            || raw[113..120] != [0; 7]
            || raw[122..128] != [0; 6]
            || raw[128 + len..4224].iter().any(|byte| *byte != 0)
        {
            return Err(OwnerError::Fenced);
        }
        let value = Self {
            tenant: raw[..32].try_into().unwrap(),
            session: raw[32..64].try_into().unwrap(),
            parent: number(64),
            serial: number(72),
            records: number(80),
            sidecars: number(88),
            state_device: number(96),
            state_inode: number(104),
            phase: raw[112],
            state: PathBuf::from(std::ffi::OsString::from_vec(raw[128..128 + len].to_vec())),
            record_id: (number(4224), number(4232)),
            key_id: (number(4240), number(4248)),
            sealed_len: number(4256),
            sealed_sha256: raw[4264..4296].try_into().unwrap(),
        };
        if !value.valid() {
            return Err(OwnerError::Fenced);
        }
        Ok(value)
    }
    pub(crate) fn marker(&self, epoch: Epoch) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(224);
        bytes.extend_from_slice(b"SCRETM01");
        bytes.extend_from_slice(&epoch.key.installation);
        bytes.extend_from_slice(&epoch.key.profile);
        bytes.extend_from_slice(&epoch.generation.to_le_bytes());
        bytes.extend_from_slice(&self.tenant);
        bytes.extend_from_slice(&self.session);
        for value in [
            self.parent,
            self.serial,
            self.state_device,
            self.state_inode,
            self.records,
            self.sidecars,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let digest = Sha256::digest(&bytes);
        bytes.extend_from_slice(&digest);
        bytes
    }
}

struct Catalog {
    epoch: Epoch,
    entries: Vec<RetainedRecord>,
}
impl Catalog {
    fn matches_held(&self, expected: &[RetainedRecord]) -> bool {
        self.entries.len() == expected.len()
            && self
                .entries
                .iter()
                .all(|entry| expected.iter().any(|held| held == entry))
    }
    fn encode(&self) -> Result<Vec<u8>> {
        if self.entries.len() > MAX_ENTRIES || self.epoch.generation == 0 {
            return Err(OwnerError::Fenced);
        }
        let mut bytes = Vec::with_capacity(HEADER + ENTRY * self.entries.len() + 32);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.epoch.key.installation);
        bytes.extend_from_slice(&self.epoch.key.profile);
        bytes.extend_from_slice(&self.epoch.generation.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&[0; 6]);
        for (index, entry) in self.entries.iter().enumerate() {
            if !entry.valid()
                || self.entries[..index].iter().any(|old| {
                    old.serial == entry.serial
                        || old.session == entry.session
                        || old.state == entry.state
                        || (old.state_device, old.state_inode)
                            == (entry.state_device, entry.state_inode)
                })
            {
                return Err(OwnerError::Fenced);
            }
            let raw = entry.encode_base()?;
            bytes.extend_from_slice(&raw);
        }
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        digest.update(&bytes);
        bytes.extend_from_slice(&digest.finalize());
        Ok(bytes)
    }
    fn decode(bytes: &[u8], epoch: Epoch) -> Result<Self> {
        if bytes.len() < HEADER + 32 || bytes.len() > MAX_BYTES || &bytes[..8] != MAGIC {
            return Err(OwnerError::Fenced);
        }
        let count = u16::from_le_bytes(bytes[80..82].try_into().unwrap()) as usize;
        if count > MAX_ENTRIES
            || bytes.len() != HEADER + count * ENTRY + 32
            || bytes[8..40] != epoch.key.installation
            || bytes[40..72] != epoch.key.profile
            || bytes[72..80] != epoch.generation.to_le_bytes()
            || bytes[82..88] != [0; 6]
        {
            return Err(OwnerError::Fenced);
        }
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        digest.update(&bytes[..bytes.len() - 32]);
        if digest.finalize().as_slice() != &bytes[bytes.len() - 32..] {
            return Err(OwnerError::Fenced);
        }
        let mut entries = Vec::with_capacity(count);
        for raw in bytes[HEADER..bytes.len() - 32].chunks_exact(ENTRY) {
            // V1 keeps all 56 tail bytes reserved and zero.
            if raw[4296..].iter().any(|byte| *byte != 0) {
                return Err(OwnerError::Fenced);
            }
            entries.push(RetainedRecord::decode_base(raw)?);
        }

        let catalog = Self { epoch, entries };
        if catalog.encode()? != bytes {
            return Err(OwnerError::Fenced);
        }
        Ok(catalog)
    }
}

impl Storage {
    fn retained_catalog(&self, epoch: Epoch) -> Result<Catalog> {
        self.absent_names(&[CATALOG_INTENT, CATALOG_NEXT])?;
        if !self.name_present(CATALOG)? {
            return Ok(Catalog {
                epoch,
                entries: Vec::new(),
            });
        }
        let file = open_at(&self.directory, CATALOG, libc::O_RDONLY)?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(OwnerError::Fenced);
        }
        let mut bytes = Vec::with_capacity(MAX_BYTES + 1);
        file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
        Catalog::decode(&bytes, epoch)
    }
    fn publish_retained(&self, catalog: &Catalog) -> Result<()> {
        self.check()?;
        self.absent_names(&[CATALOG_INTENT, CATALOG_NEXT])?;
        // Normal publication cannot perform a profile/generation migration.
        // That requires the separately owned immutable transfer transaction.
        let current = self.read()?;
        if current.phase != Phase::Active
            || current.key != catalog.epoch.key
            || current.generation != catalog.epoch.generation
        {
            return Err(OwnerError::Fenced);
        }
        let bytes = catalog.encode()?;
        let mut file = open_at(
            &self.directory,
            CATALOG_INTENT,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let from = CString::new(CATALOG_INTENT).unwrap();
        let to = CString::new(CATALOG).unwrap();
        self.check()?;
        // SAFETY: fixed names under the original private installation directory.
        if unsafe {
            libc::renameat(
                self.directory.as_raw_fd(),
                from.as_ptr(),
                self.directory.as_raw_fd(),
                to.as_ptr(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        self.directory.sync_all()?;
        Ok(())
    }
}

impl InstallationOwner {
    fn latch_retained_debt(&self, epoch: Epoch) -> Result<()> {
        use std::os::unix::fs::FileExt;
        self.storage.check()?;
        if object_id(&self._owner)? != self.storage.ids[1] {
            return Err(OwnerError::Fenced);
        }
        let mut bytes = Vec::with_capacity(112);
        bytes.extend_from_slice(b"SCRETB01");
        bytes.extend_from_slice(&epoch.key.installation);
        bytes.extend_from_slice(&epoch.key.profile);
        bytes.extend_from_slice(&epoch.generation.to_le_bytes());
        let digest = Sha256::digest(&bytes);
        bytes.extend_from_slice(&digest);
        let length = self._owner.metadata()?.len();
        if length == 0 {
            // Write through the retained original EX-owner descriptor. Never
            // truncate, replace, unlink or clear this denial-only latch.
            self._owner.write_all_at(&bytes, 0)?;
        } else {
            if length != bytes.len() as u64 {
                return Err(OwnerError::Fenced);
            }
            let mut held = vec![0; bytes.len()];
            self._owner.read_exact_at(&mut held, 0)?;
            if held != bytes {
                return Err(OwnerError::Fenced);
            }
        }
        self._owner.sync_all()?;
        self.storage.check()?;
        if object_id(&self._owner)? != self.storage.ids[1] {
            return Err(OwnerError::Fenced);
        }
        Ok(())
    }
    /// Caller retains the actual original reservation and canonical protection
    /// lock throughout this transaction. Errors never prove no effects.
    pub(crate) fn protect_retained_telemetry(
        &mut self,
        epoch: Epoch,
        record: &RetainedRecord,
        directory: &File,
        held: &[RetainedRecord],
    ) -> Result<()> {
        self.active_binding(epoch)?;
        check_private(directory, true)?;
        check_filesystem(directory)?;
        let id = object_id(directory)?;
        if !record.valid()
            || record.phase != 1
            || (id.device, id.inode) != (record.state_device, record.state_inode)
        {
            return Err(OwnerError::InvalidStorage);
        }
        let mut catalog = self.storage.retained_catalog(epoch)?;
        if !catalog.matches_held(held) {
            return Err(OwnerError::Fenced);
        }
        self.latch_retained_debt(epoch)?;
        catalog.entries.push(record.clone());
        self.storage.publish_retained(&catalog)?;
        let marker = mvm_core::runtime_state_protection::RETAINED_TELEMETRY_MARKER_FILE;
        let mut file = open_at(
            directory,
            marker,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        file.write_all(&record.marker(epoch))?;
        file.sync_all()?;
        directory.sync_all()?;
        catalog.entries.last_mut().ok_or(OwnerError::Fenced)?.phase = 2;
        self.storage.publish_retained(&catalog)
    }
    /// Advance only the exact already-published original record. No deletion
    /// or budget refund is performed, including when all writers have stopped.
    pub(crate) fn update_retained_telemetry(
        &mut self,
        epoch: Epoch,
        expected: &RetainedRecord,
        next: &RetainedRecord,
        held: &[RetainedRecord],
    ) -> Result<()> {
        self.active_binding(epoch)?;
        let mut catalog = self.storage.retained_catalog(epoch)?;
        if !catalog.matches_held(held) {
            return Err(OwnerError::Fenced);
        }
        let original = catalog
            .entries
            .iter_mut()
            .find(|item| item.serial == expected.serial)
            .ok_or(OwnerError::Fenced)?;
        if original != expected || !matches!((expected.phase, next.phase), (2, 3) | (3, 4)) {
            return Err(OwnerError::Fenced);
        }
        if expected.phase == 3
            && (expected.record_id != next.record_id || expected.key_id != next.key_id)
        {
            return Err(OwnerError::Fenced);
        }
        let mut identity = next.clone();
        identity.phase = expected.phase;
        identity.record_id = expected.record_id;
        identity.key_id = expected.key_id;
        identity.sealed_len = expected.sealed_len;
        identity.sealed_sha256 = expected.sealed_sha256;
        if &identity != expected {
            return Err(OwnerError::Fenced);
        }
        *original = next.clone();
        self.storage.publish_retained(&catalog)
    }
}

#[cfg(test)]
#[path = "retained_transfer_tests.rs"]
mod transfer_tests;
