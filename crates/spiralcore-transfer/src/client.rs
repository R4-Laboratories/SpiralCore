//! Synchronous bounded sender. The exchange implementation owns transport authentication.
use crate::{
    dag::{Graph, Node},
    protocol::{Frame, Mode, Offer, Reader},
    Error, Id, Result, MAX_CHUNK, MAX_OBJECT,
};
use std::{collections::BTreeSet, time::Instant};

#[derive(Debug, Clone)]
pub struct Stats {
    pub object: Id,
    pub mode: Mode,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub data_bytes: u64,
    pub records_sent: usize,
    pub requests: usize,
    pub elapsed_micros: u128,
}
struct Sender<F> {
    exchange: F,
    id: Id,
    seq: u64,
    stats: Stats,
}
impl<F: FnMut(&Frame) -> Result<Frame>> Sender<F> {
    fn call(&mut self, kind: u8, body: Vec<u8>) -> Result<Frame> {
        if self.seq >= 4096 {
            return Err(Error::Limit);
        }
        let f = Frame {
            kind,
            transfer: self.id,
            seq: self.seq,
            body,
        };
        self.stats.request_bytes += f.encode()?.len() as u64;
        self.stats.requests += 1;
        let reply = (self.exchange)(&f)?;
        self.stats.response_bytes += reply.encode()?.len() as u64;
        if reply.transfer != self.id || reply.seq != self.seq {
            return Err(Error::Integrity);
        }
        if reply.kind == 127 {
            let mut r = Reader::new(&reply.body);
            let code = r.u16()?;
            r.end()?;
            return Err(match code {
                1 => Error::Malformed,
                2 => Error::Unauthorized,
                3 => Error::Limit,
                4 => Error::Integrity,
                5 => Error::Missing,
                6 => Error::Sequence,
                7 => Error::Unknown,
                8 => Error::Expired,
                9 => Error::Storage,
                _ => Error::Unsupported,
            });
        }
        self.seq += 1;
        Ok(reply)
    }
    fn data(&mut self, body: Vec<u8>) -> Result<()> {
        self.stats.data_bytes += body.len().saturating_sub(5) as u64;
        let reply = self.call(2, body)?;
        if reply.kind != 102 || reply.body.len() != 8 {
            return Err(Error::Integrity);
        }
        Ok(())
    }
    fn records(&mut self, nodes: &[Node]) -> Result<()> {
        for batch in nodes.chunks(480) {
            let encoded = batch.iter().map(Node::encode).collect::<Vec<_>>();
            let len: usize = encoded.iter().map(Vec::len).sum();
            let mut body = vec![2];
            body.extend_from_slice(&(len as u32).to_le_bytes());
            body.extend_from_slice(&(batch.len() as u16).to_le_bytes());
            for record in encoded {
                body.extend_from_slice(&record);
            }
            self.stats.records_sent += batch.len();
            self.data(body)?;
        }
        Ok(())
    }
}
pub fn transfer<F: FnMut(&Frame) -> Result<Frame>>(
    bytes: &[u8],
    mode: Mode,
    base: Option<&Graph>,
    transfer_id: Id,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if bytes.len() as u64 > MAX_OBJECT {
        return Err(Error::Limit);
    }
    if mode != Mode::Dag && base.is_some() {
        return Err(Error::Malformed);
    }
    let graph = if mode == Mode::Dag {
        Some(Graph::fold(bytes, 262144)?)
    } else {
        None
    };
    let offer = if let Some(g) = &graph {
        Offer {
            mode,
            len: g.manifest.len,
            object: g.manifest.object,
            manifest: Some(g.manifest.clone()),
            base: base.map(|g| g.manifest.id()),
        }
    } else {
        Offer::cold(bytes, mode)
    };
    let mut s = Sender {
        exchange,
        id: transfer_id,
        seq: 0,
        stats: Stats {
            object: offer.object,
            mode,
            request_bytes: 0,
            response_bytes: 0,
            data_bytes: 0,
            records_sent: 0,
            requests: 0,
            elapsed_micros: 0,
        },
    };
    let ready = s.call(1, offer.encode()?)?;
    if ready.kind == 103 {
        check_published(&ready, offer.object)?;
        s.stats.elapsed_micros = start.elapsed().as_micros();
        return Ok(s.stats);
    }
    if ready.kind != 101 {
        return Err(Error::Integrity);
    }
    let mut r = Reader::new(&ready.body);
    if r.u8()? != mode as u8 || r.u8()? != u8::from(base.is_some()) || r.u64()? != s.seq {
        return Err(Error::Integrity);
    }
    r.end()?;
    if let Some(g) = &graph {
        let nodes = if let Some(base) = base {
            g.delta(base)
        } else {
            g.nodes.values().cloned().collect()
        };
        s.records(&nodes)?;
    } else {
        for chunk in bytes.chunks(MAX_CHUNK) {
            let compressed = if mode == Mode::Zstd {
                Some(zstd::bulk::compress(chunk, 1).map_err(|_| Error::Malformed)?)
            } else {
                None
            };
            let (encoding, payload) = match &compressed {
                Some(b) if b.len() < chunk.len() => (1, b.as_slice()),
                _ => (0, chunk),
            };
            let mut body = vec![encoding];
            body.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            body.extend_from_slice(payload);
            s.data(body)?;
        }
    }
    for repair in 0..=2 {
        let reply = s.call(3, vec![])?;
        if reply.kind == 103 {
            check_published(&reply, offer.object)?;
            s.stats.elapsed_micros = start.elapsed().as_micros();
            return Ok(s.stats);
        }
        if reply.kind != 104 || repair == 2 {
            return Err(Error::Missing);
        }
        let g = graph.as_ref().ok_or(Error::Integrity)?;
        let mut r = Reader::new(&reply.body);
        let count = r.u16()? as usize;
        if count == 0 || count > 1024 {
            return Err(Error::Malformed);
        }
        let mut stack = Vec::with_capacity(count);
        for _ in 0..count {
            stack.push(r.id()?);
        }
        r.end()?;
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let n = g.nodes.get(&id).ok_or(Error::Integrity)?;
            if let Node::Parent { children, .. } = n {
                stack.extend_from_slice(children);
            }
            if seen.len() > g.nodes.len() {
                return Err(Error::Limit);
            }
        }
        let nodes = seen
            .into_iter()
            .map(|id| g.nodes[&id].clone())
            .collect::<Vec<_>>();
        s.records(&nodes)?;
    }
    Err(Error::Missing)
}
fn check_published(f: &Frame, id: Id) -> Result<()> {
    if f.kind == 103 && f.body == id {
        Ok(())
    } else {
        Err(Error::Integrity)
    }
}
