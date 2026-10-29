//! Immutable verified graphs and bounded same-length object preparation.
//!
//! Mutable public [`Graph`] values never grant trusted-node status. Prepared
//! objects own their bytes and graph, and updates hash only changed leaves and
//! their ancestors while retaining a mandatory whole-object SHA-256 pass.

use crate::{
    dag::{work_bound, Graph, Manifest, Node},
    object_id, Error, Id, Result,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const NODE_CHARGE: u64 = 512;
const REACHABLE_ID_CHARGE: u64 = 128;
const METADATA_CHARGE: u64 = 1024;

fn graph_charge(nodes: usize) -> u64 {
    // Counts are bounded by the canonical occurrence count for MAX_OBJECT.
    nodes as u64 * NODE_CHARGE + METADATA_CHARGE
}

/// An immutable, complete canonical graph with verified node identities.
///
/// There is deliberately no public constructor accepting a mutable `Graph`.
///
/// ```compile_fail
/// use spiralcore_transfer::{dag::Graph, prepared::VerifiedGraph};
/// let graph = Graph::fold(b"data", 100).unwrap();
/// let poisoned = VerifiedGraph { graph };
/// ```
#[derive(Debug)]
pub struct VerifiedGraph {
    graph: Graph,
}
impl VerifiedGraph {
    pub(crate) fn fold(bytes: &[u8], max_nodes: usize) -> Result<Self> {
        let graph = Graph::fold(bytes, max_nodes)?;
        Ok(Self::from_validated_parts(graph.manifest, graph.nodes))
    }

    /// Adopt internal ownership after validation, without rehashing nodes.
    ///
    /// The caller MUST have validated canonical manifest shape, each node's ID
    /// and scale, the complete reachable closure (with no unrelated entries),
    /// exact reconstructed length/padding and the whole-object SHA-256. Only
    /// direct construction and fully validated received ownership may call this.
    pub(crate) fn from_validated_parts(manifest: Manifest, nodes: BTreeMap<Id, Node>) -> Self {
        Self {
            graph: Graph { manifest, nodes },
        }
    }
    pub fn manifest(&self) -> &Manifest {
        &self.graph.manifest
    }
    pub fn node(&self, id: &Id) -> Option<&Node> {
        self.graph.nodes.get(id)
    }
    pub fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }
    /// Conservative owned graph accounting, not a process-RSS measurement.
    pub fn resident_charge(&self) -> u64 {
        graph_charge(self.node_count())
    }
    pub(crate) fn nodes(&self) -> &BTreeMap<Id, Node> {
        &self.graph.nodes
    }
    /// Reconstruct verified nodes without rehashing them; check the final bytes.
    pub fn rebuild(&self) -> Result<Vec<u8>> {
        rebuild_verified(self.manifest(), |id| self.node(id))
    }
}

/// Owned bytes paired with immutable verified content, suitable for repeated edits.
#[derive(Debug, Clone)]
pub struct PreparedObject {
    graph: Arc<VerifiedGraph>,
    bytes: Arc<[u8]>,
    max_nodes: usize,
}
impl PreparedObject {
    pub fn new(bytes: &[u8], max_nodes: usize) -> Result<Self> {
        Ok(Self {
            graph: Arc::new(VerifiedGraph::fold(bytes, max_nodes)?),
            bytes: Arc::from(bytes),
            max_nodes,
        })
    }
    pub fn graph(&self) -> &Arc<VerifiedGraph> {
        &self.graph
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Includes the complete graph and owned bytes, even when an Arc is shared.
    pub fn resident_charge(&self) -> u64 {
        self.graph.resident_charge() + self.bytes.len() as u64 + METADATA_CHARGE
    }

    /// Prepare a same-length update. Different lengths require a full-fold fallback.
    ///
    /// Comparison and whole-object hashing remain O(bytes). Only changed leaves
    /// and ancestors receive new node hashes. A bounded target-closure traversal
    /// enforces the unique-node limit and establishes repair authorization.
    pub fn prepare_delta(&self, target: &[u8]) -> Result<PreparedDelta> {
        if target.len() != self.bytes.len() {
            return Err(Error::Unsupported);
        }
        let object = object_id(target);
        if target == self.bytes.as_ref() {
            if object != self.graph.manifest().object {
                return Err(Error::Integrity);
            }
            return Ok(PreparedDelta {
                base: Arc::clone(&self.graph),
                bytes: Arc::clone(&self.bytes),
                manifest: self.graph.manifest().clone(),
                overlay: BTreeMap::new(),
                reachable: None,
                max_nodes: self.max_nodes,
            });
        }
        let changed: Vec<usize> = target
            .chunks(32)
            .zip(self.bytes.chunks(32))
            .enumerate()
            .filter_map(|(i, (new, old))| (new != old).then_some(i))
            .collect();
        let mut builder = Update {
            base: &self.graph,
            target,
            overlay: BTreeMap::new(),
            max_nodes: self.max_nodes,
        };
        let mut roots = Vec::with_capacity(self.graph.manifest().roots.len());
        let mut start = 0;
        let mut pending = changed.as_slice();
        for &(scale, id) in &self.graph.manifest().roots {
            let end = start + 3usize.pow(scale as u32);
            let split = pending.partition_point(|&i| i < end);
            let updated = builder.branch(id, scale, start, &pending[..split])?;
            roots.push((scale, updated));
            pending = &pending[split..];
            start = end;
        }
        if !pending.is_empty() {
            return Err(Error::Integrity);
        }
        let manifest = Manifest {
            len: target.len() as u64,
            object,
            roots,
        };
        let reachable = collect_reachable(&manifest, self.max_nodes, |id| {
            builder.overlay.get(id).or_else(|| self.graph.node(id))
        })?;
        // Every newly built node must belong to the target, including when
        // identical subtrees collapse at multiple updated positions.
        if builder.overlay.keys().any(|id| !reachable.contains(id)) {
            return Err(Error::Integrity);
        }
        Ok(PreparedDelta {
            base: Arc::clone(&self.graph),
            bytes: Arc::from(target),
            manifest,
            overlay: builder.overlay,
            reachable: Some(reachable),
            max_nodes: self.max_nodes,
        })
    }
}

/// A short-lived update sharing one flat base graph, never a chain of deltas.
#[derive(Debug)]
pub struct PreparedDelta {
    base: Arc<VerifiedGraph>,
    bytes: Arc<[u8]>,
    manifest: Manifest,
    overlay: BTreeMap<Id, Node>,
    // None is the exact-repeat case: every base node remains reachable.
    reachable: Option<BTreeSet<Id>>,
    max_nodes: usize,
}
impl PreparedDelta {
    /// Identity of the immutable base whose records this update may reuse.
    pub fn base_manifest(&self) -> &Manifest {
        self.base.manifest()
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn is_repeat(&self) -> bool {
        self.reachable.is_none()
    }
    pub fn node_count(&self) -> usize {
        self.reachable
            .as_ref()
            .map_or_else(|| self.base.node_count(), BTreeSet::len)
    }
    /// Newly required records, sorted by canonical ID, matching full-fold delta.
    pub fn new_records(&self) -> impl ExactSizeIterator<Item = (&Id, &Node)> {
        self.overlay.iter()
    }
    /// Lookup is restricted to the target closure, even if the base owns an ID.
    pub fn node(&self, id: &Id) -> Option<&Node> {
        if self.reachable.as_ref().is_some_and(|set| !set.contains(id)) {
            return None;
        }
        self.overlay.get(id).or_else(|| self.base.node(id))
    }
    pub fn rebuild(&self) -> Result<Vec<u8>> {
        rebuild_verified(&self.manifest, |id| self.node(id))
    }
    /// Resolve at most 1,024 requested records in request order. Callers that
    /// need descendant repair can walk `node()`; both paths enforce ownership.
    pub fn repair_records(&self, ids: &[Id]) -> Result<Vec<Node>> {
        if ids.len() > 1024 {
            return Err(Error::Limit);
        }
        ids.iter()
            .map(|id| self.node(id).cloned().ok_or(Error::Integrity))
            .collect()
    }
    /// Charges the full retained base, new nodes, target-reachability index,
    /// target bytes and metadata. Other live PreparedObjects remain the
    /// caller's responsibility; shared allocations are conservatively charged.
    pub fn resident_charge(&self) -> u64 {
        self.base.resident_charge()
            + self.overlay.len() as u64 * NODE_CHARGE
            + self.reachable.as_ref().map_or(0, BTreeSet::len) as u64 * REACHABLE_ID_CHARGE
            + self.bytes.len() as u64
            + METADATA_CHARGE
    }
    /// Peak reservation for promotion while this delta still retains its base.
    /// Exact repeats reuse the already-flat graph without allocating an index.
    pub fn promotion_peak_charge(&self) -> u64 {
        self.resident_charge()
            + if self.is_repeat() {
                0
            } else {
                graph_charge(self.node_count()) + METADATA_CHARGE
            }
    }
    /// Consume this delta and produce an independent flat base for the next edit.
    ///
    /// The caller must reserve the reported peak in its aggregate memory budget.
    /// This method rejects insufficient reservation before index allocation.
    /// Node values are copied from verified ownership without hashing again.
    pub fn promote(self, max_resident_bytes: u64) -> Result<PreparedObject> {
        if self.promotion_peak_charge() > max_resident_bytes {
            return Err(Error::Limit);
        }
        let graph = if let Some(reachable) = &self.reachable {
            let mut nodes = BTreeMap::new();
            for id in reachable {
                nodes.insert(*id, self.node(id).ok_or(Error::Integrity)?.clone());
            }
            Arc::new(VerifiedGraph::from_validated_parts(self.manifest, nodes))
        } else {
            self.base
        };
        Ok(PreparedObject {
            graph,
            bytes: self.bytes,
            max_nodes: self.max_nodes,
        })
    }
}

struct Update<'a> {
    base: &'a VerifiedGraph,
    target: &'a [u8],
    overlay: BTreeMap<Id, Node>,
    max_nodes: usize,
}
impl Update<'_> {
    fn branch(&mut self, old: Id, scale: u8, start: usize, changed: &[usize]) -> Result<Id> {
        if changed.is_empty() {
            return Ok(old);
        }
        let original = self.base.node(&old).ok_or(Error::Integrity)?;
        if original.scale() != scale {
            return Err(Error::Integrity);
        }
        let node = match original {
            Node::Leaf(_) => {
                let offset = start * 32;
                let chunk = self
                    .target
                    .get(offset..self.target.len().min(offset + 32))
                    .ok_or(Error::Integrity)?;
                let mut data = [0; 32];
                data[..chunk.len()].copy_from_slice(chunk);
                Node::Leaf(data)
            }
            Node::Parent { children, .. } => {
                let width = 3usize.pow((scale - 1) as u32);
                let mut pending = changed;
                let mut updated = *children;
                for (i, child) in updated.iter_mut().enumerate() {
                    let child_start = start + i * width;
                    let split = pending.partition_point(|&n| n < child_start + width);
                    *child = self.branch(*child, scale - 1, child_start, &pending[..split])?;
                    pending = &pending[split..];
                }
                Node::Parent {
                    scale,
                    children: updated,
                }
            }
        };
        let id = node.id();
        if let Some(existing) = self.base.nodes().get(&id).or_else(|| self.overlay.get(&id)) {
            if existing != &node {
                return Err(Error::Integrity);
            }
        } else {
            if self.overlay.len() >= self.max_nodes {
                return Err(Error::Limit);
            }
            self.overlay.insert(id, node);
        }
        Ok(id)
    }
}

fn collect_reachable<'a>(
    manifest: &Manifest,
    max_nodes: usize,
    lookup: impl Fn(&Id) -> Option<&'a Node>,
) -> Result<BTreeSet<Id>> {
    manifest.validate()?;
    let mut seen = BTreeSet::new();
    let mut stack = manifest.roots.clone();
    let mut visits = 0;
    while let Some((scale, id)) = stack.pop() {
        visits += 1;
        if visits > work_bound(manifest.len) {
            return Err(Error::Limit);
        }
        let node = lookup(&id).ok_or(Error::Missing)?;
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
    Ok(seen)
}

pub(crate) fn rebuild_verified<'a>(
    manifest: &Manifest,
    lookup: impl Fn(&Id) -> Option<&'a Node>,
) -> Result<Vec<u8>> {
    manifest.validate()?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(manifest.len as usize)
        .map_err(|_| Error::Limit)?;
    let mut stack: Vec<_> = manifest.roots.iter().rev().copied().collect();
    let mut visits = 0;
    while let Some((scale, id)) = stack.pop() {
        visits += 1;
        if visits > work_bound(manifest.len) {
            return Err(Error::Limit);
        }
        let node = lookup(&id).ok_or(Error::Missing)?;
        if node.scale() != scale {
            return Err(Error::Integrity);
        }
        match node {
            Node::Leaf(data) => {
                let left = (manifest.len as usize)
                    .checked_sub(bytes.len())
                    .ok_or(Error::Integrity)?;
                let count = left.min(32);
                if data[count..].iter().any(|&b| b != 0) {
                    return Err(Error::Integrity);
                }
                bytes.extend_from_slice(&data[..count]);
            }
            Node::Parent { children, .. } => {
                let child_scale = scale.checked_sub(1).ok_or(Error::Integrity)?;
                for child in children.iter().rev() {
                    stack.push((child_scale, *child));
                }
            }
        }
    }
    if bytes.len() != manifest.len as usize || object_id(&bytes) != manifest.object {
        return Err(Error::Integrity);
    }
    Ok(bytes)
}
