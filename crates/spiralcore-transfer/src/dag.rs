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
impl Graph {
    /// Reuse the imported exact fold, then reidentify its immutable structure under v1.
    pub fn fold(bytes: &[u8], max_nodes: usize) -> Result<Self> {
        if bytes.len() as u64 > MAX_OBJECT {
            return Err(Error::Limit);
        }
        let limits = uor_r4_object_store::Limits {
            max_object_bytes: MAX_OBJECT,
            max_records: max_nodes,
            max_record_bytes: max_nodes.checked_mul(49).ok_or(Error::Limit)?,
            max_returned_bytes: MAX_OBJECT,
            max_visits: work_bound(bytes.len() as u64),
            max_occurrences: work_bound(bytes.len() as u64),
        };
        let mut store = uor_r4_object_store::Store::new(limits);
        let folded = store.fold(bytes).map_err(core_error)?;
        if store.rebuild(&folded.manifest).map_err(core_error)?.bytes() != bytes {
            return Err(Error::Integrity);
        }
        let mut ids = BTreeMap::new();
        let mut nodes = BTreeMap::new();
        fn convert(
            old: [u8; 12],
            scale: u8,
            store: &uor_r4_object_store::Store,
            ids: &mut BTreeMap<[u8; 12], Id>,
            nodes: &mut BTreeMap<Id, Node>,
        ) -> Result<Id> {
            let raw = store.record_bytes(old).ok_or(Error::Missing)?;
            let rec = uor_r4_object_store::Record::decode(&raw).map_err(core_error)?;
            if rec.scale() != scale {
                return Err(Error::Integrity);
            }
            if let Some(id) = ids.get(&old) {
                return Ok(*id);
            }
            let node = if scale == 0 {
                Node::Leaf(rec.payload().try_into().map_err(|_| Error::Malformed)?)
            } else {
                let mut children = [[0; 32]; 3];
                for (dst, p) in children.iter_mut().zip(rec.payload().chunks_exact(12)) {
                    *dst = convert(
                        p.try_into().map_err(|_| Error::Malformed)?,
                        scale - 1,
                        store,
                        ids,
                        nodes,
                    )?;
                }
                Node::Parent { scale, children }
            };
            let id = node.id();
            nodes.insert(id, node);
            ids.insert(old, id);
            Ok(id)
        }
        let mut roots = Vec::new();
        for (id, scale) in folded.manifest.roots() {
            roots.push((*scale, convert(*id, *scale, &store, &mut ids, &mut nodes)?));
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
fn core_error(e: uor_r4_object_store::Error) -> Error {
    match e {
        uor_r4_object_store::Error::LimitExceeded(_) => Error::Limit,
        _ => Error::Integrity,
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
