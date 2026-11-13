//! Stateful application endpoint. The caller supplies authenticated context and current authority.
//! This module does not authenticate sockets, implement a mailbox envelope, or claim FIPS validation.
use crate::{
    dag::{self, Node},
    hash, hex, object_id,
    prepared::{self, VerifiedGraph},
    protocol::{Frame, Mode, Offer, Reader},
    Error, Id, Result, MAX_CHUNK, MAX_OBJECT,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
};
mod storage;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use storage::owned::{OwnedLaneNamespaces, OwnedStoreLease, OwnedStorePolicy};
use storage::{private_dir, private_file};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use {std::os::fd::AsFd, storage::owned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub tenant: String,
    pub domain: String,
    pub sender: String,
    pub recipient: String,
    pub mailbox: String,
    pub capability: String,
    pub authorization_scope: String,
    pub policy_epoch: u64,
    pub execution_epoch: u64,
}
impl Context {
    pub fn namespace(&self) -> Result<Id> {
        let mut b = Vec::new();
        for s in [
            &self.tenant,
            &self.domain,
            &self.sender,
            &self.recipient,
            &self.mailbox,
            &self.capability,
            &self.authorization_scope,
        ] {
            if s.is_empty() || s.len() > 256 {
                return Err(Error::Malformed);
            }
            b.extend_from_slice(&(s.len() as u16).to_le_bytes());
            b.extend_from_slice(s.as_bytes());
        }
        b.extend_from_slice(&self.policy_epoch.to_le_bytes());
        b.extend_from_slice(&self.execution_epoch.to_le_bytes());
        Ok(hash(b"spiralcore.object.namespace.v1\0", &[&b]))
    }
}
/// Implement this using current authenticated mailbox/application policy, never payload assertions.
pub trait Authority {
    fn authorize(&self, context: &Context) -> Result<()>;
}

/// Local receiver choice for optional graph construction after a cold publication.
/// Explicit DAG transfers remain fully verified under either policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColdGraphPolicy {
    EagerBounded,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub max_nodes: usize,
    pub max_active: usize,
    pub resident_bytes: u64,
    pub namespace_bytes: u64,
    pub cache_bytes: u64,
    /// Verified graph retention is charged inside resident_bytes, never in addition to it.
    pub graph_cache_bytes: u64,
    pub graph_cache_entries: usize,
    pub max_states: usize,
    pub max_publications: usize,
    pub idle_seconds: u64,
    pub retention_seconds: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_nodes: 262144,
            max_active: 4,
            resident_bytes: 512 << 20,
            namespace_bytes: 512 << 20,
            cache_bytes: 1 << 30,
            graph_cache_bytes: 64 << 20,
            graph_cache_entries: 32,
            max_states: 128,
            max_publications: 4096,
            idle_seconds: 30,
            retention_seconds: 300,
        }
    }
}
struct Attempt {
    offer: Offer,
    next: u64,
    ledger: BTreeMap<u64, (Id, Frame)>,
    bytes: Vec<u8>,
    nodes: BTreeMap<Id, Node>,
    accepted: u64,
    state: u8,
    error: Option<Error>,
    last: u64,
    created: u64,
    reserved: u64,
    repair: u8,
    base_pinned: bool,
    staged_nodes: usize,
    base_graph: Option<Arc<VerifiedGraph>>,
    base_bytes: Vec<u8>,
    sparse_operations: usize,
    prefix_encoded: Vec<u8>,
    prefix_chunks: usize,
    runtime_ready: bool,
    codec: Option<CodecHandle>,
}
impl Attempt {
    fn node(&self, id: &Id) -> Option<&Node> {
        self.nodes
            .get(id)
            .or_else(|| self.base_graph.as_ref().and_then(|g| g.node(id)))
    }
    fn clear(&mut self) {
        self.bytes = Vec::new();
        self.nodes = BTreeMap::new();
        self.base_graph = None;
        self.base_bytes = Vec::new();
        self.prefix_encoded = Vec::new();
        self.runtime_ready = false;
        self.codec = None;
        self.reserved = 0;
    }
    fn reject(&mut self, e: Error) {
        self.state = 3;
        self.error = Some(e);
        self.clear();
    }
}
enum StoreRoot {
    Path {
        path: PathBuf,
        _owner: File,
    },
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Owned(OwnedStoreLease),
}

impl StoreRoot {
    fn path(&self) -> Result<&Path> {
        match self {
            Self::Path { path, .. } => Ok(path),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Self::Owned(_) => Err(Error::Storage),
        }
    }
}

#[derive(Clone)]
enum Locator {
    Path(PathBuf),
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Owned(OwnedStoreLease, Id),
}

pub struct Endpoint<A: Authority> {
    resources: crate::resource_profile::ResourceProfile,
    root: StoreRoot,
    limits: Limits,
    cold_graph_policy: ColdGraphPolicy,
    authority: A,
    attempts: BTreeMap<(Id, Id), Attempt>,
    graphs: BTreeMap<(Id, Id), (Arc<VerifiedGraph>, u64)>,
    graph_stats: GraphCacheStats,
    codec_cache_enabled: bool,
    codecs: BTreeMap<(Id, u8, Id), (CodecHandle, u64)>,
    codec_stats: CodecCacheStats,
}
type CodecHandle = Arc<Mutex<crate::fitted_transfer::PreparedDecoder>>;
#[derive(Debug, Clone, Copy, Default)]
pub struct CodecCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub preparations: u64,
    pub evictions: u64,
    /// Conservative admission charge, not process RSS.
    pub resident_bytes: u64,
    pub entries: usize,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub resident_bytes: u64,
    pub entries: usize,
}
#[derive(Clone)]
struct Publication<L = Locator> {
    path: L,
    len: u64,
    object: Id,
    manifest: Option<Id>,
}
// An owned read holds the original namespace alongside the checked file until
// the operation verifies the named file and namespace one last time.
struct OpenPublication {
    meta: Publication,
    file: File,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    directory: Option<File>,
}
impl OpenPublication {
    fn check_named(&self) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Locator::Owned(lease, ns) = &self.meta.path {
            let dir = self.directory.as_ref().ok_or(Error::Storage)?;
            owned::check_file_at(
                dir.as_fd(),
                object_name(&self.meta.object).as_ref(),
                &self.file,
            )?;
            lease.check_namespace(*ns, dir.as_fd())?;
        }
        Ok(())
    }
}
impl<L> Publication<L> {
    fn disk_len(&self) -> Result<u64> {
        self.len
            .checked_add(if self.manifest.is_some() { 81 } else { 49 })
            .ok_or(Error::Limit)
    }
}
static TEMP: AtomicU64 = AtomicU64::new(0);

/// A victim selected by the ordered owner from its bounded verified index.
pub(crate) struct OrderedEviction {
    pub(crate) id: Id,
    pub(crate) len: u64,
    pub(crate) disk: u64,
}

impl<A: Authority> Endpoint<A> {
    pub(crate) fn authorize(&self, context: &Context) -> Result<()> {
        self.authority.authorize(context)?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            lease.check_lane(context.namespace()?)?;
        }
        Ok(())
    }
    pub fn open(root: &Path, limits: Limits, authority: A) -> Result<Self> {
        Self::open_with_policy(root, limits, authority, ColdGraphPolicy::EagerBounded)
    }
    pub fn open_with_policy(
        root: &Path,
        limits: Limits,
        authority: A,
        cold_graph_policy: ColdGraphPolicy,
    ) -> Result<Self> {
        Self::open_config(
            root,
            limits,
            authority,
            cold_graph_policy,
            crate::resource_profile::ResourceProfile::General,
        )
    }
    pub fn open_with_resource_profile(
        root: &Path,
        limits: Limits,
        authority: A,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Result<Self> {
        let policy = if resources == crate::resource_profile::ResourceProfile::SmallV1 {
            ColdGraphPolicy::Disabled
        } else {
            ColdGraphPolicy::EagerBounded
        };
        Self::open_config(root, limits, authority, policy, resources)
    }
    fn open_config(
        root: &Path,
        limits: Limits,
        authority: A,
        cold_graph_policy: ColdGraphPolicy,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Result<Self> {
        Self::validate_limits(&limits, resources)?;
        private_dir(root)?;
        let marker = root.join(".spiralcore-profile-v1");
        let initialized = marker.exists();
        if initialized {
            storage::check_profile(&marker)?;
        } else if fs::read_dir(root)?.next().is_some() {
            return Err(Error::Storage);
        }
        let owner_path = root.join(".owner");
        if fs::symlink_metadata(&owner_path).is_ok() {
            private_file(&owner_path)?;
        }
        let owner = storage::open_owner(&owner_path)?;
        storage::lock_owner(&owner, &owner_path, initialized)?;
        if !initialized {
            storage::initialize_profile(root, storage::PROFILE_CONTENT)?;
        }
        // Only our reserved staging name is collected, with exclusive root ownership.
        for dir in fs::read_dir(root)? {
            let dir = dir?;
            if !dir.file_type()?.is_dir()
                || !canonical_namespace(&dir.file_name().to_string_lossy())
            {
                continue;
            }
            private_dir(&dir.path())?;
            for f in fs::read_dir(dir.path())? {
                let f = f?;
                if f.file_type()?.is_file() && owned_stage(&f.file_name().to_string_lossy()) {
                    private_file(&f.path())?;
                    storage::remove_stage(&f.path())?;
                }
            }
        }
        Ok(Self::from_store(
            StoreRoot::Path {
                path: root.to_path_buf(),
                _owner: owner,
            },
            limits,
            authority,
            cold_graph_policy,
            resources,
        ))
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn open_at(
        lease: &OwnedStoreLease,
        limits: Limits,
        authority: A,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Result<Self> {
        Self::validate_limits(&limits, resources)?;
        lease.claim(&limits, resources)?;
        Ok(Self::from_store(
            StoreRoot::Owned(lease.clone()),
            limits,
            authority,
            ColdGraphPolicy::Disabled,
            resources,
        ))
    }

    fn validate_limits(
        limits: &Limits,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Result<()> {
        resources.check_limits(limits)?;
        if limits.max_active == 0
            || limits.max_active > 4
            || limits.max_nodes == 0
            || limits.max_states < limits.max_active
            || limits.graph_cache_bytes > 64 << 20
            || limits.graph_cache_entries > 32
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn from_store(
        root: StoreRoot,
        limits: Limits,
        authority: A,
        cold_graph_policy: ColdGraphPolicy,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Self {
        Self {
            resources,
            root,
            limits,
            cold_graph_policy,
            authority,
            attempts: BTreeMap::new(),
            graphs: BTreeMap::new(),
            graph_stats: GraphCacheStats::default(),
            codec_cache_enabled: false,
            codecs: BTreeMap::new(),
            codec_stats: CodecCacheStats::default(),
        }
    }
    fn path(&self, ns: Id, id: Id) -> Result<PathBuf> {
        Ok(self
            .root
            .path()?
            .join(hex(&ns))
            .join(format!("{}.object", hex(&id))))
    }
    fn charged(&self) -> u64 {
        self.graph_cache_stats().resident_bytes
            + self.codec_cache_stats().resident_bytes
            + self
                .attempts
                .values()
                .map(|s| {
                    if s.state == 0 {
                        s.reserved
                    } else {
                        history_charge(s)
                    }
                })
                .sum::<u64>()
    }
    pub fn graph_cache_stats(&self) -> GraphCacheStats {
        GraphCacheStats {
            resident_bytes: self.graphs.values().map(|(g, _)| g.resident_charge()).sum(),
            entries: self.graphs.len(),
            ..self.graph_stats
        }
    }
    /// Opt-in retention of validated fitted decoder state. Changes are allowed
    /// only with no active transfer; disabling drops all unpinned state.
    pub fn set_codec_cache_enabled(&mut self, enabled: bool) -> Result<()> {
        if enabled && self.resources == crate::resource_profile::ResourceProfile::SmallV1 {
            return Err(Error::Unsupported);
        }
        if self.attempts.values().any(|s| s.state == 0) {
            return Err(Error::Sequence);
        }
        self.codec_cache_enabled = enabled;
        if !enabled {
            self.codec_stats.evictions += self.codecs.len() as u64;
            self.codecs.clear();
        }
        Ok(())
    }
    pub fn codec_cache_stats(&self) -> CodecCacheStats {
        CodecCacheStats {
            resident_bytes: self.codecs.len() as u64 * crate::fitted_transfer::CODEC_CHARGE,
            entries: self.codecs.len(),
            ..self.codec_stats
        }
    }
    fn evict_codec(&mut self) -> bool {
        let victim = self
            .codecs
            .iter()
            .filter(|(_, (p, _))| Arc::strong_count(p) == 1)
            .min_by_key(|(_, (_, age))| *age)
            .map(|(key, _)| *key);
        if let Some(key) = victim {
            self.codecs.remove(&key);
            self.codec_stats.evictions += 1;
            true
        } else {
            false
        }
    }
    fn prepare_codec(
        &mut self,
        ns: Id,
        offer: &Offer,
        verified_artifact: &[u8],
        extra: u64,
        now: u64,
    ) -> Result<Option<CodecHandle>> {
        if !self.codec_cache_enabled || !crate::fitted_transfer::is_buffered(offer.mode) {
            return Ok(None);
        }
        let artifact = if offer.mode == Mode::ZstdWholeV1 {
            [0; 32]
        } else {
            offer.base.ok_or(Error::Malformed)?
        };
        let key = (ns, offer.mode as u8, artifact);
        if let Some((p, age)) = self.codecs.get_mut(&key) {
            *age = now;
            self.codec_stats.hits += 1;
            return Ok(Some(Arc::clone(p)));
        }
        self.codec_stats.misses += 1;
        while self.codecs.len() >= 8 {
            if !self.evict_codec() {
                return Ok(None);
            }
        }
        // Charge the retained entry BEFORE construction, in addition to the
        // active attempt's conservative scratch charge. Under pressure reuse
        // is optional; the already-admitted unprepared decoder still works.
        if self
            .reserve_with_eviction(
                extra
                    .checked_add(crate::fitted_transfer::CODEC_CHARGE)
                    .ok_or(Error::Limit)?,
            )
            .is_err()
        {
            return Ok(None);
        }
        let decoder = Arc::new(Mutex::new(crate::fitted_transfer::PreparedDecoder::new(
            offer.mode,
            verified_artifact,
        )?));
        self.codecs.insert(key, (Arc::clone(&decoder), now));
        self.codec_stats.preparations += 1;
        Ok(Some(decoder))
    }
    fn evict_graph(&mut self) -> bool {
        let victim = self
            .graphs
            .iter()
            .filter(|(_, (g, _))| Arc::strong_count(g) == 1)
            .min_by_key(|(_, (_, age))| *age)
            .map(|(key, _)| *key);
        if let Some(key) = victim {
            self.graphs.remove(&key);
            self.graph_stats.evictions += 1;
            true
        } else {
            false
        }
    }
    fn reserve_with_eviction(&mut self, n: u64) -> Result<()> {
        while self.reserve(n).is_err() {
            if !self.evict_graph() && !self.evict_codec() {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
    fn retain_graph(&mut self, ns: Id, graph: Arc<VerifiedGraph>, extra: u64, age: u64) {
        let key = (ns, graph.manifest().id());
        if self.graphs.contains_key(&key)
            || graph.resident_charge() > self.limits.graph_cache_bytes
            || self.limits.graph_cache_entries == 0
        {
            return;
        }
        while self.graphs.len() >= self.limits.graph_cache_entries
            || self.graph_cache_stats().resident_bytes + graph.resident_charge()
                > self.limits.graph_cache_bytes
            || self.reserve(extra + graph.resident_charge()).is_err()
        {
            if !self.evict_graph() {
                return;
            }
        }
        self.graphs.insert(key, (graph, age));
    }
    fn reserve(&self, n: u64) -> Result<()> {
        if self.charged().checked_add(n).ok_or(Error::Limit)? > self.limits.resident_bytes {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn sweep(&mut self, now: u64) {
        for s in self.attempts.values_mut() {
            if s.state == 0
                && (now.saturating_sub(s.last) > self.limits.idle_seconds
                    || now.saturating_sub(s.created) > self.limits.retention_seconds)
            {
                s.reject(Error::Expired);
            }
        }
        self.attempts.retain(|_, s| {
            s.state == 0 || now.saturating_sub(s.last) <= self.limits.retention_seconds
        });
    }
    pub fn handle_bytes(&mut self, c: &Context, b: &[u8], now: u64) -> Result<Vec<u8>> {
        self.handle(c, &Frame::decode(b)?, now)?.encode()
    }
    pub fn handle(&mut self, c: &Context, f: &Frame, now: u64) -> Result<Frame> {
        self.handle_with_evictions(c, f, now, &[])
    }
    pub(crate) fn handle_with_evictions(
        &mut self,
        c: &Context,
        f: &Frame,
        now: u64,
        evictions: &[OrderedEviction],
    ) -> Result<Frame> {
        self.authorize(c)?;
        let ns = c.namespace()?;
        if self.resources == crate::resource_profile::ResourceProfile::SmallV1
            && matches!(f.kind, 1 | 4)
        {
            let body = if f.kind == 4 {
                f.body.get(32..).ok_or(Error::Malformed)?
            } else {
                &f.body
            };
            self.resources.check_offer(&Offer::decode(body)?)?;
        }
        let wire = f.encode()?;
        if !matches!(f.kind, 1..=6) {
            return Err(Error::Unsupported);
        }
        self.sweep(now);
        let key = (ns, f.transfer);
        if !evictions.is_empty() {
            self.ordered_bounds()?;
            if f.kind != 1 || f.seq != 0 || self.attempts.contains_key(&key) {
                return Err(Error::Sequence);
            }
        }
        let digest = hash(b"spiralcore.object.request.v1\0", &[&wire]);
        if !self.attempts.contains_key(&key) {
            if f.seq != 0 {
                return Err(Error::Unknown);
            }
            if f.kind == 4 {
                let mut r = Reader::new(&f.body);
                let expected = r.id()?;
                let offer = Offer::decode(r.remaining())?;
                if offer.id()? != expected {
                    return Err(Error::Integrity);
                }
                let p = self.publication(ns, offer.object)?;
                if self.resources == crate::resource_profile::ResourceProfile::SmallV1
                    && p.len != offer.len
                {
                    return Err(Error::Integrity);
                }
                self.reserve_with_eviction(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
                let bytes = read_publication_for_receipt(&p)?;
                if bytes.len() as u64 != offer.len {
                    return Err(Error::Integrity);
                }
                self.authorize(c)?;
                return Ok(f.reply(103, offer.object.to_vec()));
            }
            if f.kind != 1 {
                return Err(Error::Unknown);
            }
            let reply = self.begin(c, ns, f, digest, now, evictions)?;
            self.apply_ordered_evictions(c, evictions)?;
            return Ok(reply);
        }
        let mut s = self.attempts.remove(&key).ok_or(Error::Unknown)?;
        let result = (|| {
            if let Some(e) = s.error {
                return Err(e);
            }
            if let Some((old, reply)) = s.ledger.get(&f.seq) {
                if old == &digest {
                    return Ok(reply.clone());
                }
                s.reject(Error::Sequence);
                return Err(Error::Sequence);
            }
            if f.seq != s.next {
                return Err(Error::Sequence);
            }
            if s.ledger.len() >= 4096 {
                return Err(Error::Limit);
            }
            // This attempt is temporarily removed from the map. Active reservations include
            // a 2 MiB ledger/scratch allowance; terminal attempts retain only their actual ledger.
            if s.state == 0 {
                if history_charge(&s) + crate::MAX_FRAME as u64 + 256 > 2 << 20 {
                    return Err(Error::Limit);
                }
                self.reserve(s.reserved)?;
            } else {
                self.reserve(history_charge(&s) + crate::MAX_FRAME as u64 + 256)?;
            }
            let reply = self.apply(c, ns, f, &mut s)?;
            s.next = s.next.checked_add(1).ok_or(Error::Limit)?;
            s.last = now;
            s.ledger.insert(f.seq, (digest, reply.clone()));
            Ok(reply)
        })();
        if let Err(e) = result {
            if matches!(
                e,
                Error::Malformed
                    | Error::Integrity
                    | Error::Limit
                    | Error::Unsupported
                    | Error::Missing
            ) {
                s.reject(e);
            }
        }
        self.attempts.insert(key, s);
        result
    }
    fn begin(
        &mut self,
        c: &Context,
        ns: Id,
        f: &Frame,
        digest: Id,
        now: u64,
        evictions: &[OrderedEviction],
    ) -> Result<Frame> {
        let offer = Offer::decode(&f.body)?;
        if self.attempts.len() >= self.limits.max_states
            || self.attempts.values().filter(|s| s.state == 0).count() >= self.limits.max_active
        {
            return Err(Error::Limit);
        }
        let existing = self.publication(ns, offer.object);
        if let Ok(p) = &existing {
            if p.len != offer.len {
                return Err(Error::Integrity);
            }
        }
        if matches!(existing, Err(Error::Unknown)) {
            self.admit_disk(ns, &offer, evictions)?;
        } else if let Err(e) = existing {
            return Err(e);
        } else if !evictions.is_empty() {
            return Err(Error::Integrity);
        }
        // Charge conservative peak allocations, including construction/indexes and two output buffers.
        let mut charge = offer.len.checked_mul(2).ok_or(Error::Limit)?;
        let warming = if matches!(
            offer.mode,
            Mode::SparseV1
                | Mode::ZstdPrefixV1
                | Mode::RuntimeV1
                | Mode::SchemaV1
                | Mode::ZstdDictionaryV1
                | Mode::SchemaZstdV1
                | Mode::ZstdWholeV1
        ) || (matches!(offer.mode, Mode::Raw | Mode::Zstd)
            && self.cold_graph_policy == ColdGraphPolicy::Disabled)
            || existing.is_ok()
        {
            0
        } else {
            dag::work_bound(offer.len).min(self.limits.max_nodes) as u64
        };
        // DAG admission covers staging, a target closure set and a detached retained
        // index simultaneously. Raw warming constructs only one graph.
        let per_node = if offer.mode == Mode::Dag { 1152 } else { 512 };
        charge = charge
            .checked_add(warming.checked_mul(per_node).ok_or(Error::Limit)?)
            .and_then(|x| x.checked_add(2 << 20))
            .ok_or(Error::Limit)?;
        if offer.mode == Mode::ZstdPrefixV1 && existing.is_err() {
            charge = charge
                .checked_add(crate::zstd_prefix::encoded_cap(offer.len as usize)? as u64)
                .and_then(|n| n.checked_add(self.resources.decoder_charge()))
                .ok_or(Error::Limit)?;
        }
        if crate::fitted_transfer::is_buffered(offer.mode) && existing.is_err() {
            charge = charge
                .checked_add(
                    crate::fitted_transfer::encoded_cap(offer.mode, offer.len as usize)? as u64,
                )
                .and_then(|n| n.checked_add(self.resources.decoder_charge()))
                .ok_or(Error::Limit)?;
        }
        if offer.mode == Mode::SchemaZstdV1 && existing.is_err() {
            // Encoded input, expanded SCVAL001 intermediate, final output and
            // prepared-template scratch are independent admitted allocations.
            charge = charge
                .checked_add(crate::schema::encoded_cap(offer.len as usize)? as u64)
                .and_then(|n| n.checked_add(1 << 20))
                .ok_or(Error::Limit)?;
        }
        if offer.mode == Mode::RuntimeV1 && existing.is_err() {
            // A prediction may be larger than the target after a truncation.
            // Reserve the bounded generator's capacity/growth separately from
            // target output, checkpoint bytes and the existing scratch allowance.
            charge = charge
                .checked_add(2 * self.resources.max_prediction_len() as u64)
                .ok_or(Error::Limit)?;
        }
        let cached = if offer.mode == Mode::Dag && existing.is_err() {
            offer
                .base
                .and_then(|mid| self.graphs.get_mut(&(ns, mid)))
                .map(|(g, age)| {
                    *age = now;
                    Arc::clone(g)
                })
        } else {
            None
        };
        let base = if let Some(mid) = offer.base.filter(|_| existing.is_err()) {
            let p = if matches!(
                offer.mode,
                Mode::SparseV1
                    | Mode::ZstdPrefixV1
                    | Mode::RuntimeV1
                    | Mode::SchemaV1
                    | Mode::ZstdDictionaryV1
                    | Mode::SchemaZstdV1
            ) {
                self.publication(ns, mid).map_err(|e| {
                    if e == Error::Unknown {
                        Error::Missing
                    } else {
                        e
                    }
                })?
            } else if let Some(g) = &cached {
                let p = self.publication(ns, g.manifest().object).map_err(|e| {
                    if e == Error::Unknown {
                        Error::Missing
                    } else {
                        e
                    }
                })?;
                if p.manifest != Some(mid) {
                    return Err(Error::Integrity);
                }
                p
            } else {
                self.find_base(ns, mid)?
            };
            Some(p)
        } else {
            None
        };
        if let Some(p) = &base {
            if self.resources == crate::resource_profile::ResourceProfile::SmallV1
                && p.len > self.resources.max_artifact_len() as u64
            {
                return Err(Error::Limit);
            }
            if crate::fitted_transfer::is_fitted(offer.mode) && p.len > 64 * 1024 {
                return Err(Error::Limit);
            }
            if offer.mode == Mode::RuntimeV1 && p.len > crate::shared_runtime::MAX_CHECKPOINT as u64
            {
                return Err(Error::Limit);
            }
            if offer.mode == Mode::ZstdPrefixV1
                && p.len > crate::zstd_prefix::MAX_PREFIX_OBJECT as u64
            {
                return Err(Error::Limit);
            }
            let base_nodes = if matches!(
                offer.mode,
                Mode::SparseV1
                    | Mode::ZstdPrefixV1
                    | Mode::RuntimeV1
                    | Mode::SchemaV1
                    | Mode::ZstdDictionaryV1
                    | Mode::SchemaZstdV1
            ) || cached.is_some()
            {
                0
            } else {
                dag::work_bound(p.len).min(self.limits.max_nodes) as u64
            };
            charge = charge
                .checked_add(p.len * 2 + base_nodes * 512)
                .ok_or(Error::Limit)?;
        }
        self.reserve_with_eviction(charge)?;
        let mut s = Attempt {
            offer: offer.clone(),
            next: 1,
            ledger: BTreeMap::new(),
            bytes: Vec::new(),
            nodes: BTreeMap::new(),
            accepted: 0,
            state: 0,
            error: None,
            last: now,
            created: now,
            reserved: charge,
            repair: 0,
            base_pinned: false,
            staged_nodes: 0,
            base_graph: None,
            base_bytes: Vec::new(),
            sparse_operations: 0,
            prefix_encoded: Vec::new(),
            prefix_chunks: 0,
            runtime_ready: false,
            codec: None,
        };
        let reply = match existing {
            Ok(p) => {
                let data = read_publication_for_receipt(&p)?;
                if data.len() as u64 != offer.len {
                    return Err(Error::Integrity);
                }
                s.state = 1;
                s.clear();
                f.reply(103, offer.object.to_vec())
            }
            Err(Error::Unknown) => {
                if let Some(p) = base {
                    let bytes = read_publication(&p)?;
                    if matches!(
                        offer.mode,
                        Mode::SparseV1
                            | Mode::ZstdPrefixV1
                            | Mode::RuntimeV1
                            | Mode::SchemaV1
                            | Mode::ZstdDictionaryV1
                            | Mode::SchemaZstdV1
                    ) {
                        s.base_bytes = bytes;
                    } else {
                        let g = if let Some(g) = cached {
                            self.graph_stats.hits += 1;
                            g
                        } else {
                            self.graph_stats.misses += 1;
                            Arc::new(VerifiedGraph::fold(&bytes, self.limits.max_nodes)?)
                        };
                        if Some(g.manifest().id()) != offer.base {
                            return Err(Error::Integrity);
                        }
                        s.base_graph = Some(g);
                    }
                    s.base_pinned = true;
                }
                s.codec = self.prepare_codec(ns, &offer, &s.base_bytes, s.reserved, now)?;
                if offer.mode == Mode::ZstdPrefixV1 {
                    s.prefix_encoded
                        .try_reserve_exact(crate::zstd_prefix::encoded_cap(offer.len as usize)?)
                        .map_err(|_| Error::Limit)?;
                } else if crate::fitted_transfer::is_buffered(offer.mode) {
                    s.prefix_encoded
                        .try_reserve_exact(crate::fitted_transfer::encoded_cap(
                            offer.mode,
                            offer.len as usize,
                        )?)
                        .map_err(|_| Error::Limit)?;
                } else if offer.mode != Mode::Dag {
                    s.bytes
                        .try_reserve_exact(offer.len as usize)
                        .map_err(|_| Error::Limit)?;
                }
                let mut b = vec![offer.mode as u8, u8::from(s.base_pinned)];
                b.extend_from_slice(&1u64.to_le_bytes());
                f.reply(101, b)
            }
            Err(e) => return Err(e),
        };
        self.authorize(c)?;
        s.ledger.insert(0, (digest, reply.clone()));
        self.attempts.insert((ns, f.transfer), s);
        Ok(reply)
    }
    fn apply(&mut self, c: &Context, ns: Id, f: &Frame, s: &mut Attempt) -> Result<Frame> {
        if s.state != 0 {
            if s.state == 1 && f.kind == 4 {
                let mut r = Reader::new(&f.body);
                let expected = r.id()?;
                let offer = Offer::decode(r.remaining())?;
                if expected != s.offer.id()? || offer != s.offer {
                    return Err(Error::Integrity);
                }
                let p = self.publication(ns, offer.object)?;
                self.reserve_with_eviction(
                    p.len
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(history_charge(s)))
                        .ok_or(Error::Limit)?,
                )?;
                let bytes = read_publication_for_receipt(&p)?;
                if bytes.len() as u64 != offer.len {
                    return Err(Error::Integrity);
                }
                self.authorize(c)?;
                return Ok(f.reply(103, offer.object.to_vec()));
            }
            if !matches!(f.kind, 5 | 6) {
                return Err(Error::Unknown);
            }
            if !f.body.is_empty() {
                return Err(Error::Malformed);
            }
            return Ok(state_reply(f, s));
        }
        match f.kind {
            2 => {
                self.data(s, &f.body)?;
                Ok(f.reply(102, s.accepted.to_le_bytes().to_vec()))
            }
            3 => {
                if !f.body.is_empty() {
                    return Err(Error::Malformed);
                }
                let mut retained = None;
                if s.offer.mode == Mode::RuntimeV1 && !s.runtime_ready {
                    return Err(Error::Malformed);
                }
                if s.offer.mode == Mode::ZstdPrefixV1 {
                    s.bytes = crate::zstd_prefix::decode_with_resource_profile(
                        &s.base_bytes,
                        &s.prefix_encoded,
                        s.offer.len as usize,
                        self.resources,
                    )?;
                }
                if crate::fitted_transfer::is_buffered(s.offer.mode) {
                    s.bytes = if let Some(codec) = &s.codec {
                        codec
                            .lock()
                            .map_err(|_| Error::Storage)?
                            .decode(&s.prefix_encoded, s.offer.len as usize)?
                    } else {
                        crate::fitted_transfer::decode_with_resource_profile(
                            s.offer.mode,
                            &s.base_bytes,
                            &s.prefix_encoded,
                            s.offer.len as usize,
                            self.resources,
                        )?
                    };
                }
                if s.offer.mode == Mode::Dag {
                    let m = s.offer.manifest.as_ref().ok_or(Error::Malformed)?;
                    let (absent, closure) = verified_closure(m, s, self.limits.max_nodes)?;
                    if !absent.is_empty() {
                        if s.repair >= 2 {
                            return Err(Error::Missing);
                        }
                        s.repair += 1;
                        let mut b = (absent.len() as u16).to_le_bytes().to_vec();
                        for id in absent {
                            b.extend_from_slice(&id);
                        }
                        return Ok(f.reply(104, b));
                    }
                    s.bytes = prepared::rebuild_verified(m, |id| s.node(id))?;
                    // Only closure-owned, previously verified nodes enter trusted retention.
                    let nodes = closure
                        .into_iter()
                        .map(|id| Ok((id, s.node(&id).ok_or(Error::Missing)?.clone())))
                        .collect::<Result<BTreeMap<_, _>>>()?;
                    retained = Some(Arc::new(VerifiedGraph::from_validated_parts(
                        m.clone(),
                        nodes,
                    )));
                }
                if s.bytes.len() as u64 != s.offer.len || object_id(&s.bytes) != s.offer.object {
                    return Err(Error::Integrity);
                }
                let mid = if let Some(m) = &s.offer.manifest {
                    Some(m.id())
                } else if matches!(
                    s.offer.mode,
                    Mode::SparseV1
                        | Mode::ZstdPrefixV1
                        | Mode::RuntimeV1
                        | Mode::SchemaV1
                        | Mode::ZstdDictionaryV1
                        | Mode::SchemaZstdV1
                        | Mode::ZstdWholeV1
                ) || (matches!(s.offer.mode, Mode::Raw | Mode::Zstd)
                    && self.cold_graph_policy == ColdGraphPolicy::Disabled)
                {
                    None
                } else {
                    match VerifiedGraph::fold(&s.bytes, self.limits.max_nodes) {
                        Ok(g) => {
                            let mid = g.manifest().id();
                            retained = Some(Arc::new(g));
                            Some(mid)
                        }
                        Err(Error::Limit) => None,
                        Err(e) => return Err(e),
                    }
                };
                self.authorize(c)?;
                self.publish(c, ns, &s.offer, &s.bytes, mid)?;
                s.state = 1;
                s.clear();
                if let Some(g) = retained {
                    // Retention is optional after a successful durable publication.
                    self.retain_graph(
                        ns,
                        g,
                        history_charge(s) + 256 + crate::MAX_FRAME as u64,
                        s.last,
                    );
                }
                Ok(f.reply(103, s.offer.object.to_vec()))
            }
            4 => {
                let mut r = Reader::new(&f.body);
                let expected = r.id()?;
                let offer = Offer::decode(r.remaining())?;
                if expected != s.offer.id()? || offer != s.offer {
                    return Err(Error::Integrity);
                }
                let mut b = vec![s.offer.mode as u8, u8::from(s.base_pinned)];
                b.extend_from_slice(&(s.next + 1).to_le_bytes());
                Ok(f.reply(101, b))
            }
            5 => {
                if !f.body.is_empty() {
                    return Err(Error::Malformed);
                }
                Ok(state_reply(f, s))
            }
            6 => {
                if !f.body.is_empty() {
                    return Err(Error::Malformed);
                }
                s.state = 2;
                s.clear();
                Ok(state_reply(f, s))
            }
            _ => Err(Error::Sequence),
        }
    }
    fn data(&self, s: &mut Attempt, b: &[u8]) -> Result<()> {
        if crate::fitted_transfer::is_buffered(s.offer.mode) {
            crate::fitted_transfer::append(
                s.offer.mode,
                &mut s.prefix_encoded,
                &mut s.prefix_chunks,
                s.offer.len as usize,
                b,
            )?;
            s.accepted = s.prefix_encoded.len() as u64;
            return Ok(());
        }
        if s.offer.mode == Mode::RuntimeV1 && !s.runtime_ready {
            let prediction = crate::shared_transfer::replay_frame_with_limit(
                &s.base_bytes,
                b,
                self.resources.max_prediction_len(),
            )?;
            s.base_bytes = prediction;
            s.runtime_ready = true;
            return Ok(());
        }
        if s.offer.mode == Mode::ZstdPrefixV1 {
            crate::zstd_prefix::append(
                &mut s.prefix_encoded,
                &mut s.prefix_chunks,
                s.offer.len as usize,
                b,
            )?;
            s.accepted = s.prefix_encoded.len() as u64;
            return Ok(());
        }
        if matches!(s.offer.mode, Mode::SparseV1 | Mode::RuntimeV1) {
            crate::delta::apply(
                &s.base_bytes,
                &mut s.bytes,
                s.offer.len as usize,
                &mut s.sparse_operations,
                b,
            )?;
            s.accepted = s.bytes.len() as u64;
            return Ok(());
        }
        let mut r = Reader::new(b);
        let encoding = r.u8()?;
        let decoded = r.u32()? as usize;
        let payload = r.remaining();
        if s.offer.mode == Mode::Dag {
            if encoding != 2 {
                return Err(Error::Malformed);
            }
            let mut p = Reader::new(payload);
            let count = p.u16()? as usize;
            let mut staged = BTreeMap::new();
            let mut encoded = 0usize;
            for _ in 0..count {
                let prefix = p.take(34)?;
                let n = match (prefix[32], prefix[33]) {
                    (0, 0) => 32,
                    (1, 1..=13) => 96,
                    _ => return Err(Error::Malformed),
                };
                let mut raw = prefix.to_vec();
                raw.extend_from_slice(p.take(n)?);
                encoded += raw.len();
                let node = Node::decode(&raw)?;
                let id: Id = raw[..32].try_into().map_err(|_| Error::Malformed)?;
                if let Some(old) = s.node(&id).or_else(|| staged.get(&id)) {
                    if old != &node {
                        return Err(Error::Integrity);
                    }
                } else {
                    if s.staged_nodes + staged.len() + 1
                        > dag::work_bound(s.offer.len).min(self.limits.max_nodes)
                    {
                        return Err(Error::Limit);
                    }
                    staged.insert(id, node);
                }
            }
            p.end()?;
            if decoded != encoded {
                return Err(Error::Malformed);
            }
            s.staged_nodes += staged.len();
            for (id, node) in staged {
                s.accepted = s
                    .accepted
                    .checked_add(if matches!(node, Node::Leaf(_)) {
                        66
                    } else {
                        130
                    })
                    .ok_or(Error::Limit)?;
                s.nodes.insert(id, node);
            }
            Ok(())
        } else {
            if decoded > MAX_CHUNK {
                return Err(Error::Limit);
            }
            if s.bytes.len().checked_add(decoded).ok_or(Error::Limit)? > s.offer.len as usize {
                return Err(Error::Limit);
            }
            let bytes = match (s.offer.mode, encoding) {
                (_, 0) => {
                    if payload.len() != decoded {
                        return Err(Error::Malformed);
                    }
                    payload.to_vec()
                }
                (Mode::Zstd, 1) => decode_zstd(payload, decoded)?,
                _ => return Err(Error::Malformed),
            };
            s.bytes.extend_from_slice(&bytes);
            s.accepted = s.bytes.len() as u64;
            Ok(())
        }
    }
    fn publication(&self, ns: Id, id: Id) -> Result<Publication> {
        self.open_publication(ns, id).map(|record| record.meta)
    }
    fn open_publication(&self, ns: Id, id: Id) -> Result<OpenPublication> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            let dir = lease.open_namespace(ns, false)?;
            let (meta, file) = open_owned_meta(lease, ns, dir.as_fd(), object_name(&id).as_ref())?;
            lease.check_namespace(ns, dir.as_fd())?;
            return Ok(OpenPublication {
                meta,
                file,
                directory: Some(dir),
            });
        }
        let dir = self.root.path()?.join(hex(&ns));
        if !dir.exists() {
            return Err(Error::Unknown);
        }
        private_dir(&dir)?;
        let (p, file) = open_meta(&self.path(ns, id)?)?;
        if p.object != id {
            return Err(Error::Integrity);
        }
        Ok(OpenPublication {
            meta: p,
            file,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            directory: None,
        })
    }
    fn find_base(&self, ns: Id, mid: Id) -> Result<Publication> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            let mut found = None;
            let mut count = 0;
            lease.visit(Some(ns), |_, dir, name| {
                if Path::new(name).extension() == Some("object".as_ref()) {
                    count += 1;
                    if count > self.limits.max_publications {
                        return Err(Error::Limit);
                    }
                    let (p, _) = open_owned_meta(lease, ns, dir, name)?;
                    if p.manifest == Some(mid) && found.is_none() {
                        found = Some(p);
                    }
                }
                Ok(())
            })?;
            return found.ok_or(Error::Missing);
        }
        let dir = self.root.path()?.join(hex(&ns));
        if dir.exists() {
            private_dir(&dir)?;
        }
        let entries = match fs::read_dir(dir) {
            Ok(x) => x,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::Missing),
            Err(_) => return Err(Error::Storage),
        };
        let mut count = 0;
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || entry.path().extension().is_none_or(|s| s != "object")
            {
                continue;
            }
            count += 1;
            if count > self.limits.max_publications {
                return Err(Error::Limit);
            }
            let p = read_meta(&entry.path())?;
            if p.manifest == Some(mid) {
                return Ok(p);
            }
        }
        Err(Error::Missing)
    }
    fn disk_usage(&self, ns: Id) -> Result<(u64, u64, usize)> {
        #[cfg(test)]
        let _quota = commit_costs::span(commit_costs::Stage::Quota);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            let (mut scoped, mut all, mut count) = (0u64, 0u64, 0);
            lease.visit(None, |space, dir, name| {
                if space.is_some() && Path::new(name).extension() == Some("object".as_ref()) {
                    let n = owned::quota_len_at(dir, name)?;
                    all = all.checked_add(n).ok_or(Error::Limit)?;
                    if space == Some(ns) {
                        scoped = scoped.checked_add(n).ok_or(Error::Limit)?;
                    }
                    count += 1;
                    if count > self.limits.max_publications {
                        return Err(Error::Limit);
                    }
                }
                Ok(())
            })?;
            return Ok((scoped, all, count));
        }
        #[cfg(windows)]
        return storage::quota_usage(self.root.path()?, &hex(&ns), self.limits.max_publications);
        #[cfg(not(windows))]
        {
            let mut all = 0u64;
            let mut scoped = 0u64;
            let mut count = 0;
            for d in fs::read_dir(self.root.path()?)? {
                let d = d?;
                if !d.file_type()?.is_dir()
                    || !canonical_namespace(&d.file_name().to_string_lossy())
                {
                    continue;
                }
                private_dir(&d.path())?;
                for f in fs::read_dir(d.path())? {
                    let f = f?;
                    if !f.file_type()?.is_file()
                        || f.path().extension().is_none_or(|s| s != "object")
                    {
                        continue;
                    }
                    let n = storage::quota_file_len(&f.path())?;
                    all = all.checked_add(n).ok_or(Error::Limit)?;
                    if d.file_name() == hex(&ns).as_str() {
                        scoped = scoped.checked_add(n).ok_or(Error::Limit)?;
                    }
                    count += 1;
                    if count > self.limits.max_publications {
                        return Err(Error::Limit);
                    }
                }
            }
            Ok((scoped, all, count))
        }
    }
    fn publish(
        &self,
        context: &Context,
        ns: Id,
        offer: &Offer,
        bytes: &[u8],
        mid: Option<Id>,
    ) -> Result<()> {
        match self.open_publication(ns, offer.object) {
            Ok(record) => {
                verify_publication(&record.meta, &record.file, bytes)?;
                record.check_named()?;
                match &record.meta.path {
                    Locator::Path(path) => {
                        drop(record.file);
                        storage::sync_existing(path)?;
                    }
                    #[cfg(any(target_os = "linux", target_os = "macos"))]
                    Locator::Owned(lease, ns) => {
                        let dir = record.directory.as_ref().ok_or(Error::Storage)?;
                        lease.sync_publication(
                            *ns,
                            dir.as_fd(),
                            object_name(&record.meta.object).as_ref(),
                            &record.file,
                        )?;
                    }
                }
                self.authorize(context)?;
                return Ok(());
            }
            Err(Error::Unknown) => {}
            Err(e) => return Err(e),
        }
        let (scoped, all, count) = self.disk_usage(ns)?;
        let len = bytes.len() as u64 + if mid.is_some() { 81 } else { 49 };
        if scoped.checked_add(len).ok_or(Error::Limit)? > self.limits.namespace_bytes
            || all.checked_add(len).ok_or(Error::Limit)? > self.limits.cache_bytes
            || count >= self.limits.max_publications
        {
            return Err(Error::Limit);
        }
        let stage = format!(
            ".stage-{}-{}",
            std::process::id(),
            TEMP.fetch_add(1, Ordering::Relaxed)
        );
        match &self.root {
            StoreRoot::Path { path: root, .. } => {
                let path = self.path(ns, offer.object)?;
                let parent = path.parent().ok_or(Error::Storage)?;
                private_dir(parent)?;
                let tmp = parent.join(&stage);
                let mut f = storage::create_new_file(&tmp)?;
                let result = (|| {
                    write_record(&mut f, offer, mid, bytes)?;
                    drop(f);
                    self.authorize(context)?;
                    #[cfg(test)]
                    let _publish = commit_costs::span(commit_costs::Stage::NamespacePublish);
                    storage::publish(&tmp, &path, root)
                })();
                if result.is_err() {
                    let _ = storage::remove_stage(&tmp);
                }
                result?;
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            StoreRoot::Owned(lease) => {
                let extra = match lease.open_namespace(ns, false) {
                    Ok(_) => 1,
                    Err(Error::Unknown) => 2,
                    Err(e) => return Err(e),
                };
                lease.check_room(extra)?;
                let dir = lease.open_namespace(ns, true)?;
                let mut f = owned::create_new_at(dir.as_fd(), stage.as_ref())?;
                let result = (|| {
                    write_record(&mut f, offer, mid, bytes)?;
                    self.authorize(context)?;
                    #[cfg(test)]
                    let _publish = commit_costs::span(commit_costs::Stage::NamespacePublish);
                    lease.publish_at(
                        ns,
                        dir.as_fd(),
                        stage.as_ref(),
                        object_name(&offer.object).as_ref(),
                        &f,
                    )
                })();
                if let Err(e) = result {
                    lease.check_namespace(ns, dir.as_fd())?;
                    // Only the still-named original stage may be removed. A completed rename
                    // can leave a final publication after a later sync failure.
                    match owned::open_read_at(dir.as_fd(), stage.as_ref()) {
                        Ok(_) => owned::unlink_checked_at(dir.as_fd(), stage.as_ref(), &f)?,
                        Err(Error::Unknown) => (),
                        Err(error) => return Err(error),
                    }
                    return Err(e);
                }
            }
        }
        #[cfg(test)]
        let _verify = commit_costs::span(commit_costs::Stage::VerifyPublished);
        let published = self.open_publication(ns, offer.object)?;
        verify_publication(&published.meta, &published.file, bytes)?;
        published.check_named()?;
        self.authorize(context)
    }
    /// Serialized verification; the returned Vec is then owned/accounted by the caller.
    pub fn read_object(&mut self, c: &Context, id: Id) -> Result<Vec<u8>> {
        self.authorize(c)?;
        let p = self.publication(c.namespace()?, id)?;
        if self.resources == crate::resource_profile::ResourceProfile::SmallV1
            && p.len > self.resources.max_artifact_len() as u64
        {
            return Err(Error::Limit);
        }
        self.reserve_with_eviction(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
        let bytes = read_publication(&p)?;
        self.authorize(c)?;
        Ok(bytes)
    }
    // Only the explicit ordered Small facade may retire/evict. Ordinary v1
    // endpoints retain their original replay and archival behavior.
    pub(crate) fn ordered_bounds(&self) -> Result<(usize, u64, u64)> {
        if self.resources != crate::resource_profile::ResourceProfile::SmallV1 {
            return Err(Error::Unsupported);
        }
        Ok((
            self.limits.max_publications,
            self.limits.namespace_bytes,
            self.limits.cache_bytes,
        ))
    }
    pub(crate) fn ordered_attempt_count(&self, c: &Context) -> Result<usize> {
        self.authorize(c)?;
        let ns = c.namespace()?;
        if self.attempts.keys().any(|(space, _)| *space != ns) {
            return Err(Error::Unauthorized);
        }
        Ok(self.attempts.len())
    }
    pub(crate) fn ordered_active_objects(&self, c: &Context) -> Result<BTreeSet<Id>> {
        self.authorize(c)?;
        self.ordered_bounds()?;
        let ns = c.namespace()?;
        let mut pinned = BTreeSet::new();
        for ((space, _), attempt) in &self.attempts {
            if *space != ns {
                return Err(Error::Unauthorized);
            }
            if attempt.state == 0 {
                pinned.insert(attempt.offer.object);
                if let Some(base) = attempt.offer.base {
                    pinned.insert(base);
                }
            }
        }
        Ok(pinned)
    }
    pub(crate) fn ordered_retire(&mut self, c: &Context, transfer: Id) -> Result<()> {
        self.authorize(c)?;
        self.ordered_bounds()?;
        let key = (c.namespace()?, transfer);
        if self
            .attempts
            .get(&key)
            .is_some_and(|attempt| attempt.state != 1)
        {
            return Err(Error::Sequence);
        }
        // A successful Probe has no attempt; an expired terminal record may
        // already have been collected by the unchanged ordinary TTL sweep.
        self.attempts.remove(&key);
        Ok(())
    }
    pub(crate) fn ordered_cache_scan(&self, c: &Context) -> Result<Vec<(Id, u64, u64)>> {
        self.authorize(c)?;
        self.ordered_bounds()?;
        let ns = c.namespace()?;
        let namespace = hex(&ns);
        let mut records = Vec::new();
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            lease.visit(None, |space, dir, name| {
                if space.is_none() {
                    if name != ".owner" && name != ".spiralcore-profile-v1" {
                        let id = crate::wan::parse_id(name.to_str().ok_or(Error::Storage)?)
                            .map_err(|_| Error::Storage)?;
                        // A canonical name alone does not prove a directory occupies it.
                        lease.open_namespace(id, false)?;
                    }
                    return Ok(());
                }
                if space != Some(ns) {
                    return Ok(());
                }
                if records.len() >= self.limits.max_publications {
                    return Err(Error::Limit);
                }
                let (p, file) = open_owned_meta(lease, ns, dir, name)?;
                self.reserve(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
                drop(read_publication_body(&p, &file)?);
                owned::check_file_at(dir, name, &file)?;
                records.push((p.object, p.len, p.disk_len()?));
                Ok(())
            })?;
            self.authorize(c)?;
            return Ok(records);
        }
        for entry in fs::read_dir(self.root.path()?)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".owner" || name == ".spiralcore-profile-v1" {
                continue;
            }
            if !entry.file_type()?.is_dir() || name != namespace.as_str() {
                return Err(Error::Storage);
            }
            private_dir(&entry.path())?;
            for file in fs::read_dir(entry.path())? {
                let file = file?;
                if !file.file_type()?.is_file() || records.len() >= self.limits.max_publications {
                    return Err(Error::Limit);
                }
                let publication = read_meta(&file.path())?;
                if publication.len > self.resources.max_artifact_len() as u64
                    || file.file_name() != format!("{}.object", hex(&publication.object)).as_str()
                {
                    return Err(Error::Integrity);
                }
                self.reserve(publication.len.checked_mul(2).ok_or(Error::Limit)?)?;
                // Scan uses one bounded verification buffer at a time; the
                // retained index contains IDs/scalars, never copied paths.
                drop(read_publication(&publication)?);
                records.push((publication.object, publication.len, publication.disk_len()?));
            }
        }
        self.authorize(c)?;
        Ok(records)
    }
    pub(crate) fn ordered_cache_meta(&self, c: &Context, id: Id) -> Result<(u64, u64)> {
        self.ordered_cache_read(c, id)
            .map(|(bytes, disk)| (bytes.len() as u64, disk))
    }
    pub(crate) fn ordered_cache_read(&self, c: &Context, id: Id) -> Result<(Vec<u8>, u64)> {
        self.authorize(c)?;
        self.ordered_bounds()?;
        let record = self.open_publication(c.namespace()?, id)?;
        let p = &record.meta;
        if p.len > self.resources.max_artifact_len() as u64 {
            return Err(Error::Limit);
        }
        self.reserve(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
        let bytes = read_publication_body(p, &record.file)?;
        record.check_named()?;
        self.authorize(c)?;
        Ok((bytes, p.disk_len()?))
    }
    fn ordered_remove_cache(&mut self, c: &Context, victim: &OrderedEviction) -> Result<()> {
        #[cfg(test)]
        let _victim = commit_costs::span(commit_costs::Stage::Victim);
        self.authorize(c)?;
        self.ordered_bounds()?;
        if self.ordered_active_objects(c)?.contains(&victim.id) {
            return Err(Error::Limit);
        }
        let record = self.open_publication(c.namespace()?, victim.id)?;
        let p = &record.meta;
        let disk = p.disk_len()?;
        if (p.len, disk) != (victim.len, victim.disk) {
            return Err(Error::Storage);
        }
        if p.len > self.resources.max_artifact_len() as u64 {
            return Err(Error::Limit);
        }
        self.reserve(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
        drop(read_publication_body(p, &record.file)?);
        self.authorize(c)?;
        // Reuse the existing private-file deletion backend under this actual
        // exclusive endpoint owner; Windows checks ACL/reparse/hardlink state
        // and applies disposition to its exact opened file handle.
        match &p.path {
            Locator::Path(path) => {
                drop(record.file);
                storage::remove_stage(path)?;
                match fs::symlink_metadata(path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    _ => Err(Error::Storage),
                }
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Locator::Owned(lease, ns) => {
                let dir = record.directory.as_ref().ok_or(Error::Storage)?;
                let name = object_name(&p.object);
                lease.check_namespace(*ns, dir.as_fd())?;
                owned::unlink_checked_at(dir.as_fd(), name.as_ref(), &record.file)?;
                lease.sync_namespace(*ns, dir.as_fd())?;
                match owned::open_read_at(dir.as_fd(), name.as_ref()) {
                    Err(Error::Unknown) => Ok(()),
                    _ => Err(Error::Storage),
                }
            }
        }
    }
    fn apply_ordered_evictions(
        &mut self,
        c: &Context,
        evictions: &[OrderedEviction],
    ) -> Result<()> {
        // Recheck the whole set before deleting any file. The original
        // admitted attempt now holds its resident charge and pins its base.
        // A single removal validates the original charge, bytes, pins and
        // authority immediately before deletion. With multiple victims, verify
        // the entire set first so an invalid later victim preserves the first.
        if evictions.len() > 1 {
            for victim in evictions {
                if self.ordered_cache_meta(c, victim.id).ok() != Some((victim.len, victim.disk)) {
                    return Err(Error::Storage);
                }
            }
        }
        for victim in evictions {
            self.ordered_remove_cache(c, victim)
                .map_err(|_| Error::Storage)?;
        }
        Ok(())
    }
    fn stored_len(&self, ns: Id, id: Id) -> Result<u64> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let StoreRoot::Owned(lease) = &self.root {
            let dir = lease.open_namespace(ns, false)?;
            let length = owned::quota_len_at(dir.as_fd(), object_name(&id).as_ref())?;
            lease.check_namespace(ns, dir.as_fd())?;
            return Ok(length);
        }
        storage::quota_file_len(&self.path(ns, id)?)
    }
    fn is_stored(&self, ns: Id, id: Id) -> Result<bool> {
        match &self.root {
            StoreRoot::Path { .. } => Ok(self.path(ns, id)?.exists()),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            StoreRoot::Owned(_) => match self.stored_len(ns, id) {
                Ok(_) => Ok(true),
                Err(Error::Unknown) => Ok(false),
                Err(e) => Err(e),
            },
        }
    }
    fn admit_disk(&self, ns: Id, offer: &Offer, evictions: &[OrderedEviction]) -> Result<()> {
        let (mut scoped, mut all, mut count) = self.disk_usage(ns)?;
        // Ordered Small admission projects the exact space to be freed.
        // The ordered owner performs deletions only after Begin succeeds.
        for victim in evictions {
            let bytes = self.stored_len(ns, victim.id)?;
            if bytes != victim.disk {
                return Err(Error::Storage);
            }
            scoped = scoped.checked_sub(bytes).ok_or(Error::Storage)?;
            all = all.checked_sub(bytes).ok_or(Error::Storage)?;
            count = count.checked_sub(1).ok_or(Error::Storage)?;
        }
        let mut pending = BTreeMap::new();
        for ((space, _), s) in &self.attempts {
            if s.state == 0 && !self.is_stored(*space, s.offer.object)? {
                if *space == ns && s.offer.object == offer.object && s.offer.len != offer.len {
                    return Err(Error::Integrity);
                }
                pending.insert((*space, s.offer.object), s.offer.len + 81);
            }
        }
        pending.insert((ns, offer.object), offer.len + 81);
        for ((space, _), len) in pending {
            all = all.checked_add(len).ok_or(Error::Limit)?;
            if space == ns {
                scoped = scoped.checked_add(len).ok_or(Error::Limit)?;
            }
            count += 1;
        }
        if scoped > self.limits.namespace_bytes
            || all > self.limits.cache_bytes
            || count > self.limits.max_publications
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
/// All values came from Node::decode or immutable verified ownership. Never use
/// this traversal with an externally mutable Graph or unverified map.
fn verified_closure(
    m: &dag::Manifest,
    s: &Attempt,
    max_nodes: usize,
) -> Result<(Vec<Id>, BTreeSet<Id>)> {
    m.validate()?;
    let mut seen = BTreeSet::new();
    let mut missing = BTreeSet::new();
    let mut stack = m.roots.clone();
    let mut visits = 0;
    while let Some((scale, id)) = stack.pop() {
        visits += 1;
        if visits > dag::work_bound(m.len) {
            return Err(Error::Limit);
        }
        let Some(node) = s.node(&id) else {
            missing.insert(id);
            if missing.len() > 1024 {
                return Err(Error::Limit);
            }
            continue;
        };
        if node.scale() != scale {
            return Err(Error::Integrity);
        }
        if !seen.insert(id) {
            continue;
        }
        if seen.len() > max_nodes {
            return Err(Error::Limit);
        }
        if let Node::Parent { children, .. } = node {
            let child_scale = scale.checked_sub(1).ok_or(Error::Integrity)?;
            for child in children {
                stack.push((child_scale, *child));
            }
        }
    }
    Ok((missing.into_iter().collect(), seen))
}
fn history_charge(s: &Attempt) -> u64 {
    512 + s
        .ledger
        .values()
        .map(|(_, f)| 256 + f.body.len() as u64)
        .sum::<u64>()
}
fn state_reply(f: &Frame, s: &Attempt) -> Frame {
    let mut b = vec![s.state];
    b.extend_from_slice(&s.accepted.to_le_bytes());
    f.reply(105, b)
}
fn write_record(file: &mut File, offer: &Offer, mid: Option<Id>, bytes: &[u8]) -> Result<()> {
    file.write_all(b"SCPUB001")?;
    file.write_all(&offer.len.to_le_bytes())?;
    file.write_all(&offer.object)?;
    file.write_all(&[u8::from(mid.is_some())])?;
    if let Some(m) = mid {
        file.write_all(&m)?;
    }
    file.write_all(bytes)?;
    #[cfg(test)]
    let _file_sync = commit_costs::span(commit_costs::Stage::FileSync);
    file.sync_all()?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_owned_meta(
    lease: &OwnedStoreLease,
    ns: Id,
    dir: std::os::fd::BorrowedFd<'_>,
    name: &std::ffi::OsStr,
) -> Result<(Publication, File)> {
    let f = owned::open_read_at(dir, name)?;
    let p = read_meta_file(Locator::Owned(lease.clone(), ns), &f)?;
    if name != object_name(&p.object).as_str() {
        return Err(Error::Integrity);
    }
    if p.len > crate::resource_profile::SMALL_ARTIFACT as u64 {
        return Err(Error::Limit);
    }
    Ok((p, f))
}

fn read_meta(path: &Path) -> Result<Publication> {
    open_meta(path).map(|(metadata, _)| metadata)
}
fn open_meta(path: &Path) -> Result<(Publication, File)> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            // Windows open_read validates the actual consumed handle. Its
            // discarded private_file preflight repeats the same native checks.
            #[cfg(not(windows))]
            private_file(path)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::Unknown),
        Err(_) => return Err(Error::Storage),
    }
    let f = storage::open_read(path)?;
    let metadata = read_meta_file(Locator::Path(path.to_owned()), &f)?;
    Ok((metadata, f))
}
fn read_meta_file<L>(path: L, mut f: &File) -> Result<Publication<L>> {
    #[cfg(test)]
    ordered_eviction_tests::HEADER_READS.with(|count| count.set(count.get() + 1));
    let mut h = [0u8; 49];
    f.read_exact(&mut h).map_err(|_| Error::Integrity)?;
    if &h[..8] != b"SCPUB001" {
        return Err(Error::Integrity);
    }
    let len = u64::from_le_bytes(h[8..16].try_into().map_err(|_| Error::Integrity)?);
    if len > MAX_OBJECT {
        return Err(Error::Limit);
    }
    let object = h[16..48].try_into().map_err(|_| Error::Integrity)?;
    let manifest = match h[48] {
        0 => None,
        1 => {
            let mut m = [0; 32];
            f.read_exact(&mut m).map_err(|_| Error::Integrity)?;
            Some(m)
        }
        _ => return Err(Error::Integrity),
    };
    if f.metadata()?.len() != len + if manifest.is_some() { 81 } else { 49 } {
        return Err(Error::Integrity);
    }
    Ok(Publication {
        path,
        len,
        object,
        manifest,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn verify_owned_file(id: Id, file: File, max_body: u64) -> Result<()> {
    let metadata = read_meta_file((), &file)?;
    if metadata.object != id {
        return Err(Error::Integrity);
    }
    if metadata.len > max_body {
        return Err(Error::Limit);
    }
    read_publication_body(&metadata, file).map(drop)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn object_name(id: &Id) -> String {
    format!("{}.object", hex(id))
}

fn canonical_namespace(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn owned_stage(s: &str) -> bool {
    let Some(rest) = s.strip_prefix(".stage-") else {
        return false;
    };
    let mut p = rest.split('-');
    let digits =
        |x: Option<&str>| x.is_some_and(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()));
    digits(p.next()) && digits(p.next()) && p.next().is_none()
}
fn read_publication_for_receipt(p: &Publication) -> Result<Vec<u8>> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Locator::Owned(lease, ns) = &p.path {
        return read_owned_publication(p, lease, *ns, true);
    }
    read_publication(p)
}
fn read_publication(p: &Publication) -> Result<Vec<u8>> {
    match &p.path {
        Locator::Path(path) => read_reopened_publication(p, &storage::open_read(path)?),
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        Locator::Owned(lease, ns) => read_owned_publication(p, lease, *ns, false),
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn read_owned_publication(
    p: &Publication,
    lease: &OwnedStoreLease,
    ns: Id,
    durable_receipt: bool,
) -> Result<Vec<u8>> {
    let dir = lease.open_namespace(ns, false)?;
    let name = object_name(&p.object);
    let file = owned::open_read_at(dir.as_fd(), name.as_ref())?;
    let bytes = read_reopened_publication(p, &file)?;
    if durable_receipt {
        // A previous rename may be visible even though its directory sync failed.
        // Finish the original durability barrier before returning Already.
        lease.sync_publication(ns, dir.as_fd(), name.as_ref(), &file)?;
    } else {
        owned::check_file_at(dir.as_fd(), name.as_ref(), &file)?;
        lease.check_namespace(ns, dir.as_fd())?;
    }
    Ok(bytes)
}
fn read_reopened_publication(p: &Publication, f: &File) -> Result<Vec<u8>> {
    let fresh = read_meta_file((), f)?;
    if fresh.object != p.object || fresh.len != p.len || fresh.manifest != p.manifest {
        return Err(Error::Integrity);
    }
    read_publication_body(p, f)
}
// The freshly opened, checked file is positioned just after its validated header.
fn read_publication_body<L>(p: &Publication<L>, f: impl Read) -> Result<Vec<u8>> {
    #[cfg(test)]
    ordered_eviction_tests::FULL_READS.with(|count| count.set(count.get() + 1));
    let mut data = Vec::new();
    data.try_reserve_exact(p.len as usize)
        .map_err(|_| Error::Limit)?;
    f.take(p.len + 1).read_to_end(&mut data)?;
    if data.len() as u64 != p.len || object_id(&data) != p.object {
        return Err(Error::Integrity);
    }
    Ok(data)
}
fn verify_publication<L>(p: &Publication<L>, mut f: impl Read, expected: &[u8]) -> Result<()> {
    if expected.len() as u64 != p.len || object_id(expected) != p.object {
        return Err(Error::Integrity);
    }
    // Full byte equality against the canonical-digest-checked bytes proves the
    // readback's identity without allocating a second object-sized plaintext.
    let mut buffer = [0u8; 8192];
    for chunk in expected.chunks(buffer.len()) {
        f.read_exact(&mut buffer[..chunk.len()])
            .map_err(|_| Error::Integrity)?;
        if &buffer[..chunk.len()] != chunk {
            return Err(Error::Integrity);
        }
    }
    if f.read(&mut buffer[..1])? != 0 {
        return Err(Error::Integrity);
    }
    Ok(())
}
pub fn decode_zstd(b: &[u8], decoded: usize) -> Result<Vec<u8>> {
    if decoded > MAX_CHUNK {
        return Err(Error::Limit);
    }
    if b.get(..4) != Some(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Err(Error::Malformed);
    }
    if zstd::zstd_safe::find_frame_compressed_size(b).map_err(|_| Error::Malformed)? != b.len()
        || zstd::zstd_safe::get_dict_id_from_frame(b).is_some()
    {
        return Err(Error::Malformed);
    }
    let mut decoder = zstd::stream::read::Decoder::new(b)
        .map_err(|_| Error::Malformed)?
        .single_frame();
    decoder.window_log_max(20).map_err(|_| Error::Malformed)?;
    let mut out = Vec::with_capacity(decoded);
    decoder
        .take(decoded as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| Error::Malformed)?;
    if out.len() != decoded {
        return Err(Error::Integrity);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "endpoint/storage/tests.rs"]
mod storage_tests;

#[cfg(test)]
#[path = "endpoint/metadata_tests.rs"]
mod metadata_tests;

#[cfg(test)]
mod ordered_eviction_tests;

#[cfg(test)]
mod commit_costs;
