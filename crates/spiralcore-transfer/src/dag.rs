use crate::{hash, object_id, protocol::Reader, Error, Id, Result, MAX_OBJECT};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Leaf([u8; 32]),
    Parent { scale: u8, children: [Id; 3] },
}
impl Node {
    pub fn scale(&self) -> u8 {
        match self {
            Self::Leaf(_) => 0,
            Self::Parent { scale, .. } => *scale,
        }
    }
    fn body(&self) -> Vec<u8> {
        match self {
            Self::Leaf(data) => {
                let mut b = vec![0, 0];
                b.extend_from_slice(data);
                b
            }
            Self::Parent { scale, children } => {
                let mut b = vec![1, *scale];
                for c in children {
                    b.extend_from_slice(c);
                }
                b
            }
        }
    }
    pub fn id(&self) -> Id {
        hash(b"spiralcore.object.node.v1\0", &[&self.body()])
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut b = self.id().to_vec();
        b.extend_from_slice(&self.body());
        b
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let id = r.id()?;
        let ty = r.u8()?;
        let scale = r.u8()?;
        let node = match (ty, scale) {
            (0, 0) => Self::Leaf(r.take(32)?.try_into().map_err(|_| Error::Malformed)?),
            (1, 1..=13) => Self::Parent {
                scale,
                children: [r.id()?, r.id()?, r.id()?],
            },
            _ => return Err(Error::Malformed),
        };
        r.end()?;
        if node.id() != id {
            return Err(Error::Integrity);
        }
        Ok(node)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub len: u64,
    pub object: Id,
    pub roots: Vec<(u8, Id)>,
}
impl Manifest {
    pub fn encode(&self) -> Vec<u8> {
        let mut b = 1u16.to_le_bytes().to_vec();
        b.extend_from_slice(&self.len.to_le_bytes());
        b.extend_from_slice(&self.object);
        b.push(self.roots.len() as u8);
        for (scale, id) in &self.roots {
            b.push(*scale);
            b.extend_from_slice(id);
        }
        b
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > 1024 {
            return Err(Error::Limit);
        }
        let mut r = Reader::new(b);
        if r.u16()? != 1 {
            return Err(Error::Unsupported);
        }
        let len = r.u64()?;
        let object = r.id()?;
        let n = r.u8()? as usize;
        if n > 28 {
            return Err(Error::Limit);
        }
        let mut roots = Vec::with_capacity(n);
        for _ in 0..n {
            roots.push((r.u8()?, r.id()?));
        }
        r.end()?;
        let m = Self { len, object, roots };
        m.validate()?;
        Ok(m)
    }
    pub fn id(&self) -> Id {
        hash(b"spiralcore.object.manifest.v1\0", &[&self.encode()])
    }
    pub fn validate(&self) -> Result<()> {
        if self.len > MAX_OBJECT || self.roots.len() > 28 {
            return Err(Error::Limit);
        }
        let mut n = self.len.div_ceil(32);
        let mut scales = Vec::new();
        let mut scale = 0;
        while n > 0 {
            for _ in 0..n % 3 {
                scales.push(scale);
            }
            n /= 3;
            scale += 1;
        }
        scales.reverse();
        if scales != self.roots.iter().map(|p| p.0).collect::<Vec<_>>() {
            return Err(Error::Malformed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub manifest: Manifest,
    pub nodes: BTreeMap<Id, Node>,
}

// Fold-local routing memo, never a content identity or an authorization source.
// Its owning canonical map is append-only for the entire memo lifetime. Complete
// Node equality is required on every hit; collisions merely replace a slot.
// The fixed transient storage fits inside the endpoint's fold scratch allowance.
struct FoldMemo {
    slots: [Option<(Node, Id)>; 128],
    #[cfg(test)]
    hashes: usize,
}
impl FoldMemo {
    fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            #[cfg(test)]
            hashes: 0,
        }
    }
    fn slot(node: &Node) -> usize {
        fn mix(state: &mut u64, bytes: &[u8; 32]) {
            for word in bytes.chunks_exact(8) {
                let value = u64::from_le_bytes(word.try_into().expect("eight-byte chunk"));
                *state = (*state ^ value)
                    .rotate_left(13)
                    .wrapping_mul(0x9e3779b97f4a7c15);
            }
        }
        let mut state = 0x243f6a8885a308d3 ^ node.scale() as u64;
        match node {
            Node::Leaf(bytes) => mix(&mut state, bytes),
            Node::Parent { children, .. } => {
                for child in children {
                    mix(&mut state, child)
                }
            }
        }
        ((state ^ (state >> 32)) as usize) & 127
    }
    fn insert(
        &mut self,
        nodes: &mut BTreeMap<Id, Node>,
        node: Node,
        max_nodes: usize,
    ) -> Result<Id> {
        let slot = Self::slot(&node);
        if let Some((old, id)) = &self.slots[slot] {
            if old == &node {
                return Ok(*id);
            }
        }
        #[cfg(test)]
        {
            self.hashes += 1;
        }
        let id = node.id();
        let len = nodes.len();
        match nodes.entry(id) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                if entry.get() != &node {
                    return Err(Error::Integrity);
                }
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                if len >= max_nodes {
                    return Err(Error::Limit);
                }
                entry.insert(node.clone());
            }
        }
        self.slots[slot] = Some((node, id));
        Ok(id)
    }
}
impl Graph {
    /// Construct the canonical v1 ternary forest directly under full SHA-256 IDs.
    pub fn fold(bytes: &[u8], max_nodes: usize) -> Result<Self> {
        if bytes.len() as u64 > MAX_OBJECT {
            return Err(Error::Limit);
        }
        // Preserve the legacy API's overflow rejection without allocating from
        // this budget. Only newly encountered unique nodes consume the budget.
        max_nodes.checked_mul(49).ok_or(Error::Limit)?;
        let mut nodes = BTreeMap::new();
        let mut roots: Vec<(u8, Id)> = Vec::new();
        let mut memo = FoldMemo::new();
        for chunk in bytes.chunks(32) {
            let mut leaf = [0; 32];
            leaf[..chunk.len()].copy_from_slice(chunk);
            roots.push((0, memo.insert(&mut nodes, Node::Leaf(leaf), max_nodes)?));
            while roots.len() >= 3 {
                let start = roots.len() - 3;
                let scale = roots[start].0;
                if roots[start + 1].0 != scale || roots[start + 2].0 != scale {
                    break;
                }
                let children = [roots[start].1, roots[start + 1].1, roots[start + 2].1];
                // MAX_OBJECT bounds the tree to scale 13 and the carry stack
                // to 28 roots, independent of how many nodes deduplicate.
                let scale = scale + 1;
                let id = memo.insert(&mut nodes, Node::Parent { scale, children }, max_nodes)?;
                roots.truncate(start);
                roots.push((scale, id));
            }
        }
        Ok(Self {
            manifest: Manifest {
                len: bytes.len() as u64,
                object: object_id(bytes),
                roots,
            },
            nodes,
        })
    }
    pub fn rebuild(&self) -> Result<Vec<u8>> {
        rebuild(&self.manifest, &self.nodes)
    }
    pub fn delta(&self, base: &Graph) -> Vec<Node> {
        self.nodes
            .iter()
            .filter(|(id, _)| !base.nodes.contains_key(*id))
            .map(|(_, n)| n.clone())
            .collect()
    }
}
pub fn work_bound(len: u64) -> usize {
    ((len.div_ceil(32) * 3).div_ceil(2)) as usize
}

pub fn missing(manifest: &Manifest, nodes: &BTreeMap<Id, Node>) -> Result<Vec<Id>> {
    manifest.validate()?;
    let mut seen = BTreeSet::new();
    let mut absent = BTreeSet::new();
    let mut stack = manifest.roots.clone();
    let mut visits = 0;
    while let Some((scale, id)) = stack.pop() {
        visits += 1;
        if visits > work_bound(manifest.len) {
            return Err(Error::Limit);
        }
        let Some(node) = nodes.get(&id) else {
            absent.insert(id);
            if absent.len() > 1024 {
                return Err(Error::Limit);
            }
            continue;
        };
        if node.scale() != scale || node.id() != id || matches!(node, Node::Parent { scale: 0, .. })
        {
            return Err(Error::Integrity);
        }
        if !seen.insert(id) {
            continue;
        }
        if let Node::Parent { children, .. } = node {
            for c in children {
                stack.push((scale - 1, *c));
            }
        }
    }
    Ok(absent.into_iter().collect())
}
pub fn rebuild(manifest: &Manifest, nodes: &BTreeMap<Id, Node>) -> Result<Vec<u8>> {
    manifest.validate()?;
    let mut out = Vec::new();
    out.try_reserve_exact(manifest.len as usize)
        .map_err(|_| Error::Limit)?;
    let mut stack = manifest.roots.iter().rev().copied().collect::<Vec<_>>();
    let mut visits = 0;
    while let Some((scale, id)) = stack.pop() {
        visits += 1;
        if visits > work_bound(manifest.len) {
            return Err(Error::Limit);
        }
        let node = nodes.get(&id).ok_or(Error::Missing)?;
        if node.scale() != scale || node.id() != id || matches!(node, Node::Parent { scale: 0, .. })
        {
            return Err(Error::Integrity);
        }
        match node {
            Node::Leaf(data) => {
                let remaining = (manifest.len as usize)
                    .checked_sub(out.len())
                    .ok_or(Error::Integrity)?;
                let n = remaining.min(32);
                if data[n..].iter().any(|b| *b != 0) {
                    return Err(Error::Integrity);
                }
                out.extend_from_slice(&data[..n]);
            }
            Node::Parent { children, .. } => {
                for c in children.iter().rev() {
                    stack.push((scale - 1, *c));
                }
            }
        }
    }
    if out.len() != manifest.len as usize || object_id(&out) != manifest.object {
        return Err(Error::Integrity);
    }
    Ok(out)
}

#[cfg(test)]
mod fold_memo_tests {
    use super::*;
    #[test]
    fn hot_memo_reuses_only_exact_nodes_and_preserves_unique_budget() {
        let mut memo = FoldMemo::new();
        let mut nodes = BTreeMap::new();
        let a = Node::Leaf([1; 32]);
        let b = Node::Leaf([2; 32]);
        let id = memo.insert(&mut nodes, a.clone(), 1).unwrap();
        assert_eq!(memo.hashes, 1);
        assert_eq!(memo.insert(&mut nodes, a.clone(), 1).unwrap(), id);
        assert_eq!(memo.hashes, 1);
        assert_eq!(memo.insert(&mut nodes, b.clone(), 1), Err(Error::Limit));
        // Force a routing-index collision: only complete node equality permits reuse.
        memo.slots[FoldMemo::slot(&b)] = Some((a, id));
        assert_eq!(memo.insert(&mut nodes, b.clone(), 2).unwrap(), b.id());
        assert_eq!(nodes.len(), 2);
    }
    #[test]
    fn rejected_records_never_enter_the_memo() {
        let mut memo = FoldMemo::new();
        let mut nodes = BTreeMap::new();
        let node = Node::Leaf([3; 32]);
        assert_eq!(memo.insert(&mut nodes, node.clone(), 0), Err(Error::Limit));
        assert!(memo.slots.iter().all(Option::is_none));
        nodes.insert(node.id(), Node::Leaf([4; 32]));
        assert_eq!(memo.insert(&mut nodes, node, 1), Err(Error::Integrity));
        assert!(memo.slots.iter().all(Option::is_none));
    }
    #[test]
    fn parent_memo_is_exact_and_has_a_fixed_small_allocation_bound() {
        assert!(std::mem::size_of::<FoldMemo>() <= 20 * 1024);
        let mut memo = FoldMemo::new();
        let mut nodes = BTreeMap::new();
        let a = Node::Parent {
            scale: 1,
            children: [[1; 32], [2; 32], [3; 32]],
        };
        let id = memo.insert(&mut nodes, a.clone(), 3).unwrap();
        assert_eq!(memo.insert(&mut nodes, a, 3).unwrap(), id);
        assert_eq!(memo.hashes, 1);
        for b in [
            Node::Parent {
                scale: 1,
                children: [[3; 32], [2; 32], [1; 32]],
            },
            Node::Parent {
                scale: 2,
                children: [[1; 32], [2; 32], [3; 32]],
            },
        ] {
            assert_ne!(memo.insert(&mut nodes, b, 3).unwrap(), id);
        }
        assert_eq!(nodes.len(), 3);
        assert_eq!(memo.hashes, 3);
    }
}
