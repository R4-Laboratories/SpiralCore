//! Synchronous bounded sender. The exchange implementation owns transport authentication.
use crate::{
    dag::{Graph, Node},
    prepared::PreparedDelta,
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
    if matches!(
        mode,
        Mode::SparseV1
            | Mode::ZstdPrefixV1
            | Mode::RuntimeV1
            | Mode::SchemaV1
            | Mode::ZstdDictionaryV1
            | Mode::SchemaZstdV1
    ) {
        return Err(Error::Unsupported);
    }
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

/// WAN experiment object ceiling. This does not change the legacy sender's 64 MiB ceiling.
pub const MAX_BATCHED_OBJECT: usize = 16 * 1024 * 1024;
/// SCWBAT01 envelope limits, including its header and per-frame length words.
pub const MAX_BATCH_FRAMES: usize = 512;
pub const MAX_BATCH_BYTES: usize = 20 * 1024 * 1024;
const BATCH_HEADER_BYTES: usize = 12;
const FRAME_LENGTH_BYTES: usize = 4;

enum BatchOutcome {
    Continue,
    Published,
    Need(Vec<Id>),
}
struct BatchSender<F> {
    exchange: F,
    transfer: Id,
    next: u64,
    mode: Mode,
    has_base: bool,
    accepted: u64,
    accepted_upper: u64,
    stats: Stats,
}
impl<F: FnMut(&[Frame]) -> Result<Vec<Frame>>> BatchSender<F> {
    fn frame(&mut self, kind: u8, body: Vec<u8>) -> Result<Frame> {
        if self.next >= 4096 {
            return Err(Error::Limit);
        }
        let frame = Frame {
            kind,
            transfer: self.transfer,
            seq: self.next,
            body,
        };
        self.next += 1;
        Ok(frame)
    }
    fn upload(&mut self, frames: &[Frame]) -> Result<BatchOutcome> {
        if frames.is_empty() || frames.len() > MAX_BATCH_FRAMES {
            return Err(Error::Limit);
        }
        let mut bytes = BATCH_HEADER_BYTES;
        // Count every uploaded request before examining the reply prefix. A Begin cache hit may
        // stop the receiver, but cannot undo bytes already sent in this HTTP request body.
        for frame in frames {
            let length = frame.encode()?.len();
            bytes = bytes
                .checked_add(FRAME_LENGTH_BYTES + length)
                .ok_or(Error::Limit)?;
            if bytes > MAX_BATCH_BYTES {
                return Err(Error::Limit);
            }
            self.stats.request_bytes += length as u64;
            self.stats.requests += 1;
            if frame.kind == 2 {
                self.stats.data_bytes +=
                    frame.body.len().checked_sub(5).ok_or(Error::Malformed)? as u64;
                if self.mode == Mode::Dag {
                    let count = frame.body.get(5..7).ok_or(Error::Malformed)?;
                    self.stats.records_sent +=
                        u16::from_le_bytes(count.try_into().map_err(|_| Error::Malformed)?)
                            as usize;
                }
            }
        }
        let replies = (self.exchange)(frames)?;
        if replies.is_empty() || replies.len() > frames.len() {
            return Err(Error::Integrity);
        }
        for reply in &replies {
            self.stats.response_bytes += reply.encode()?.len() as u64;
        }
        let reply_count = replies.len();
        for (index, (request, reply)) in frames.iter().zip(replies).enumerate() {
            if reply.transfer != request.transfer || reply.seq != request.seq {
                return Err(Error::Integrity);
            }
            let terminal = index + 1 == reply_count;
            if reply.kind == 127 {
                if !terminal {
                    return Err(Error::Integrity);
                }
                return Err(protocol_error(&reply.body)?);
            }
            if reply.kind == 103 {
                if !matches!(request.kind, 1 | 3) || !terminal {
                    return Err(Error::Integrity);
                }
                check_published(&reply, self.stats.object)?;
                return Ok(BatchOutcome::Published);
            }
            match request.kind {
                1 => {
                    if reply.kind != 101 {
                        return Err(Error::Integrity);
                    }
                    let mut reader = Reader::new(&reply.body);
                    if reader.u8()? != self.mode as u8
                        || reader.u8()? != u8::from(self.has_base)
                        || reader.u64()? != request.seq + 1
                    {
                        return Err(Error::Integrity);
                    }
                    reader.end()?;
                }
                2 => {
                    if reply.kind != 102 {
                        return Err(Error::Integrity);
                    }
                    let declared = request.body.get(1..5).ok_or(Error::Malformed)?;
                    self.accepted_upper = self
                        .accepted_upper
                        .checked_add(u32::from_le_bytes(
                            declared.try_into().map_err(|_| Error::Malformed)?,
                        ) as u64)
                        .ok_or(Error::Limit)?;
                    let mut reader = Reader::new(&reply.body);
                    let accepted = reader.u64()?;
                    reader.end()?;
                    if accepted < self.accepted
                        || accepted > self.accepted_upper
                        || (self.mode != Mode::Dag && accepted != self.accepted_upper)
                    {
                        return Err(Error::Integrity);
                    }
                    self.accepted = accepted;
                }
                3 => {
                    if reply.kind != 104
                        || self.mode != Mode::Dag
                        || !terminal
                        || reply_count != frames.len()
                    {
                        return Err(Error::Integrity);
                    }
                    return Ok(BatchOutcome::Need(need_ids(&reply.body)?));
                }
                _ => return Err(Error::Integrity),
            }
        }
        // Only terminal Published/Error can justify an omitted suffix. Ready/Data receipts cannot.
        if reply_count != frames.len() {
            return Err(Error::Integrity);
        }
        Ok(BatchOutcome::Continue)
    }
    fn plan(&mut self, frames: &[Frame]) -> Result<BatchOutcome> {
        let mut offset = 0;
        while offset < frames.len() {
            let mut end = offset;
            let mut bytes = BATCH_HEADER_BYTES;
            while end < frames.len() && end - offset < MAX_BATCH_FRAMES {
                let length = FRAME_LENGTH_BYTES + frames[end].encode()?.len();
                if bytes.checked_add(length).ok_or(Error::Limit)? > MAX_BATCH_BYTES {
                    break;
                }
                bytes += length;
                end += 1;
            }
            if end == offset {
                return Err(Error::Limit);
            }
            match self.upload(&frames[offset..end])? {
                BatchOutcome::Continue => offset = end,
                terminal => return Ok(terminal),
            }
        }
        Ok(BatchOutcome::Continue)
    }
    fn records(&mut self, nodes: &[Node], frames: &mut Vec<Frame>) -> Result<()> {
        for batch in nodes.chunks(480) {
            let mut body = vec![2, 0, 0, 0, 0];
            body.extend_from_slice(&(batch.len() as u16).to_le_bytes());
            for node in batch {
                body.extend_from_slice(&node.encode());
            }
            let encoded = u32::try_from(body.len() - 7).map_err(|_| Error::Limit)?;
            body[1..5].copy_from_slice(&encoded.to_le_bytes());
            frames.push(self.frame(2, body)?);
        }
        Ok(())
    }
}
fn protocol_error(body: &[u8]) -> Result<Error> {
    let mut reader = Reader::new(body);
    let code = reader.u16()?;
    reader.end()?;
    Ok(match code {
        1 => Error::Malformed,
        2 => Error::Unauthorized,
        3 => Error::Limit,
        4 => Error::Integrity,
        5 => Error::Missing,
        6 => Error::Sequence,
        7 => Error::Unknown,
        8 => Error::Expired,
        9 => Error::Storage,
        10 => Error::Unsupported,
        _ => return Err(Error::Malformed),
    })
}
fn need_ids(body: &[u8]) -> Result<Vec<Id>> {
    let mut reader = Reader::new(body);
    let count = reader.u16()? as usize;
    if count == 0 || count > 1024 {
        return Err(Error::Malformed);
    }
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(reader.id()?);
    }
    reader.end()?;
    Ok(ids)
}
fn repair_nodes(graph: &Graph, pending: Vec<Id>) -> Result<Vec<Node>> {
    repair_nodes_from(pending, graph.nodes.len(), |id| graph.nodes.get(id))
}
fn repair_nodes_from<'a>(
    mut pending: Vec<Id>,
    max_nodes: usize,
    lookup: impl Fn(&Id) -> Option<&'a Node>,
) -> Result<Vec<Node>> {
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = lookup(&id).ok_or(Error::Integrity)?;
        if let Node::Parent { children, .. } = node {
            pending.extend_from_slice(children);
        }
        if seen.len() > max_nodes {
            return Err(Error::Limit);
        }
    }
    seen.into_iter()
        .map(|id| lookup(&id).cloned().ok_or(Error::Integrity))
        .collect()
}

/// Transfer ordered batches through a WAN transport adapter without changing SCMOBJ01 frames.
///
/// Each batch fits 512 frames and a 20 MiB SCWBAT01 body, including envelope/length words. Objects
/// are limited to 16 MiB for this campaign; DAG construction retains its 262,144-node ceiling.
/// `warm_probe` sends Begin alone, avoiding a data upload on an exact-object cache hit. Without it,
/// Begin/Data/Finish are uploaded together when they fit, even if Begin later returns Published.
/// Request statistics count all uploaded frames; response statistics count only actual replies.
/// Envelope/TLS/HTTP bytes belong to the adapter's separate measurements.
pub fn transfer_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    bytes: &[u8],
    mode: Mode,
    base: Option<&Graph>,
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if matches!(
        mode,
        Mode::SparseV1
            | Mode::ZstdPrefixV1
            | Mode::RuntimeV1
            | Mode::SchemaV1
            | Mode::ZstdDictionaryV1
            | Mode::SchemaZstdV1
    ) {
        return Err(Error::Unsupported);
    }
    if bytes.len() > MAX_BATCHED_OBJECT
        || base
            .is_some_and(|g| g.manifest.len > MAX_BATCHED_OBJECT as u64 || g.nodes.len() > 262144)
    {
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
    let offer = if let Some(graph) = &graph {
        Offer {
            mode,
            len: graph.manifest.len,
            object: graph.manifest.object,
            manifest: Some(graph.manifest.clone()),
            base: base.map(|g| g.manifest.id()),
        }
    } else {
        Offer::cold(bytes, mode)
    };
    let payload = match &graph {
        Some(graph) => BatchedPayload::Graph(graph, base),
        None => BatchedPayload::Bytes(bytes),
    };
    send_batched(offer, payload, transfer_id, warm_probe, exchange, start)
}

/// Send a locally prepared, immutable same-length DAG update using the existing
/// DAG wire format. Exact repeats use a raw identity offer; `warm_probe` retains
/// its usual control over whether Begin is uploaded separately.
///
/// Preparation time is outside these sender statistics. Missing-base errors are
/// returned without retrying a different offer under the same transfer ID.
pub fn transfer_prepared_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    target: &PreparedDelta,
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if target.bytes().len() > MAX_BATCHED_OBJECT || target.node_count() > 262144 {
        return Err(Error::Limit);
    }
    if target.is_repeat() {
        return transfer_batched(
            target.bytes(),
            Mode::Raw,
            None,
            transfer_id,
            warm_probe,
            exchange,
        );
    }
    let offer = Offer {
        mode: Mode::Dag,
        len: target.manifest().len,
        object: target.manifest().object,
        manifest: Some(target.manifest().clone()),
        base: Some(target.base_manifest().id()),
    };
    send_batched(
        offer,
        BatchedPayload::Prepared(target),
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

/// Transfer a bounded COPY/LITERAL recipe against caller-owned local base bytes.
///
/// The base identity is computed from these exact bytes. The receiving endpoint
/// must authorize and possess that base in the same context. An inefficient
/// recipe falls back to Raw before any exchange; a missing remote base is an
/// error and never triggers another offer under the same transfer ID.
/// Exact repeats use Raw identity probes according to `warm_probe`.
pub fn transfer_sparse_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    base: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if base.len() > MAX_BATCHED_OBJECT || target.len() > MAX_BATCHED_OBJECT {
        return Err(Error::Limit);
    }
    let recipe = if base == target {
        None
    } else {
        crate::delta::encode(base, target)?
    };
    let Some(recipe) = recipe else {
        let mut stats =
            transfer_batched(target, Mode::Raw, None, transfer_id, warm_probe, exchange)?;
        stats.elapsed_micros = start.elapsed().as_micros();
        return Ok(stats);
    };
    let offer = Offer {
        mode: Mode::SparseV1,
        len: target.len() as u64,
        object: crate::object_id(target),
        manifest: None,
        base: Some(crate::object_id(base)),
    };
    send_batched(
        offer,
        BatchedPayload::Sparse(recipe),
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

/// Transfer a whole-base zstd prefix frame with the version-one bounded profile.
/// Encoding and fallback selection are included in elapsed sender time. A patch
/// is selected only when complete object-protocol traffic is cheaper than Raw;
/// a missing remote base is returned without retry or changing the offer.
pub fn transfer_zstd_prefix_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    base: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if base.len() > MAX_BATCHED_OBJECT || target.len() > MAX_BATCHED_OBJECT {
        return Err(Error::Limit);
    }
    let encoded = if base == target {
        Vec::new()
    } else {
        crate::zstd_prefix::encode(base, target)?
    };
    // Each Data contributes its frame (56 + encoding/length/version), an
    // Accepted frame (64), and two batch length prefixes (8). Prefix Begin has
    // 32 more bytes for the required base ID. Common Begin/Finish and batch
    // headers cancel. Both complete transfers fit one batch at this ceiling.
    let prefix_cost = encoded.len() + encoded.len().div_ceil(MAX_CHUNK) * 134 + 32;
    let raw_cost = target.len() + target.len().div_ceil(MAX_CHUNK) * 133;
    if encoded.is_empty() || prefix_cost >= raw_cost {
        drop(encoded);
        let mut stats =
            transfer_batched(target, Mode::Raw, None, transfer_id, warm_probe, exchange)?;
        stats.elapsed_micros = start.elapsed().as_micros();
        return Ok(stats);
    }
    let offer = Offer {
        mode: Mode::ZstdPrefixV1,
        len: target.len() as u64,
        object: crate::object_id(target),
        manifest: None,
        base: Some(crate::object_id(base)),
    };
    let frames = crate::zstd_prefix::chunks(&encoded)?;
    drop(encoded);
    send_batched(
        offer,
        BatchedPayload::Prefix(frames),
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

/// Transfer exact bytes using a frozen fitted schema, or Raw if the full
/// successful-miss body cost would not improve. Fit/seed explicitly beforehand.
pub fn transfer_schema_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    artifact: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    transfer_fitted_batched(
        Mode::SchemaV1,
        artifact,
        target,
        transfer_id,
        warm_probe,
        exchange,
    )
}

/// Like `transfer_schema_batched`, with a genuinely trained zstd dictionary.
/// Context loading, artifact/target hashing and discarded candidate work are
/// included in elapsed time. A remote artifact miss never changes the offer.
pub fn transfer_zstd_dictionary_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    artifact: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    transfer_fitted_batched(
        Mode::ZstdDictionaryV1,
        artifact,
        target,
        transfer_id,
        warm_probe,
        exchange,
    )
}

/// Compress the exact schema value/literal stream before transfer.
pub fn transfer_schema_zstd_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    artifact: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    transfer_fitted_batched(
        Mode::SchemaZstdV1,
        artifact,
        target,
        transfer_id,
        warm_probe,
        exchange,
    )
}

/// Reuse a previously prepared schema and zstd compression context. Construction
/// belongs to the caller's setup accounting; every encode/hash/send stays timed.
pub fn transfer_prepared_schema_zstd_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    prepared: &mut crate::schema_zstd::PreparedEncoder,
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    let encoded = prepared.encode(target)?;
    send_fitted(
        FittedCandidate {
            mode: Mode::SchemaZstdV1,
            encoded,
            artifact_id: || prepared.artifact_id(),
        },
        target,
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

/// Reuse a loaded trained dictionary and compression context under the same
/// exact-byte/fallback contract as the unprepared dictionary sender.
pub fn transfer_prepared_zstd_dictionary_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    prepared: &mut crate::zstd_dictionary::PreparedEncoder,
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    let encoded = prepared.encode(target)?;
    send_fitted(
        FittedCandidate {
            mode: Mode::ZstdDictionaryV1,
            encoded,
            artifact_id: || prepared.artifact_id(),
        },
        target,
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

struct FittedCandidate<I> {
    mode: Mode,
    encoded: Vec<u8>,
    artifact_id: I,
}

fn transfer_fitted_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    mode: Mode,
    artifact: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<Stats> {
    let start = Instant::now();
    if artifact.len() > 64 * 1024 || target.len() > MAX_BATCHED_OBJECT {
        return Err(Error::Limit);
    }
    let encoded = match mode {
        Mode::SchemaV1 => crate::schema::encode(artifact, target)?,
        Mode::ZstdDictionaryV1 => crate::zstd_dictionary::encode(artifact, target)?,
        Mode::SchemaZstdV1 => crate::schema_zstd::encode(artifact, target)?,
        _ => return Err(Error::Unsupported),
    };
    send_fitted(
        FittedCandidate {
            mode,
            encoded,
            artifact_id: || crate::object_id(artifact),
        },
        target,
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

fn send_fitted<I: FnOnce() -> Id, F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    candidate: FittedCandidate<I>,
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
    start: Instant,
) -> Result<Stats> {
    let FittedCandidate {
        mode,
        encoded,
        artifact_id,
    } = candidate;
    let cap = crate::fitted_transfer::encoded_cap(mode, target.len())?;
    if encoded.len() > cap {
        return Err(Error::Limit);
    }
    let candidate_cost = encoded.len() + encoded.len().div_ceil(MAX_CHUNK) * 134 + 32;
    let raw_cost = target.len() + target.len().div_ceil(MAX_CHUNK) * 133;
    if candidate_cost >= raw_cost {
        drop(encoded);
        let mut stats =
            transfer_batched(target, Mode::Raw, None, transfer_id, warm_probe, exchange)?;
        stats.elapsed_micros = start.elapsed().as_micros();
        return Ok(stats);
    }
    let offer = Offer {
        mode,
        len: target.len() as u64,
        object: crate::object_id(target),
        manifest: None,
        base: Some(artifact_id()),
    };
    let bodies = crate::fitted_transfer::chunks(mode, &encoded)?;
    drop(encoded);
    send_batched(
        offer,
        BatchedPayload::Encoded(bodies),
        transfer_id,
        warm_probe,
        exchange,
        start,
    )
}

/// Transfer outcome and the local pre-Begin selection receipt.
#[derive(Debug, Clone)]
pub struct AdaptiveStats {
    pub transfer: Stats,
    pub selection: crate::selection::Selection,
}

/// Byte-first transfer with the shared ternary/Kuramoto/CA runtime as an active
/// candidate. A recipe is explicit application knowledge, never guessed by an
/// unbounded search. All candidate computation is included in elapsed time.
/// The checkpoint must already be authorized and verified at the receiver if
/// the runtime candidate wins. Missing state never triggers an implicit reoffer.
pub fn transfer_runtime_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    checkpoint: &[u8],
    target: &[u8],
    recipe: &crate::shared_runtime::Recipe,
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<AdaptiveStats> {
    transfer_runtime_candidate_batched(
        checkpoint,
        target,
        transfer_id,
        warm_probe,
        exchange,
        || crate::shared_transfer::prepare(checkpoint, target, recipe),
        |needs_base| {
            (
                crate::object_id(target),
                needs_base.then(|| crate::object_id(checkpoint)),
            )
        },
    )
}

/// Reuse an immutable sender prediction while retaining normal selection,
/// receiver authorization, replay and exact durable publication. Elapsed time
/// covers this send; initial prediction preparation is a separate caller cost.
pub fn transfer_prepared_runtime_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    prediction: &crate::shared_transfer::PreparedPrediction,
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<AdaptiveStats> {
    transfer_runtime_candidate_batched(
        prediction.checkpoint(),
        target,
        transfer_id,
        warm_probe,
        exchange,
        || prediction.prepare(target),
        |needs_base| {
            (
                prediction.object_id_for(target),
                needs_base.then(|| prediction.checkpoint_id()),
            )
        },
    )
}

fn transfer_runtime_candidate_batched<F, P, I>(
    checkpoint: &[u8],
    target: &[u8],
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
    runtime: P,
    identities: I,
) -> Result<AdaptiveStats>
where
    F: FnMut(&[Frame]) -> Result<Vec<Frame>>,
    P: FnOnce() -> Result<Option<Vec<Vec<u8>>>>,
    I: FnOnce(bool) -> (Id, Option<Id>),
{
    let start = Instant::now();
    if checkpoint.len() > crate::shared_runtime::MAX_CHECKPOINT {
        return Err(Error::Limit);
    }
    let prepared = crate::selection::prepare_runtime(target, checkpoint, warm_probe, runtime)?;
    let mode = prepared.selection.selected;
    let (object, base) = identities(matches!(
        mode,
        Mode::SparseV1
            | Mode::ZstdPrefixV1
            | Mode::RuntimeV1
            | Mode::SchemaV1
            | Mode::ZstdDictionaryV1
            | Mode::SchemaZstdV1
    ));
    let offer = Offer {
        mode,
        len: target.len() as u64,
        object,
        manifest: None,
        base,
    };
    let payload = match prepared.bodies {
        Some(bodies) => BatchedPayload::Encoded(bodies),
        None => BatchedPayload::Bytes(target),
    };
    let transfer = send_batched(offer, payload, transfer_id, warm_probe, exchange, start)?;
    Ok(AdaptiveStats {
        transfer,
        selection: prepared.selection,
    })
}

/// Select the smallest eligible successful-miss HTTP body representation before
/// Begin, then send the already encoded winner. Raw/Zstd/Sparse/Prefix are the
/// deterministic tie order. This opt-in policy minimizes bytes, not CPU/time.
///
/// A locally exact base/target repeat with `warm_probe` uses a Raw identity probe
/// without evaluating codecs; if evicted remotely it sends Raw. Base possession
/// is never authorization. Missing-base, revoked-policy and transport errors are
/// returned unchanged, without another offer or hidden retry. Elapsed time
/// includes discarded candidate work and object hashing through commit response.
pub fn transfer_adaptive_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    target: &[u8],
    base: Option<&[u8]>,
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
) -> Result<AdaptiveStats> {
    let start = Instant::now();
    let prepared = crate::selection::prepare(target, base, warm_probe)?;
    let mode = prepared.selection.selected;
    let base_id = if matches!(mode, Mode::SparseV1 | Mode::ZstdPrefixV1) {
        Some(crate::object_id(base.ok_or(Error::Integrity)?))
    } else {
        None
    };
    let offer = Offer {
        mode,
        len: target.len() as u64,
        object: crate::object_id(target),
        manifest: None,
        base: base_id,
    };
    let payload = match prepared.bodies {
        Some(bodies) => BatchedPayload::Encoded(bodies),
        None => BatchedPayload::Bytes(target),
    };
    let transfer = send_batched(offer, payload, transfer_id, warm_probe, exchange, start)?;
    Ok(AdaptiveStats {
        transfer,
        selection: prepared.selection,
    })
}

enum BatchedPayload<'a> {
    Bytes(&'a [u8]),
    Graph(&'a Graph, Option<&'a Graph>),
    Prepared(&'a PreparedDelta),
    Sparse(crate::delta::Recipe),
    Prefix(Vec<Vec<u8>>),
    Encoded(Vec<Vec<u8>>),
}

fn send_batched<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    offer: Offer,
    mut payload: BatchedPayload<'_>,
    transfer_id: Id,
    warm_probe: bool,
    exchange: F,
    start: Instant,
) -> Result<Stats> {
    let mode = offer.mode;
    let mut sender = BatchSender {
        exchange,
        transfer: transfer_id,
        next: 0,
        mode,
        has_base: offer.base.is_some(),
        accepted: 0,
        accepted_upper: 0,
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
    let begin = sender.frame(1, offer.encode()?)?;
    let mut frames = Vec::new();
    if warm_probe {
        match sender.upload(std::slice::from_ref(&begin))? {
            BatchOutcome::Published => {
                sender.stats.elapsed_micros = start.elapsed().as_micros();
                return Ok(sender.stats);
            }
            BatchOutcome::Continue => {}
            BatchOutcome::Need(_) => return Err(Error::Integrity),
        }
    } else {
        frames.push(begin);
    }
    match &mut payload {
        BatchedPayload::Graph(graph, base) => {
            let nodes = if let Some(base) = base {
                graph.delta(base)
            } else {
                graph.nodes.values().cloned().collect()
            };
            sender.records(&nodes, &mut frames)?;
        }
        BatchedPayload::Prepared(target) => {
            let nodes = target
                .new_records()
                .map(|(_, n)| n.clone())
                .collect::<Vec<_>>();
            sender.records(&nodes, &mut frames)?;
        }
        BatchedPayload::Bytes(bytes) => {
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
                frames.push(sender.frame(2, body)?);
            }
        }
        BatchedPayload::Sparse(recipe) => {
            for body in std::mem::take(&mut recipe.frames) {
                frames.push(sender.frame(2, body)?);
            }
        }
        BatchedPayload::Prefix(bodies) | BatchedPayload::Encoded(bodies) => {
            for body in std::mem::take(bodies) {
                frames.push(sender.frame(2, body)?);
            }
        }
    }
    for repair in 0..=2 {
        frames.push(sender.frame(3, vec![])?);
        match sender.plan(&frames)? {
            BatchOutcome::Published => {
                sender.stats.elapsed_micros = start.elapsed().as_micros();
                return Ok(sender.stats);
            }
            BatchOutcome::Need(ids) if repair < 2 => {
                let nodes = match &payload {
                    BatchedPayload::Graph(graph, _) => repair_nodes(graph, ids)?,
                    BatchedPayload::Prepared(target) => {
                        repair_nodes_from(ids, target.node_count(), |id| target.node(id))?
                    }
                    BatchedPayload::Bytes(_)
                    | BatchedPayload::Sparse(_)
                    | BatchedPayload::Prefix(_)
                    | BatchedPayload::Encoded(_) => return Err(Error::Integrity),
                };
                frames.clear();
                sender.records(&nodes, &mut frames)?;
            }
            BatchOutcome::Need(_) => return Err(Error::Missing),
            BatchOutcome::Continue => return Err(Error::Integrity),
        }
    }
    Err(Error::Missing)
}
