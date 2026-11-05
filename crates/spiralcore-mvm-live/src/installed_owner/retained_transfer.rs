//! Pure local-storage codecs. These values cannot construct an owner or grant.
use super::*;

const V2_MAGIC: &[u8; 8] = b"SCRETN02";
const V2_DOMAIN: &[u8] = b"spiralcore/retained-telemetry-catalog/v2\0";
const INTENT_MAGIC: &[u8; 8] = b"SCRETX01";
const INTENT_DOMAIN: &[u8] = b"spiralcore/retained-telemetry-transfer/v1\0";
const INTENT_BYTES: usize = 768;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CatalogMode {
    Active,
    StoppedRetained,
}
impl CatalogMode {
    fn byte(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::StoppedRetained => 1,
        }
    }
    fn decode(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Active),
            1 => Ok(Self::StoppedRetained),
            _ => Err(OwnerError::Fenced),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferKind {
    Restart,
    RetainedStop,
}
impl TransferKind {
    fn byte(self) -> u8 {
        match self {
            Self::Restart => 1,
            Self::RetainedStop => 2,
        }
    }
    fn decode(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Restart),
            2 => Ok(Self::RetainedStop),
            _ => Err(OwnerError::Fenced),
        }
    }
    fn target_mode(self) -> CatalogMode {
        match self {
            Self::Restart => CatalogMode::Active,
            Self::RetainedStop => CatalogMode::StoppedRetained,
        }
    }
    fn check(self, old: Epoch, target: Epoch) -> Result<()> {
        check_key(old.key)?;
        check_key(target.key)?;
        if old.generation == 0 || old.key.installation != target.key.installation {
            return Err(OwnerError::Fenced);
        }
        match self {
            Self::Restart if old.generation.checked_add(1) == Some(target.generation) => Ok(()),
            Self::RetainedStop if old == target => Ok(()),
            _ => Err(OwnerError::Fenced),
        }
    }
}
#[derive(Clone)]
pub(super) struct OriginRecord {
    pub(super) origin: Epoch,
    pub(super) record: RetainedRecord,
}
pub(super) struct CatalogV2 {
    pub(super) epoch: Epoch,
    pub(super) mode: CatalogMode,
    pub(super) entries: Vec<OriginRecord>,
}
impl CatalogV2 {
    pub(super) fn from_v1(
        bytes: &[u8],
        old: Epoch,
        target: Epoch,
        kind: TransferKind,
    ) -> Result<Self> {
        kind.check(old, target)?;
        let original = Catalog::decode(bytes, old)?;
        Ok(Self {
            epoch: target,
            mode: kind.target_mode(),
            entries: original
                .entries
                .into_iter()
                .map(|record| OriginRecord {
                    origin: old,
                    record,
                })
                .collect(),
        })
    }
    // No allocation or cloning: every producer uses the same bounded shape
    // predicate. CatalogMode is a closed enum; decode checks its numeric tag.
    fn validate_shape(&self) -> Result<()> {
        check_key(self.epoch.key)?;
        if self.epoch.generation == 0 || self.entries.len() > MAX_ENTRIES {
            return Err(OwnerError::Fenced);
        }
        for (index, entry) in self.entries.iter().enumerate() {
            check_key(entry.origin.key)?;
            if !entry.record.valid()
                || entry.origin.key.installation != self.epoch.key.installation
                || entry.origin.generation == 0
                || entry.origin.generation > self.epoch.generation
                || self.entries[..index].iter().any(|old| {
                    (old.origin == entry.origin && old.record.serial == entry.record.serial)
                        || old.record.session == entry.record.session
                        || old.record.state == entry.record.state
                        || (old.record.state_device, old.record.state_inode)
                            == (entry.record.state_device, entry.record.state_inode)
                })
            {
                return Err(OwnerError::Fenced);
            }
        }
        Ok(())
    }
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        self.validate_shape()?;
        let mut bytes = Vec::with_capacity(HEADER + ENTRY * self.entries.len() + 32);
        bytes.extend_from_slice(V2_MAGIC);
        bytes.extend_from_slice(&self.epoch.key.installation);
        bytes.extend_from_slice(&self.epoch.key.profile);
        bytes.extend_from_slice(&self.epoch.generation.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        bytes.push(self.mode.byte());
        bytes.extend_from_slice(&[0; 5]);
        for entry in &self.entries {
            let mut raw = entry.record.encode_base()?;
            raw[4296..4328].copy_from_slice(&entry.origin.key.profile);
            raw[4328..4336].copy_from_slice(&entry.origin.generation.to_le_bytes());
            bytes.extend_from_slice(&raw);
        }
        let mut hash = Sha256::new();
        hash.update(V2_DOMAIN);
        hash.update(&bytes);
        bytes.extend_from_slice(&hash.finalize());
        Ok(bytes)
    }
    pub(super) fn decode(bytes: &[u8], expected: Epoch) -> Result<Self> {
        if bytes.len() < HEADER + 32 || bytes.len() > MAX_BYTES || &bytes[..8] != V2_MAGIC {
            return Err(OwnerError::Fenced);
        }
        let count = u16::from_le_bytes(bytes[80..82].try_into().unwrap()) as usize;
        if count > MAX_ENTRIES
            || bytes.len() != HEADER + count * ENTRY + 32
            || bytes[8..40] != expected.key.installation
            || bytes[40..72] != expected.key.profile
            || bytes[72..80] != expected.generation.to_le_bytes()
            || bytes[83..88] != [0; 5]
        {
            return Err(OwnerError::Fenced);
        }
        let mode = CatalogMode::decode(bytes[82])?;
        let mut hash = Sha256::new();
        hash.update(V2_DOMAIN);
        hash.update(&bytes[..bytes.len() - 32]);
        if hash.finalize().as_slice() != &bytes[bytes.len() - 32..] {
            return Err(OwnerError::Fenced);
        }
        let mut entries = Vec::with_capacity(count);
        for raw in bytes[HEADER..bytes.len() - 32].chunks_exact(ENTRY) {
            if raw[4336..] != [0; 16] {
                return Err(OwnerError::Fenced);
            }
            entries.push(OriginRecord {
                origin: Epoch {
                    key: InstallationKey {
                        installation: expected.key.installation,
                        profile: raw[4296..4328].try_into().unwrap(),
                    },
                    generation: u64::from_le_bytes(raw[4328..4336].try_into().unwrap()),
                },
                record: RetainedRecord::decode_base(raw)?,
            });
        }
        let value = Self {
            epoch: expected,
            mode,
            entries,
        };
        if value.encode()? != bytes {
            return Err(OwnerError::Fenced);
        }
        Ok(value)
    }
    pub(super) fn reverse_source(
        &self,
        version: u8,
        old: Epoch,
        mode: CatalogMode,
    ) -> Result<Vec<u8>> {
        self.validate_shape()?;
        if old.key.installation != self.epoch.key.installation {
            return Err(OwnerError::Fenced);
        }
        match version {
            1 => {
                if mode != CatalogMode::Active
                    || self.entries.iter().any(|entry| entry.origin != old)
                {
                    return Err(OwnerError::Fenced);
                }
                #[cfg(test)]
                super::transfer_tests::observe_reverse_clone(1);
                Catalog {
                    epoch: old,
                    entries: self
                        .entries
                        .iter()
                        .map(|entry| entry.record.clone())
                        .collect(),
                }
                .encode()
            }
            2 => {
                #[cfg(test)]
                super::transfer_tests::observe_reverse_clone(2);
                Self {
                    epoch: old,
                    mode,
                    entries: self.entries.clone(),
                }
                .encode()
            }
            _ => Err(OwnerError::Fenced),
        }
    }
}

pub(super) struct TransferIntent {
    pub(super) source_version: u8,
    pub(super) kind: TransferKind,
    pub(super) old_epoch: Epoch,
    pub(super) target_epoch: Epoch,
    pub(super) objects: [(u64, u64); 3],
    pub(super) old_catalog_sha256: [u8; 32],
    pub(super) target_catalog_sha256: [u8; 32],
    pub(super) beacon_sha256: [u8; 32],
    pub(super) old_record: [u8; 176],
    pub(super) target_record: [u8; 176],
    pub(super) old_catalog_len: u64,
    pub(super) target_catalog_len: u64,
    pub(super) entries: u16,
    pub(super) source_mode: CatalogMode,
    pub(super) target_mode: CatalogMode,
}
impl TransferIntent {
    fn validate(&self) -> Result<()> {
        self.kind.check(self.old_epoch, self.target_epoch)?;
        if !matches!(self.source_version, 1 | 2)
            || (self.source_version == 1 && self.source_mode != CatalogMode::Active)
            || self.target_mode != self.kind.target_mode()
            || self.entries == 0
            || usize::from(self.entries) > MAX_ENTRIES
        {
            return Err(OwnerError::Fenced);
        }
        let length = (HEADER + ENTRY * usize::from(self.entries) + 32) as u64;
        if self.old_catalog_len != length || self.target_catalog_len != length {
            return Err(OwnerError::Fenced);
        }
        let old = Record::decode(&self.old_record)?;
        let target = Record::decode(&self.target_record)?;
        if old.phase != Phase::Active
            || target.phase != Phase::Active
            || old.key != self.old_epoch.key
            || old.generation != self.old_epoch.generation
            || target.key != self.target_epoch.key
            || target.generation != self.target_epoch.generation
        {
            return Err(OwnerError::Fenced);
        }
        for (at, (a, b)) in [
            (old.directory, target.directory),
            (old.owner, target.owner),
            (old.holders, target.holders),
        ]
        .iter()
        .enumerate()
        {
            if a != b || a.device == 0 || a.inode == 0 || (a.device, a.inode) != self.objects[at] {
                return Err(OwnerError::Fenced);
            }
        }
        Ok(())
    }
    pub(super) fn encode(&self) -> Result<[u8; INTENT_BYTES]> {
        self.validate()?;
        let mut bytes = [0; INTENT_BYTES];
        bytes[..8].copy_from_slice(INTENT_MAGIC);
        bytes[8] = self.source_version;
        bytes[9] = self.kind.byte();
        bytes[10] = 2;
        bytes[16..48].copy_from_slice(&self.old_epoch.key.installation);
        bytes[48..80].copy_from_slice(&self.old_epoch.key.profile);
        bytes[80..112].copy_from_slice(&self.target_epoch.key.profile);
        bytes[112..120].copy_from_slice(&self.old_epoch.generation.to_le_bytes());
        bytes[120..128].copy_from_slice(&self.target_epoch.generation.to_le_bytes());
        for (at, (device, inode)) in self.objects.iter().enumerate() {
            let offset = 128 + 16 * at;
            bytes[offset..offset + 8].copy_from_slice(&device.to_le_bytes());
            bytes[offset + 8..offset + 16].copy_from_slice(&inode.to_le_bytes());
        }
        bytes[176..208].copy_from_slice(&self.old_catalog_sha256);
        bytes[208..240].copy_from_slice(&self.target_catalog_sha256);
        bytes[240..272].copy_from_slice(&self.beacon_sha256);
        bytes[272..448].copy_from_slice(&self.old_record);
        bytes[448..624].copy_from_slice(&self.target_record);
        bytes[624..632].copy_from_slice(&self.old_catalog_len.to_le_bytes());
        bytes[632..640].copy_from_slice(&self.target_catalog_len.to_le_bytes());
        bytes[640..642].copy_from_slice(&self.entries.to_le_bytes());
        bytes[642] = self.source_mode.byte();
        bytes[643] = self.target_mode.byte();
        let mut hash = Sha256::new();
        hash.update(INTENT_DOMAIN);
        hash.update(&bytes[..736]);
        bytes[736..].copy_from_slice(&hash.finalize());
        Ok(bytes)
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != INTENT_BYTES
            || &bytes[..8] != INTENT_MAGIC
            || bytes[10] != 2
            || bytes[11..16] != [0; 5]
            || bytes[644..736] != [0; 92]
        {
            return Err(OwnerError::Fenced);
        }
        let mut hash = Sha256::new();
        hash.update(INTENT_DOMAIN);
        hash.update(&bytes[..736]);
        if hash.finalize().as_slice() != &bytes[736..] {
            return Err(OwnerError::Fenced);
        }
        let number = |at| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
        let installation = bytes[16..48].try_into().unwrap();
        let value = Self {
            source_version: bytes[8],
            kind: TransferKind::decode(bytes[9])?,
            old_epoch: Epoch {
                key: InstallationKey {
                    installation,
                    profile: bytes[48..80].try_into().unwrap(),
                },
                generation: number(112),
            },
            target_epoch: Epoch {
                key: InstallationKey {
                    installation,
                    profile: bytes[80..112].try_into().unwrap(),
                },
                generation: number(120),
            },
            objects: std::array::from_fn(|at| (number(128 + 16 * at), number(136 + 16 * at))),
            old_catalog_sha256: bytes[176..208].try_into().unwrap(),
            target_catalog_sha256: bytes[208..240].try_into().unwrap(),
            beacon_sha256: bytes[240..272].try_into().unwrap(),
            old_record: bytes[272..448].try_into().unwrap(),
            target_record: bytes[448..624].try_into().unwrap(),
            old_catalog_len: number(624),
            target_catalog_len: number(632),
            entries: u16::from_le_bytes(bytes[640..642].try_into().unwrap()),
            source_mode: CatalogMode::decode(bytes[642])?,
            target_mode: CatalogMode::decode(bytes[643])?,
        };
        if value.encode()?.as_slice() != bytes {
            return Err(OwnerError::Fenced);
        }
        Ok(value)
    }
    pub(super) fn verify_reverse(&self, target_bytes: &[u8]) -> Result<()> {
        self.validate()?;
        if target_bytes.len() as u64 != self.target_catalog_len
            || Sha256::digest(target_bytes).as_slice() != self.target_catalog_sha256
        {
            return Err(OwnerError::Fenced);
        }
        let target = CatalogV2::decode(target_bytes, self.target_epoch)?;
        if target.mode != self.target_mode || target.entries.len() != usize::from(self.entries) {
            return Err(OwnerError::Fenced);
        }
        let original =
            target.reverse_source(self.source_version, self.old_epoch, self.source_mode)?;
        if original.len() as u64 != self.old_catalog_len
            || Sha256::digest(&original).as_slice() != self.old_catalog_sha256
        {
            return Err(OwnerError::Fenced);
        }
        Ok(())
    }
}
