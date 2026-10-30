use crate::{
    reserve, span, Error, Limits, Manifest, NodeId, ObjectVersion, Record, RecordBackend, Result,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldStats {
    pub raw_bytes: u64,
    pub retained_record_bytes: usize,
    pub new_record_bytes: usize,
    pub new_records: usize,
}
#[derive(Debug, Clone)]
pub struct Folded {
    pub manifest: Manifest,
    pub stats: FoldStats,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportStats {
    pub new_records: usize,
    pub new_record_bytes: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadStats {
    /// Counts occurrences, including repeated visits to a deduplicated node.
    pub visits: usize,
    pub distinct_records: usize,
    /// Wire bytes over all visits, not bytes fetched from a cache/network.
    pub visited_record_bytes: usize,
}
#[derive(Debug, Clone)]
pub struct RangeVerifiedAgainstRoot {
    bytes: Vec<u8>,
    version: ObjectVersion,
    range: Range<u64>,
    remainder: Vec<Range<u64>>,
    stats: ReadStats,
}
impl RangeVerifiedAgainstRoot {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn text(&self) -> Result<&str> {
        std::str::from_utf8(&self.bytes).map_err(|_| Error::Utf8)
    }
    pub fn version(&self) -> ObjectVersion {
        self.version
    }
    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }
    pub fn unverified_remainder(&self) -> &[Range<u64>] {
        &self.remainder
    }
    pub fn whole_object_digest_verified(&self) -> bool {
        false
    }
    pub fn stats(&self) -> &ReadStats {
        &self.stats
    }
}
#[derive(Debug, Clone)]
pub struct WholeObjectDigestVerified {
    bytes: Vec<u8>,
    version: ObjectVersion,
    stats: ReadStats,
}
impl WholeObjectDigestVerified {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn text(&self) -> Result<&str> {
        std::str::from_utf8(&self.bytes).map_err(|_| Error::Utf8)
    }
    pub fn version(&self) -> ObjectVersion {
        self.version
    }
    pub fn whole_object_digest_verified(&self) -> bool {
        true
    }
    pub fn stats(&self) -> &ReadStats {
        &self.stats
    }
}
/// Immutable deduplicated records. Writes publish only after all input has validated.
#[derive(Debug)]
pub struct Store {
    records: BTreeMap<NodeId, Record>,
    limits: Limits,
    wire_bytes: usize,
}
impl RecordBackend for Store {
    fn fetch(&self, id: NodeId) -> Result<Option<Record>> {
        Ok(self.records.get(&id).cloned())
    }
}
impl Store {
    pub fn new(limits: Limits) -> Self {
        Self {
            records: BTreeMap::new(),
            limits,
            wire_bytes: 0,
        }
    }
    pub fn limits(&self) -> &Limits {
        &self.limits
    }
    pub fn record_count(&self) -> usize {
        self.records.len()
    }
    pub fn retained_record_bytes(&self) -> usize {
        self.wire_bytes
    }

    /// Encoded native record bytes for one node id, for peer serving.
    pub fn record_bytes(&self, id: NodeId) -> Option<Vec<u8>> {
        self.records.get(&id).map(Record::encode)
    }

    /// Whether the store holds the node id.
    pub fn has_record(&self, id: NodeId) -> bool {
        self.records.contains_key(&id)
    }
    fn stage(
        &self,
        pending: &mut BTreeMap<NodeId, Record>,
        added: &mut usize,
        r: Record,
    ) -> Result<()> {
        if let Some(old) = self.records.get(&r.id()).or_else(|| pending.get(&r.id())) {
            return if old == &r {
                Ok(())
            } else {
                Err(Error::IntegrityFailure)
            };
        }
        let count = self
            .records
            .len()
            .checked_add(pending.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::LimitExceeded("records"))?;
        if count > self.limits.max_records {
            return Err(Error::LimitExceeded("records"));
        }
        let bytes = added
            .checked_add(r.wire_len())
            .ok_or(Error::LimitExceeded("record bytes"))?;
        if self
            .wire_bytes
            .checked_add(bytes)
            .ok_or(Error::LimitExceeded("record bytes"))?
            > self.limits.max_record_bytes
        {
            return Err(Error::LimitExceeded("record bytes"));
        }
        // Both record-count and byte budgets precede tree-node allocation.
        pending.insert(r.id(), r);
        *added = bytes;
        Ok(())
    }
    pub fn fold(&mut self, bytes: &[u8]) -> Result<Folded> {
        let len = u64::try_from(bytes.len()).map_err(|_| Error::LimitExceeded("object bytes"))?;
        if len > self.limits.max_object_bytes {
            return Err(Error::LimitExceeded("object bytes"));
        }
        let atoms = bytes.len() / 32 + usize::from(!bytes.len().is_multiple_of(32));
        let mut visits = atoms;
        let mut parents = atoms / 3;
        while parents > 0 {
            visits = visits
                .checked_add(parents)
                .ok_or(Error::LimitExceeded("construction visits"))?;
            parents /= 3;
        }
        if visits > self.limits.max_visits {
            return Err(Error::LimitExceeded("construction visits"));
        }
        let mut pending = BTreeMap::new();
        let mut added = 0;
        let mut roots: Vec<(NodeId, u8)> = Vec::new();
        reserve(&mut roots, 128)?;
        for chunk in bytes.chunks(32) {
            let leaf = Record::leaf(chunk);
            let id = leaf.id();
            self.stage(&mut pending, &mut added, leaf)?;
            roots.push((id, 0));
            while roots.len() >= 3 {
                let n = roots.len();
                let a = roots[n - 3];
                let b = roots[n - 2];
                let c = roots[n - 1];
                if a.1 != b.1 || b.1 != c.1 {
                    break;
                }
                let scale = a.1.checked_add(1).ok_or(Error::SchemaMismatch)?;
                let parent = Record::parent(scale, [a.0, b.0, c.0])?;
                let id = parent.id();
                self.stage(&mut pending, &mut added, parent)?;
                roots.truncate(n - 3);
                roots.push((id, scale));
            }
        }
        let manifest =
            Manifest::from_parts(len, roots, Sha256::digest(bytes).into(), &self.limits)?;
        let stats = FoldStats {
            raw_bytes: len,
            retained_record_bytes: self.wire_bytes + added,
            new_record_bytes: added,
            new_records: pending.len(),
        };
        self.records.append(&mut pending);
        self.wire_bytes += added;
        Ok(Folded { manifest, stats })
    }
    pub fn export_records(&self) -> Result<Vec<u8>> {
        if self.wire_bytes > self.limits.max_record_bytes {
            return Err(Error::LimitExceeded("record bytes"));
        }
        let mut out = Vec::new();
        reserve(&mut out, self.wire_bytes)?;
        for r in self.records.values() {
            out.extend_from_slice(&r.encode());
        }
        Ok(out)
    }
    pub fn import_records(&mut self, bytes: &[u8]) -> Result<ImportStats> {
        if bytes.len() > self.limits.max_record_bytes {
            return Err(Error::LimitExceeded("record bytes"));
        }
        let mut pending = BTreeMap::new();
        let mut added = 0;
        let mut offset = 0;
        let mut visits = 0;
        while offset < bytes.len() {
            visits += 1;
            if visits > self.limits.max_visits {
                return Err(Error::LimitExceeded("import records"));
            }
            let size = if bytes[offset] == 0 { 45 } else { 49 };
            let end = offset.checked_add(size).ok_or(Error::SchemaMismatch)?;
            let r = Record::decode(bytes.get(offset..end).ok_or(Error::SchemaMismatch)?)?;
            self.stage(&mut pending, &mut added, r)?;
            offset = end;
        }
        let stats = ImportStats {
            new_records: pending.len(),
            new_record_bytes: added,
        };
        self.records.append(&mut pending);
        self.wire_bytes += added;
        Ok(stats)
    }
    pub fn range(
        &self,
        manifest: &Manifest,
        range: Range<u64>,
    ) -> Result<RangeVerifiedAgainstRoot> {
        Self::range_from(self, manifest, range, &self.limits)
    }
    /// Eager reference traversal; production lazy traversal belongs to the DAG-reader crate.
    pub fn range_from<B: RecordBackend>(
        backend: &B,
        manifest: &Manifest,
        range: Range<u64>,
        limits: &Limits,
    ) -> Result<RangeVerifiedAgainstRoot> {
        manifest.validate(limits)?;
        if range.start > range.end || range.end > manifest.byte_len() {
            return Err(Error::InvalidRange);
        }
        let length = range.end - range.start;
        if length > limits.max_returned_bytes {
            return Err(Error::LimitExceeded("returned bytes"));
        }
        let length = usize::try_from(length).map_err(|_| Error::LimitExceeded("returned bytes"))?;
        // Count intersecting occurrences by scale before reserving the output. This
        // needs no backend reads and bounds expansion even for a huge deduplicated DAG.
        let mut planned = 0u64;
        let mut base = 0u64;
        if length > 0 {
            for &(_, scale) in manifest.roots() {
                let end = base
                    .checked_add(span(scale)?)
                    .ok_or(Error::SchemaMismatch)?;
                let a = range.start.max(base);
                let b = range.end.min(end);
                if a < b {
                    for level in 0..=scale {
                        let width = span(level)?;
                        let nodes = (b - 1 - base) / width - (a - base) / width + 1;
                        planned = planned
                            .checked_add(nodes)
                            .ok_or(Error::LimitExceeded("visits"))?;
                        if planned > limits.max_visits as u64 {
                            return Err(Error::LimitExceeded("visits"));
                        }
                    }
                }
                base = end;
            }
        }
        let mut walk = Walk {
            backend,
            manifest,
            range: range.clone(),
            limits,
            out: Vec::new(),
            stats: ReadStats::default(),
            seen: BTreeSet::new(),
        };
        reserve(&mut walk.out, length)?;
        if length > 0 {
            let mut offset = 0u64;
            for &(id, scale) in manifest.roots() {
                walk.visit(id, scale, offset)?;
                offset = offset
                    .checked_add(span(scale)?)
                    .ok_or(Error::SchemaMismatch)?;
            }
        }
        if walk.out.len() != length {
            return Err(Error::IntegrityFailure);
        }
        walk.stats.distinct_records = walk.seen.len();
        let mut remainder = Vec::with_capacity(2);
        if range.start > 0 {
            remainder.push(0..range.start);
        }
        if range.end < manifest.byte_len() {
            remainder.push(range.end..manifest.byte_len());
        }
        Ok(RangeVerifiedAgainstRoot {
            bytes: walk.out,
            version: manifest.version(),
            range,
            remainder,
            stats: walk.stats,
        })
    }
    pub fn rebuild(&self, manifest: &Manifest) -> Result<WholeObjectDigestVerified> {
        Self::rebuild_from(self, manifest, &self.limits)
    }
    /// Strict reconstruction from a validated record overlay, without publishing its records.
    pub fn rebuild_from<B: RecordBackend>(
        backend: &B,
        manifest: &Manifest,
        limits: &Limits,
    ) -> Result<WholeObjectDigestVerified> {
        let result = Self::range_from(backend, manifest, 0..manifest.byte_len(), limits)?;
        let digest: [u8; 32] = Sha256::digest(result.bytes()).into();
        if digest != manifest.whole_sha256() {
            return Err(Error::IntegrityFailure);
        }
        Ok(WholeObjectDigestVerified {
            bytes: result.bytes,
            version: result.version,
            stats: result.stats,
        })
    }
    /// Currently discoverable missing frontier; descendants of absent parents are unknown.
    pub fn missing(&self, manifest: &Manifest) -> Result<Vec<NodeId>> {
        Self::missing_from(self, manifest, &self.limits, &mut 0)
    }
    pub(crate) fn missing_from<B: RecordBackend>(
        backend: &B,
        manifest: &Manifest,
        limits: &Limits,
        visits: &mut usize,
    ) -> Result<Vec<NodeId>> {
        manifest.validate(limits)?;
        let mut seen = BTreeMap::new();
        let mut missing = BTreeSet::new();
        fn visit<B: RecordBackend>(
            backend: &B,
            id: NodeId,
            scale: u8,
            seen: &mut BTreeMap<NodeId, u8>,
            missing: &mut BTreeSet<NodeId>,
            visits: &mut usize,
            limits: &Limits,
        ) -> Result<()> {
            *visits = visits
                .checked_add(1)
                .ok_or(Error::LimitExceeded("visits"))?;
            if *visits > limits.max_visits {
                return Err(Error::LimitExceeded("visits"));
            }
            if let Some(previous) = seen.get(&id) {
                return if *previous == scale {
                    Ok(())
                } else {
                    Err(Error::SchemaMismatch)
                };
            }
            if seen.len() >= limits.max_records {
                return Err(Error::LimitExceeded("closure records"));
            }
            seen.insert(id, scale);
            match backend.fetch(id)? {
                None => {
                    missing.insert(id);
                }
                Some(r) => {
                    if r.scale() != scale || r.id() != id {
                        return Err(Error::IntegrityFailure);
                    }
                    if scale > 0 {
                        for child in r.children()? {
                            visit(backend, child, scale - 1, seen, missing, visits, limits)?;
                        }
                    }
                }
            }
            Ok(())
        }
        for &(id, scale) in manifest.roots() {
            visit(backend, id, scale, &mut seen, &mut missing, visits, limits)?;
        }
        Ok(missing.into_iter().collect())
    }
    /// Validate the entire pending map before moving records into the store. The caller must
    /// separately validate the object's manifest, shape, padding and whole-object digest.
    pub(crate) fn commit_staged(&mut self, mut pending: BTreeMap<NodeId, Record>) -> Result<()> {
        let mut count = self.records.len();
        let mut bytes = self.wire_bytes;
        for (id, record) in &pending {
            if let Some(existing) = self.records.get(id) {
                if existing != record {
                    return Err(Error::IntegrityFailure);
                }
            } else {
                count = count
                    .checked_add(1)
                    .ok_or(Error::LimitExceeded("records"))?;
                bytes = bytes
                    .checked_add(record.wire_len())
                    .ok_or(Error::LimitExceeded("record bytes"))?;
                if count > self.limits.max_records {
                    return Err(Error::LimitExceeded("records"));
                }
                if bytes > self.limits.max_record_bytes {
                    return Err(Error::LimitExceeded("record bytes"));
                }
            }
        }
        self.records.append(&mut pending);
        self.wire_bytes = bytes;
        Ok(())
    }
    /// Atomically replace a snapshot under one-operation writer ownership, then sync its parent
    /// directory. To exclude stale read/modify/write cycles, acquire `StoreWriter` before loading.
    /// An error after rename may mean the new snapshot is visible but directory sync failed; callers
    /// must re-open and validate the canonical snapshot before retrying. Temporary files are never
    /// automatically promoted during recovery.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        StoreWriter::open(path)?.save(self)
    }
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<ImportStats> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        if len > self.limits.max_record_bytes as u64 {
            return Err(Error::LimitExceeded("record bytes"));
        }
        let len = usize::try_from(len).map_err(|_| Error::LimitExceeded("record bytes"))?;
        let mut bytes = Vec::new();
        reserve(&mut bytes, len)?;
        bytes.resize(len, 0);
        file.read_exact(&mut bytes)?;
        let mut extra = [0];
        if file.read(&mut extra)? != 0 {
            return Err(Error::SchemaMismatch);
        }
        self.import_records(&bytes)
    }
}
/// Exclusive cooperative ownership of a canonical snapshot path for a load/modify/save lifecycle.
/// The OS releases the advisory lock on drop or process exit. Keep the persistent `.uor-lock`
/// sidecar: unlinking it would allow two processes to lock different inodes for the same snapshot.
/// The directory must be controlled by the caller; this is not a defense against hostile filesystem
/// mutations. Recovery reads only the canonical snapshot and ignores any abandoned temporary files.
#[derive(Debug)]
pub struct StoreWriter {
    path: PathBuf,
    _lock: File,
    directory: File,
}
impl StoreWriter {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = path.file_name().ok_or(Error::SchemaMismatch)?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent)?;
        let path = parent.join(name);
        let directory = File::open(&parent)?;
        let mut lock_name = name.to_os_string();
        lock_name.push(".uor-lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join(lock_name))?;
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                Error::from(std::io::Error::from(std::io::ErrorKind::WouldBlock))
            }
            std::fs::TryLockError::Error(error) => Error::from(error),
        })?;
        Ok(Self {
            path,
            _lock: lock,
            directory,
        })
    }
    /// Read only the last canonical snapshot; absence remains a typed NotFound error.
    pub fn load(&self, limits: Limits) -> Result<Store> {
        let mut store = Store::new(limits);
        store.load(&self.path)?;
        Ok(store)
    }
    /// Stream records to an owned temporary file, sync it, publish by rename, then sync the parent.
    /// No second full serialized-store buffer is allocated. Success means all sync operations passed.
    pub fn save(&mut self, store: &Store) -> Result<()> {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let mut tmp_name = self
            .path
            .file_name()
            .ok_or(Error::SchemaMismatch)?
            .to_os_string();
        tmp_name.push(format!(
            ".uor-tmp-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let tmp = self.path.with_file_name(tmp_name);
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        let result = (|| -> Result<()> {
            for record in store.records.values() {
                file.write_all(&record.encode())?;
            }
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, &self.path)?;
            self.directory.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }
}

struct Walk<'a, B> {
    backend: &'a B,
    manifest: &'a Manifest,
    range: Range<u64>,
    limits: &'a Limits,
    out: Vec<u8>,
    stats: ReadStats,
    seen: BTreeSet<NodeId>,
}
impl<B: RecordBackend> Walk<'_, B> {
    fn visit(&mut self, id: NodeId, scale: u8, offset: u64) -> Result<()> {
        let end = offset
            .checked_add(span(scale)?)
            .ok_or(Error::SchemaMismatch)?;
        if offset >= self.range.end || end <= self.range.start {
            return Ok(());
        }
        self.stats.visits = self
            .stats
            .visits
            .checked_add(1)
            .ok_or(Error::LimitExceeded("visits"))?;
        if self.stats.visits > self.limits.max_visits {
            return Err(Error::LimitExceeded("visits"));
        }
        let r = self.backend.fetch(id)?.ok_or(Error::Missing(id))?;
        if r.id() != id || r.scale() != scale {
            return Err(Error::IntegrityFailure);
        }
        self.stats.visited_record_bytes = self
            .stats
            .visited_record_bytes
            .checked_add(r.wire_len())
            .ok_or(Error::LimitExceeded("visited bytes"))?;
        self.seen.insert(id);
        if scale == 0 {
            if end > self.manifest.byte_len() {
                let used = usize::try_from(
                    self.manifest
                        .byte_len()
                        .checked_sub(offset)
                        .ok_or(Error::SchemaMismatch)?,
                )
                .map_err(|_| Error::SchemaMismatch)?;
                if r.payload()[used..].iter().any(|b| *b != 0) {
                    return Err(Error::IntegrityFailure);
                }
            }
            let start = (self.range.start.max(offset) - offset) as usize;
            let stop = (self.range.end.min(end) - offset) as usize;
            self.out.extend_from_slice(&r.payload()[start..stop]);
        } else {
            let child_span = span(scale - 1)?;
            let mut child_offset = offset;
            for child in r.children()? {
                self.visit(child, scale - 1, child_offset)?;
                child_offset = child_offset
                    .checked_add(child_span)
                    .ok_or(Error::SchemaMismatch)?;
            }
        }
        Ok(())
    }
}
