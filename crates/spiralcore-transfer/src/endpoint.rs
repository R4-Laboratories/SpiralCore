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
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
};

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

#[derive(Debug, Clone)]
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
        self.reserved = 0;
    }
    fn reject(&mut self, e: Error) {
        self.state = 3;
        self.error = Some(e);
        self.clear();
    }
}
pub struct Endpoint<A: Authority> {
    root: PathBuf,
    _owner: File,
    limits: Limits,
    cold_graph_policy: ColdGraphPolicy,
    authority: A,
    attempts: BTreeMap<(Id, Id), Attempt>,
    graphs: BTreeMap<(Id, Id), (Arc<VerifiedGraph>, u64)>,
    graph_stats: GraphCacheStats,
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
struct Publication {
    path: PathBuf,
    len: u64,
    object: Id,
    manifest: Option<Id>,
}
static TEMP: AtomicU64 = AtomicU64::new(0);

impl<A: Authority> Endpoint<A> {
    pub fn open(root: &Path, limits: Limits, authority: A) -> Result<Self> {
        Self::open_with_policy(root, limits, authority, ColdGraphPolicy::EagerBounded)
    }
    pub fn open_with_policy(
        root: &Path,
        limits: Limits,
        authority: A,
        cold_graph_policy: ColdGraphPolicy,
    ) -> Result<Self> {
        if limits.max_active == 0
            || limits.max_active > 4
            || limits.max_nodes == 0
            || limits.max_states < limits.max_active
            || limits.graph_cache_bytes > 64 << 20
            || limits.graph_cache_entries > 32
        {
            return Err(Error::Limit);
        }
        private_dir(root)?;
        let marker = root.join(".spiralcore-profile-v1");
        let initialized = marker.exists();
        if initialized {
            private_file(&marker)?;
            let mut bytes = Vec::new();
            File::open(&marker)?.take(64).read_to_end(&mut bytes)?;
            if bytes != b"SpiralCore private object cache v1\n" {
                return Err(Error::Storage);
            }
        } else if fs::read_dir(root)?.next().is_some() {
            return Err(Error::Storage);
        }
        let owner_path = root.join(".owner");
        if fs::symlink_metadata(&owner_path).is_ok() {
            private_file(&owner_path)?;
        }
        let owner = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&owner_path)?;
        owner.try_lock().map_err(|_| Error::Storage)?;
        if !initialized {
            let mut f = private_options()
                .write(true)
                .create_new(true)
                .open(&marker)?;
            f.write_all(b"SpiralCore private object cache v1\n")?;
            f.sync_all()?;
            File::open(root)?.sync_all()?;
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
                    fs::remove_file(f.path())?;
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            _owner: owner,
            limits,
            cold_graph_policy,
            authority,
            attempts: BTreeMap::new(),
            graphs: BTreeMap::new(),
            graph_stats: GraphCacheStats::default(),
        })
    }
    fn path(&self, ns: Id, id: Id) -> PathBuf {
        self.root
            .join(hex(&ns))
            .join(format!("{}.object", hex(&id)))
    }
    fn charged(&self) -> u64 {
        self.graph_cache_stats().resident_bytes
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
            if !self.evict_graph() {
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
        self.authority.authorize(c)?;
        let ns = c.namespace()?;
        let wire = f.encode()?;
        if !matches!(f.kind, 1..=6) {
            return Err(Error::Unsupported);
        }
        self.sweep(now);
        let key = (ns, f.transfer);
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
                self.reserve_with_eviction(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
                let bytes = read_publication(&p)?;
                if bytes.len() as u64 != offer.len {
                    return Err(Error::Integrity);
                }
                self.authority.authorize(c)?;
                return Ok(f.reply(103, offer.object.to_vec()));
            }
            if f.kind != 1 {
                return Err(Error::Unknown);
            }
            return self.begin(c, ns, f, digest, now);
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
    fn begin(&mut self, c: &Context, ns: Id, f: &Frame, digest: Id, now: u64) -> Result<Frame> {
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
            self.admit_disk(ns, &offer)?;
        } else if let Err(e) = existing {
            return Err(e);
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
                .and_then(|n| n.checked_add(crate::zstd_prefix::DECODER_ALLOWANCE))
                .ok_or(Error::Limit)?;
        }
        if crate::fitted_transfer::is_fitted(offer.mode) && existing.is_err() {
            charge = charge
                .checked_add(
                    crate::fitted_transfer::encoded_cap(offer.mode, offer.len as usize)? as u64,
                )
                .and_then(|n| n.checked_add(crate::zstd_prefix::DECODER_ALLOWANCE))
                .ok_or(Error::Limit)?;
        }
        if offer.mode == Mode::RuntimeV1 && existing.is_err() {
            // A prediction may be larger than the target after a truncation.
            // Reserve the bounded generator's capacity/growth separately from
            // target output, checkpoint bytes and the existing scratch allowance.
            charge = charge
                .checked_add(2 * crate::shared_runtime::MAX_OUTPUT as u64)
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
        };
        let reply = match existing {
            Ok(p) => {
                let data = read_publication(&p)?;
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
                if offer.mode == Mode::ZstdPrefixV1 {
                    s.prefix_encoded
                        .try_reserve_exact(crate::zstd_prefix::encoded_cap(offer.len as usize)?)
                        .map_err(|_| Error::Limit)?;
                } else if crate::fitted_transfer::is_fitted(offer.mode) {
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
        self.authority.authorize(c)?;
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
                let bytes = read_publication(&p)?;
                if bytes.len() as u64 != offer.len {
                    return Err(Error::Integrity);
                }
                self.authority.authorize(c)?;
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
                    s.bytes = crate::zstd_prefix::decode(
                        &s.base_bytes,
                        &s.prefix_encoded,
                        s.offer.len as usize,
                    )?;
                }
                if crate::fitted_transfer::is_fitted(s.offer.mode) {
                    s.bytes = crate::fitted_transfer::decode(
                        s.offer.mode,
                        &s.base_bytes,
                        &s.prefix_encoded,
                        s.offer.len as usize,
                    )?;
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
                self.authority.authorize(c)?;
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
        if crate::fitted_transfer::is_fitted(s.offer.mode) {
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
            let prediction = crate::shared_transfer::replay_frame(&s.base_bytes, b)?;
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
        let dir = self.root.join(hex(&ns));
        if !dir.exists() {
            return Err(Error::Unknown);
        }
        private_dir(&dir)?;
        let p = read_meta(&self.path(ns, id))?;
        if p.object != id {
            return Err(Error::Integrity);
        }
        Ok(p)
    }
    fn find_base(&self, ns: Id, mid: Id) -> Result<Publication> {
        let dir = self.root.join(hex(&ns));
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
        let mut all = 0u64;
        let mut scoped = 0u64;
        let mut count = 0;
        for d in fs::read_dir(&self.root)? {
            let d = d?;
            if !d.file_type()?.is_dir() || !canonical_namespace(&d.file_name().to_string_lossy()) {
                continue;
            }
            for f in fs::read_dir(d.path())? {
                let f = f?;
                if !f.file_type()?.is_file() || f.path().extension().is_none_or(|s| s != "object") {
                    continue;
                }
                let n = f.metadata()?.len();
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
    fn publish(
        &self,
        context: &Context,
        ns: Id,
        offer: &Offer,
        bytes: &[u8],
        mid: Option<Id>,
    ) -> Result<()> {
        match self.publication(ns, offer.object) {
            Ok(p) => {
                if read_publication(&p)? != bytes {
                    return Err(Error::Integrity);
                }
                File::open(p.path.parent().ok_or(Error::Storage)?)?.sync_all()?;
                self.authority.authorize(context)?;
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
        let path = self.path(ns, offer.object);
        let parent = path.parent().ok_or(Error::Storage)?;
        private_dir(parent)?;
        let tmp = parent.join(format!(
            ".stage-{}-{}",
            std::process::id(),
            TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut f = private_options().write(true).create_new(true).open(&tmp)?;
            f.write_all(b"SCPUB001")?;
            f.write_all(&offer.len.to_le_bytes())?;
            f.write_all(&offer.object)?;
            f.write_all(&[u8::from(mid.is_some())])?;
            if let Some(m) = mid {
                f.write_all(&m)?;
            }
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            self.authority.authorize(context)?;
            fs::rename(&tmp, &path)?;
            File::open(parent)?.sync_all()?;
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }
    /// Serialized verification; the returned Vec is then owned/accounted by the caller.
    pub fn read_object(&mut self, c: &Context, id: Id) -> Result<Vec<u8>> {
        self.authority.authorize(c)?;
        let p = self.publication(c.namespace()?, id)?;
        self.reserve_with_eviction(p.len.checked_mul(2).ok_or(Error::Limit)?)?;
        let bytes = read_publication(&p)?;
        self.authority.authorize(c)?;
        Ok(bytes)
    }
    fn admit_disk(&self, ns: Id, offer: &Offer) -> Result<()> {
        let (mut scoped, mut all, mut count) = self.disk_usage(ns)?;
        let mut pending = BTreeMap::new();
        for ((space, _), s) in &self.attempts {
            if s.state == 0 && !self.path(*space, s.offer.object).exists() {
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
fn read_meta(path: &Path) -> Result<Publication> {
    match fs::symlink_metadata(path) {
        Ok(_) => private_file(path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::Unknown),
        Err(_) => return Err(Error::Storage),
    }
    let mut f = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::Unknown
        } else {
            Error::Storage
        }
    })?;
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
        path: path.to_path_buf(),
        len,
        object,
        manifest,
    })
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
fn private_options() -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o
}
fn private_file(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err(Error::Storage);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode() & 0o077 != 0 {
            return Err(Error::Storage);
        }
    }
    #[cfg(not(unix))]
    return Err(Error::Unsupported);
    Ok(())
}
fn private_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => {
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err(Error::Storage);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if m.permissions().mode() & 0o077 != 0 {
                    return Err(Error::Storage);
                }
            }
            #[cfg(not(unix))]
            return Err(Error::Unsupported);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            if !parent.exists() {
                private_dir(parent)?;
            }
            let mut b = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                b.mode(0o700);
            }
            #[cfg(not(unix))]
            return Err(Error::Unsupported);
            b.create(path)?;
            File::open(path)?.sync_all()?;
            File::open(parent)?.sync_all()?;
        }
        Err(_) => return Err(Error::Storage),
    }
    Ok(())
}
fn read_publication(p: &Publication) -> Result<Vec<u8>> {
    let fresh = read_meta(&p.path)?;
    if fresh.object != p.object || fresh.len != p.len || fresh.manifest != p.manifest {
        return Err(Error::Integrity);
    }
    let mut f = File::open(&p.path)?;
    let mut header = vec![0; if p.manifest.is_some() { 81 } else { 49 }];
    f.read_exact(&mut header)?;
    let mut data = Vec::new();
    data.try_reserve_exact(p.len as usize)
        .map_err(|_| Error::Limit)?;
    f.take(p.len + 1).read_to_end(&mut data)?;
    if data.len() as u64 != p.len || object_id(&data) != p.object {
        return Err(Error::Integrity);
    }
    Ok(data)
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
