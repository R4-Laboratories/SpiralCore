//! Peer replication for the exact object store: native manifest exchange plus missing-subtree fetch,
//! using the same wire message kinds as the P2P driver (20..=24).
//!
//! Hardening properties:
//! - Trusted anchor: pull_expected rejects a peer manifest that does not match the caller-supplied
//!   object identity (byte length + manifest version) before any record is fetched or imported.
//! - Dynamic batching: the request batch is derived from the store's limits, so a cold pull of a
//!   multi-megabyte object completes (the previous fixed cap could not finish a 1 MiB cold pull).
//! - Correlated responses: every returned record must have been requested, the declared count must
//!   equal the batch size, and duplicate ids are rejected.
//! - Atomic commit: records are staged in memory, the object is assembled and its native SHA-256 is
//!   verified against the manifest, and only then are the records imported into the store.
//! - Protocol hygiene: DONE is sent after commit, every message flushes, a clean EOF ends a serve
//!   session, and serve only answers ids in the requested manifest's closure with request/byte budgets.
//! - Fleet selection: MANIFEST_REQUEST takes an optional 32-byte selector (whole-object SHA-256 or
//!   manifest version); serve_many serves several objects from one store, verify reports closure
//!   completeness without fetching records or rebuilding the payload, and an unknown selector fails
//!   with Error::UnknownObject.

use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};

use crate::{Error, Limits, Manifest, NodeId, ObjectVersion, Record, RecordBackend, Result, Store};

const MANIFEST_REQUEST: u8 = 20;
const MANIFEST: u8 = 21;
const FETCH: u8 = 22;
const NODES: u8 = 23;
const DONE: u8 = 24;
const ERROR: u8 = 25;
const MAX_MESSAGE: usize = 64 * 1024 * 1024;
const DESIRED_BATCH: usize = 4096;

/// Independent peer budgets. Serialized record bytes are not a resident-memory estimate.
/// Legacy APIs derive conservative defaults from `Store::limits`; configured APIs may tighten each
/// budget independently without changing record, manifest or frame encoding.
#[derive(Debug, Clone)]
pub struct PeerLimits {
    pub max_message_bytes: usize,
    pub max_staged_records: usize,
    pub max_staged_bytes: usize,
    /// Cumulative node visits across discovery, decoding and strict reconstruction.
    pub max_work: usize,
    /// MANIFEST_REQUEST and FETCH frames; terminal DONE is not a request.
    pub max_requests: usize,
    pub max_in_flight_response_bytes: usize,
}
impl PeerLimits {
    pub fn for_store(limits: &Limits) -> Self {
        Self {
            max_message_bytes: MAX_MESSAGE.min(limits.max_record_bytes),
            max_staged_records: limits.max_records,
            max_staged_bytes: limits.max_record_bytes,
            max_work: limits.max_visits.saturating_mul(64),
            max_requests: limits.max_visits,
            max_in_flight_response_bytes: 8 * 1024 * 1024,
        }
    }
}
impl Default for PeerLimits {
    fn default() -> Self {
        Self::for_store(&Limits::default())
    }
}

/// Pull with independent message, staging and cumulative-work budgets.
pub fn pull_selected_with_limits<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    peer_limits: &PeerLimits,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_one(
        store,
        expected,
        selector,
        PullOptions {
            send_done: true,
            batch: None,
            window: DEFAULT_FETCH_WINDOW,
            peer_limits: Some(peer_limits.clone()),
        },
        reader,
        writer,
    )
}

/// The result of one verify check in a batched series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    Verified { missing: usize },
    Stale { missing: usize },
    Unknown,
    Mismatch,
}

/// One requested check for verify_series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyRequest {
    pub expected: Option<ExpectedObject>,
    pub selector: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyStats {
    pub missing: usize,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PullStats {
    pub rounds: u64,
    pub nodes_requested: u64,
    pub record_bytes: u64,
    pub verified: bool,
}

/// The object identity a caller already trusts (from a registry, manifest pin or local expectation).
/// Any combination of pins may be supplied; at least one must be set for an anchored pull. bytes is
/// the native object length, version is the manifest version (SHA-256 of the canonical UOROBJ01
/// manifest), and whole_sha256 is the SHA-256 of the object payload - the digest an OCI descriptor or
/// artifact store publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ExpectedObject {
    pub bytes: Option<u64>,
    pub version: Option<ObjectVersion>,
    pub whole_sha256: Option<[u8; 32]>,
}

impl ExpectedObject {
    pub fn from_manifest(manifest: &Manifest) -> ExpectedObject {
        ExpectedObject {
            bytes: Some(manifest.byte_len()),
            version: Some(manifest.version()),
            whole_sha256: None,
        }
    }

    pub fn from_sha256(sha: [u8; 32]) -> ExpectedObject {
        ExpectedObject {
            bytes: None,
            version: None,
            whole_sha256: Some(sha),
        }
    }

    pub fn with_bytes(mut self, bytes: u64) -> ExpectedObject {
        self.bytes = Some(bytes);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_none() && self.version.is_none() && self.whole_sha256.is_none()
    }

    fn matches(&self, manifest: &Manifest) -> bool {
        if self.is_empty() {
            return false;
        }
        if let Some(bytes) = self.bytes {
            if bytes != manifest.byte_len() {
                return false;
            }
        }
        if let Some(version) = self.version {
            if version != manifest.version() {
                return false;
            }
        }
        if let Some(sha) = self.whole_sha256 {
            if sha != manifest.whole_sha256() {
                return false;
            }
        }
        true
    }
}

fn write_msg<W: Write>(writer: &mut W, kind: u8, body: &[u8]) -> Result<()> {
    write_msg_limited(writer, kind, body, MAX_MESSAGE)
}
fn write_msg_limited<W: Write>(writer: &mut W, kind: u8, body: &[u8], cap: usize) -> Result<()> {
    let length = u32::try_from(body.len()).map_err(|_| Error::LimitExceeded("peer message"))?;
    if body.len() > cap.min(MAX_MESSAGE) {
        return Err(Error::LimitExceeded("peer message"));
    }
    writer.write_all(&[kind])?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(body)?;
    writer.flush()?;
    Ok(())
}

fn read_msg<R: Read>(reader: &mut R, limits: &Limits) -> Result<(u8, Vec<u8>)> {
    read_msg_bounded(reader, MAX_MESSAGE.min(limits.max_record_bytes))
}
fn read_msg_bounded<R: Read>(reader: &mut R, cap: usize) -> Result<(u8, Vec<u8>)> {
    read_msg_or_eof(reader, cap)?
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into())
}

fn read_msg_or_eof<R: Read>(reader: &mut R, cap: usize) -> Result<Option<(u8, Vec<u8>)>> {
    let mut head = [0u8; 5];
    // Only absence of the first byte is clean EOF. Every partial header/body is a failure.
    loop {
        match reader.read(&mut head[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    reader.read_exact(&mut head[1..])?;
    let len = u32::from_be_bytes(head[1..].try_into().map_err(|_| Error::SchemaMismatch)?) as usize;
    if len > MAX_MESSAGE.min(cap) {
        return Err(Error::LimitExceeded("peer message"));
    }
    let mut body = Vec::new();
    crate::reserve(&mut body, len)?;
    body.resize(len, 0);
    reader.read_exact(&mut body)?;
    Ok(Some((head[0], body)))
}

fn record_size(first_byte: u8) -> usize {
    if first_byte == 0 {
        45
    } else {
        49
    }
}

fn record_from_store(store: &Store, id: NodeId) -> Result<Record> {
    let bytes = store.record_bytes(id).ok_or(Error::Missing(id))?;
    Record::decode(&bytes)
}

fn batch_size(limits: &Limits) -> usize {
    let by_response = limits.max_record_bytes.saturating_sub(2) / 49;
    let by_request = limits.max_record_bytes.saturating_sub(2) / 12;
    DESIRED_BATCH.min(by_response).min(by_request).max(1)
}

#[cfg(test)]
fn allowed_ids(store: &Store, manifest: &Manifest) -> Result<HashSet<NodeId>> {
    let limits = store.limits();
    let mut allowed = HashSet::new();
    let mut visits = 0usize;
    for (root, scale) in manifest.roots() {
        collect_closure(store, *root, *scale, &mut allowed, &mut visits, limits)?;
    }
    Ok(allowed)
}

fn collect_closure(
    store: &Store,
    id: NodeId,
    scale: u8,
    allowed: &mut HashSet<NodeId>,
    visits: &mut usize,
    limits: &Limits,
) -> Result<()> {
    *visits = visits
        .checked_add(1)
        .ok_or(Error::LimitExceeded("closure visits"))?;
    if *visits > limits.max_visits {
        return Err(Error::LimitExceeded("closure visits"));
    }
    let record = record_from_store(store, id)?;
    if record.scale() != scale {
        return Err(Error::IntegrityFailure);
    }
    if allowed.contains(&id) {
        return Ok(());
    }
    if allowed.len() >= limits.max_records {
        return Err(Error::LimitExceeded("closure records"));
    }
    allowed.insert(id);
    if record.scale() > 0 {
        for child in record.payload().chunks_exact(12) {
            let mut child_id = [0u8; 12];
            child_id.copy_from_slice(child);
            collect_closure(store, child_id, scale - 1, allowed, visits, limits)?;
        }
    }
    Ok(())
}

/// Serve one peer session from a full store and the manifest of the object being replicated.
pub fn serve<R: Read, W: Write>(
    store: &Store,
    manifest: &Manifest,
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    serve_many(store, std::slice::from_ref(manifest), reader, writer)
}

/// Serve one peer session when the store holds several objects. The manifest request body selects an
/// object by its whole-object SHA-256 or its manifest version (32 bytes); an empty body selects the
/// first object. An unknown selector terminates the session with Error::UnknownObject. Fetch scoping
/// is per selected object, so a session can only read that object's closure.
pub fn serve_many<R: Read, W: Write>(
    store: &Store,
    manifests: &[Manifest],
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    serve_many_with_limits(
        store,
        manifests,
        &PeerLimits::for_store(store.limits()),
        reader,
        writer,
    )
}

/// Serve a session with independent frame, total transfer and work budgets.
pub fn serve_many_with_limits<R: Read, W: Write>(
    store: &Store,
    manifests: &[Manifest],
    peer_limits: &PeerLimits,
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    if manifests.is_empty() {
        return Err(Error::SchemaMismatch);
    }
    let limits = store.limits().clone();
    let mut allowed = HashSet::new();
    let mut current: Option<usize> = None;
    let mut work = 0;
    let mut requests = 0usize;
    let mut served_bytes = 0usize;
    let mut served_records = 0usize;
    loop {
        let Some((kind, body)) = read_msg_or_eof(reader, peer_limits.max_message_bytes)? else {
            return Ok(());
        };
        if kind == DONE {
            return if body.is_empty() {
                Ok(())
            } else {
                Err(Error::SchemaMismatch)
            };
        }
        spend(&mut requests, 1, peer_limits.max_requests, "serve requests")?;
        match kind {
            MANIFEST_REQUEST => {
                let selected = if body.is_empty() {
                    Some(0)
                } else if body.len() == 32 {
                    let mut selector = [0u8; 32];
                    selector.copy_from_slice(&body);
                    let mut selected = None;
                    for (index, manifest) in manifests.iter().enumerate() {
                        spend(&mut work, 1, peer_limits.max_work, "peer work")?;
                        if manifest.whole_sha256() == selector || manifest.version() == selector {
                            selected = Some(index);
                            break;
                        }
                    }
                    selected
                } else {
                    return Err(Error::SchemaMismatch);
                };
                let Some(index) = selected else {
                    write_msg_limited(
                        writer,
                        ERROR,
                        b"unknown object",
                        peer_limits.max_message_bytes,
                    )?;
                    current = None;
                    allowed.clear();
                    continue;
                };
                if current != Some(index) {
                    allowed.clear();
                    let mut closure_limits = limits.clone();
                    closure_limits.max_visits = closure_limits
                        .max_visits
                        .min(peer_limits.max_work.saturating_sub(work));
                    let mut visits = 0;
                    manifests[index].validate(&limits)?;
                    for (root, scale) in manifests[index].roots() {
                        collect_closure(
                            store,
                            *root,
                            *scale,
                            &mut allowed,
                            &mut visits,
                            &closure_limits,
                        )?;
                    }
                    spend(&mut work, visits, peer_limits.max_work, "peer work")?;
                }
                current = Some(index);
                write_msg_limited(
                    writer,
                    MANIFEST,
                    &manifests[index].encode(),
                    peer_limits.max_message_bytes,
                )?;
            }
            FETCH => {
                let Some(_) = current else {
                    return Err(Error::SchemaMismatch);
                };
                if body.len() < 2 {
                    return Err(Error::SchemaMismatch);
                }
                let count = u16::from_be_bytes([body[0], body[1]]) as usize;
                if body.len() != 2 + 12 * count {
                    return Err(Error::SchemaMismatch);
                }
                if count == 0 {
                    return Err(Error::SchemaMismatch);
                }
                spend(
                    &mut served_records,
                    count,
                    peer_limits.max_staged_records,
                    "serve records",
                )?;
                let response_cap = 2 + count * 49;
                if response_cap > peer_limits.max_message_bytes.min(MAX_MESSAGE) {
                    return Err(Error::LimitExceeded("peer message"));
                }
                if served_bytes
                    .checked_add(count * 45)
                    .is_none_or(|n| n > peer_limits.max_staged_bytes)
                {
                    return Err(Error::LimitExceeded("serve bytes"));
                }
                let mut out = Vec::new();
                crate::reserve(&mut out, response_cap)?;
                out.extend_from_slice(&(count as u16).to_be_bytes());
                for index in 0..count {
                    let mut id = [0u8; 12];
                    id.copy_from_slice(&body[2 + 12 * index..14 + 12 * index]);
                    if !allowed.contains(&id) {
                        return Err(Error::SchemaMismatch);
                    }
                    let record = store.record_bytes(id).ok_or(Error::Missing(id))?;
                    spend(&mut work, 1, peer_limits.max_work, "peer work")?;
                    spend(
                        &mut served_bytes,
                        record.len(),
                        peer_limits.max_staged_bytes,
                        "serve bytes",
                    )?;
                    out.extend_from_slice(&record);
                }
                write_msg_limited(writer, NODES, &out, peer_limits.max_message_bytes)?;
            }
            _ => return Err(Error::SchemaMismatch),
        }
    }
}

/// Pull the object described by the peer's manifest without a caller-supplied identity anchor.
pub fn pull<R: Read, W: Write>(
    store: &mut Store,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_expected(store, None, reader, writer)
}

/// Pull the object, requiring the peer manifest to match a caller-supplied identity first.
pub fn pull_expected<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_selected(store, expected, None, reader, writer)
}

/// Client-side FETCH pipelining: how many FETCH frames may be in flight before their NODES replies
/// are read in order. Kept conservative by default because every frame is a full batch; the CLI
/// exposes --fetch-window so a high-latency run can be tuned or held at 1 for lock-step behavior.
pub const DEFAULT_FETCH_WINDOW: usize = 4;

/// Pull the object selected by a 32-byte whole-object SHA-256 or manifest version. None keeps the
/// original behavior (the peer's first/default object).
pub fn pull_selected<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_one(
        store,
        expected,
        selector,
        PullOptions {
            peer_limits: None,
            send_done: true,
            batch: None,
            window: DEFAULT_FETCH_WINDOW,
        },
        reader,
        writer,
    )
}

/// Pull the default object with an explicit pipelining window; window 1 reproduces lock-step.
pub fn pull_selected_with_window<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    window: usize,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_one(
        store,
        expected,
        selector,
        PullOptions {
            peer_limits: None,
            send_done: true,
            batch: None,
            window,
        },
        reader,
        writer,
    )
}

/// Pull with an explicit FETCH batch size, clamped to 1..=65535: raise it on high-latency links.
pub fn pull_selected_with_batch<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    batch: usize,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_selected_with_batch_window(
        store,
        expected,
        selector,
        batch,
        DEFAULT_FETCH_WINDOW,
        reader,
        writer,
    )
}

/// Pull with an explicit FETCH batch size and a pipelining window; window 1 is the old lock-step path.
pub fn pull_selected_with_batch_window<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    batch: usize,
    window: usize,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    pull_one(
        store,
        expected,
        selector,
        PullOptions {
            peer_limits: None,
            send_done: true,
            batch: Some(batch),
            window,
        },
        reader,
        writer,
    )
}

/// One requested artifact in a pull series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PullRequest {
    pub expected: Option<ExpectedObject>,
    pub selector: Option<[u8; 32]>,
}

/// Pull several objects on one session. Each object is fetched, verified and imported before the next;
/// one DONE closes the session after the last object. Returns (manifest, stats) per request in order.
pub fn pull_series<R: Read, W: Write>(
    store: &mut Store,
    requests: &[PullRequest],
    reader: &mut R,
    writer: &mut W,
) -> Result<Vec<(Manifest, PullStats)>> {
    pull_series_with_batch(store, requests, None, reader, writer)
}

/// Pull several objects on one session with an explicit FETCH batch size, clamped to 1..=65535:
/// raise it on high-latency links. `sync` and `repair` pass their `--batch` through this path.
pub fn pull_series_with_batch<R: Read, W: Write>(
    store: &mut Store,
    requests: &[PullRequest],
    batch: Option<usize>,
    reader: &mut R,
    writer: &mut W,
) -> Result<Vec<(Manifest, PullStats)>> {
    pull_series_with_batch_window(store, requests, batch, DEFAULT_FETCH_WINDOW, reader, writer)
}

/// Pull a series with explicit batch and pipelining window; window 1 reproduces lock-step.
pub fn pull_series_with_batch_window<R: Read, W: Write>(
    store: &mut Store,
    requests: &[PullRequest],
    batch: Option<usize>,
    window: usize,
    reader: &mut R,
    writer: &mut W,
) -> Result<Vec<(Manifest, PullStats)>> {
    if requests.len() > store.limits().max_visits {
        return Err(Error::LimitExceeded("series requests"));
    }
    let mut results = Vec::new();
    crate::reserve(&mut results, requests.len())?;
    for request in requests {
        results.push(pull_one(
            store,
            request.expected,
            request.selector,
            PullOptions {
                peer_limits: None,
                send_done: false,
                batch,
                window,
            },
            reader,
            writer,
        )?);
    }
    write_msg(writer, DONE, &[])?;
    Ok(results)
}

fn spend(used: &mut usize, amount: usize, cap: usize, name: &'static str) -> Result<()> {
    let total = used.checked_add(amount).ok_or(Error::LimitExceeded(name))?;
    if total > cap {
        return Err(Error::LimitExceeded(name));
    }
    *used = total;
    Ok(())
}

struct PullOptions {
    peer_limits: Option<PeerLimits>,
    send_done: bool,
    batch: Option<usize>,
    window: usize,
}

fn pull_one<R: Read, W: Write>(
    store: &mut Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    options: PullOptions,
    reader: &mut R,
    writer: &mut W,
) -> Result<(Manifest, PullStats)> {
    let limits = store.limits().clone();
    let peer_limits = options
        .peer_limits
        .unwrap_or_else(|| PeerLimits::for_store(&limits));
    let mut requests = 0;
    let mut work = 0;
    spend(&mut requests, 1, peer_limits.max_requests, "peer requests")?;
    let request = selector.map(|value| value.to_vec()).unwrap_or_default();
    write_msg_limited(
        writer,
        MANIFEST_REQUEST,
        &request,
        peer_limits.max_message_bytes,
    )?;
    let (kind, body) = read_msg_bounded(reader, peer_limits.max_message_bytes)?;
    if kind == ERROR {
        return Err(Error::UnknownObject);
    }
    if kind != MANIFEST {
        return Err(Error::SchemaMismatch);
    }
    let manifest = Manifest::decode(&body, &limits)?;
    if let Some(expected) = expected {
        if !expected.matches(&manifest) {
            return Err(Error::BindingMismatch);
        }
    }

    let mut fetched: BTreeMap<NodeId, Record> = BTreeMap::new();
    let mut stats = PullStats::default();
    // The largest response (49-byte records) and request must both fit before allocating a batch.
    let body_cap = peer_limits.max_message_bytes.min(MAX_MESSAGE);
    let batch_cap = (body_cap.saturating_sub(2) / 49)
        .min(peer_limits.max_in_flight_response_bytes.saturating_sub(7) / 49);
    let batch = options
        .batch
        .unwrap_or_else(|| batch_size(&limits))
        .clamp(1, u16::MAX as usize)
        .min(batch_cap);
    // Fully cached and empty objects need no FETCH budget at all.
    loop {
        let mut traversal_limits = limits.clone();
        traversal_limits.max_visits = traversal_limits
            .max_visits
            .min(peer_limits.max_work.saturating_sub(work));
        let mut visits = 0;
        let missing = Store::missing_from(
            &Overlay {
                store,
                fetched: &fetched,
            },
            &manifest,
            &traversal_limits,
            &mut visits,
        )?;
        spend(&mut work, visits, peer_limits.max_work, "peer work")?;
        if missing.is_empty() {
            break;
        }
        if batch == 0 {
            return Err(Error::LimitExceeded("peer message"));
        }
        let response_size = 5 + 2 + 49 * batch;
        let response_window = peer_limits.max_in_flight_response_bytes / response_size;
        if response_window == 0 {
            return Err(Error::LimitExceeded("in-flight responses"));
        }
        let window = options
            .window
            .max(1)
            .min(response_window)
            .min((262_144 / (7 + 12 * batch)).max(1));
        let chunks: Vec<&[NodeId]> = missing.chunks(batch).collect();
        for group in chunks.chunks(window) {
            // Reserve the group's minimum record bytes/count before issuing any of its requests.
            let count: usize = group.iter().map(|chunk| chunk.len()).sum();
            if fetched
                .len()
                .checked_add(count)
                .is_none_or(|n| n > peer_limits.max_staged_records.min(limits.max_records))
                || (stats.record_bytes as usize)
                    .checked_add(count * 45)
                    .is_none_or(|n| n > peer_limits.max_staged_bytes.min(limits.max_record_bytes))
            {
                return Err(Error::LimitExceeded("peer staging"));
            }
            for chunk in group {
                spend(&mut requests, 1, peer_limits.max_requests, "peer requests")?;
                let mut request = Vec::new();
                crate::reserve(&mut request, 2 + 12 * chunk.len())?;
                request.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
                for id in *chunk {
                    request.extend_from_slice(id);
                }
                write_msg_limited(writer, FETCH, &request, peer_limits.max_message_bytes)?;
            }
            for chunk in group {
                read_fetch_response(
                    reader,
                    &limits,
                    &peer_limits,
                    chunk,
                    &mut fetched,
                    &mut stats,
                    &mut work,
                )?;
            }
        }
    }

    // The strict walker checks the declared scales, exact length, final-leaf padding and digest
    // against the staged overlay. No cache mutation or commit receipt precedes this check.
    let overlay = Overlay {
        store,
        fetched: &fetched,
    };
    let mut reconstruction_limits = limits.clone();
    reconstruction_limits.max_visits = reconstruction_limits
        .max_visits
        .min(peer_limits.max_work.saturating_sub(work));
    Store::rebuild_from(&overlay, &manifest, &reconstruction_limits)?;
    store.commit_staged(fetched)?;
    stats.verified = true;
    if options.send_done {
        write_msg_limited(writer, DONE, &[], peer_limits.max_message_bytes)?;
    }

    Ok((manifest, stats))
}

fn read_fetch_response<R: Read>(
    reader: &mut R,
    limits: &Limits,
    peer_limits: &PeerLimits,
    chunk: &[NodeId],
    fetched: &mut BTreeMap<NodeId, Record>,
    stats: &mut PullStats,
    work: &mut usize,
) -> Result<()> {
    let remaining = peer_limits
        .max_staged_bytes
        .min(limits.max_record_bytes)
        .saturating_sub(stats.record_bytes as usize);
    let cap = peer_limits
        .max_message_bytes
        .min(2 + chunk.len() * 49)
        .min(remaining.saturating_add(2));
    let (kind, body) = read_msg_bounded(reader, cap)?;
    if kind != NODES || body.len() < 2 {
        return Err(Error::SchemaMismatch);
    }
    let count = u16::from_be_bytes([body[0], body[1]]) as usize;
    if count != chunk.len() {
        return Err(Error::IntegrityFailure);
    }
    // The protocol returns records in request order. This makes correlation linear, with no
    // attacker-controlled quadratic chunk.contains scan or separate response membership index.
    let mut offset = 2usize;
    for expected_id in chunk {
        if offset >= body.len() {
            return Err(Error::SchemaMismatch);
        }
        let size = record_size(body[offset]);
        let end = offset.checked_add(size).ok_or(Error::SchemaMismatch)?;
        spend(work, 1, peer_limits.max_work, "peer work")?;
        if fetched.len() >= peer_limits.max_staged_records.min(limits.max_records) {
            return Err(Error::LimitExceeded("peer staging"));
        }
        let record = Record::decode(body.get(offset..end).ok_or(Error::SchemaMismatch)?)?;
        if record.id() != *expected_id || fetched.contains_key(expected_id) {
            return Err(Error::IntegrityFailure);
        }
        let total = stats
            .record_bytes
            .checked_add(size as u64)
            .ok_or(Error::LimitExceeded("peer staging"))?;
        if total > peer_limits.max_staged_bytes.min(limits.max_record_bytes) as u64 {
            return Err(Error::LimitExceeded("peer staging"));
        }
        fetched.insert(record.id(), record);
        stats.nodes_requested += 1;
        stats.record_bytes = total;
        offset = end;
    }
    if offset != body.len() {
        return Err(Error::SchemaMismatch);
    }
    stats.rounds += 1;
    Ok(())
}
/// Check whether this store already holds the peer's object without fetching records or rebuilding
/// the payload. An expected pin is checked against the peer manifest first. verified means the local
/// store's closure for the pinned manifest is complete; a byte-level re-hash requires pull.
pub fn verify<R: Read, W: Write>(
    store: &Store,
    expected: Option<ExpectedObject>,
    selector: Option<[u8; 32]>,
    reader: &mut R,
    writer: &mut W,
) -> Result<VerifyStats> {
    let limits = store.limits().clone();
    let request = selector.map(|value| value.to_vec()).unwrap_or_default();
    write_msg(writer, MANIFEST_REQUEST, &request)?;
    let (kind, body) = read_msg(reader, &limits)?;
    if kind == ERROR {
        return Err(Error::UnknownObject);
    }
    if kind != MANIFEST {
        return Err(Error::SchemaMismatch);
    }
    let manifest = Manifest::decode(&body, &limits)?;
    if let Some(expected) = expected {
        if !expected.matches(&manifest) {
            return Err(Error::BindingMismatch);
        }
    }
    let fetched = BTreeMap::new();
    let missing = missing_nodes(store, &manifest, &fetched)?;
    write_msg(writer, DONE, &[])?;
    Ok(VerifyStats {
        missing: missing.len(),
        verified: missing.is_empty(),
    })
}

/// Check several objects on one session. Requests are pipelined in bounded windows (64): the whole set
/// of manifest requests in a window is written, then the responses are read in order, so a batch costs
/// about one round trip per window instead of one per check. A DONE is sent once after the last window;
/// an unknown selector writes an error frame and keeps the session open. Per-check outcomes report
/// verified, stale (missing closure nodes), unknown selector or pin mismatch.
pub fn verify_series<R: Read, W: Write>(
    store: &Store,
    requests: &[VerifyRequest],
    reader: &mut R,
    writer: &mut W,
) -> Result<Vec<VerifyOutcome>> {
    const WINDOW: usize = 64;
    let limits = store.limits().clone();
    if requests.len() > limits.max_visits {
        return Err(Error::LimitExceeded("series requests"));
    }
    let peer_limits = PeerLimits::for_store(&limits);
    let mut work = 0;
    let mut outcomes = Vec::new();
    crate::reserve(&mut outcomes, requests.len())?;
    let mut start = 0usize;
    while start < requests.len() {
        let end = (start + WINDOW).min(requests.len());
        for request in &requests[start..end] {
            let body = request
                .selector
                .map(|value| value.to_vec())
                .unwrap_or_default();
            write_msg(writer, MANIFEST_REQUEST, &body)?;
        }
        for request in &requests[start..end] {
            let (kind, message) = read_msg(reader, &limits)?;
            if kind == ERROR {
                outcomes.push(VerifyOutcome::Unknown);
                continue;
            }
            if kind != MANIFEST {
                return Err(Error::SchemaMismatch);
            }
            let manifest = Manifest::decode(&message, &limits)?;
            if let Some(expected) = request.expected {
                if !expected.matches(&manifest) {
                    outcomes.push(VerifyOutcome::Mismatch);
                    continue;
                }
            }
            let fetched = BTreeMap::new();
            let mut traversal_limits = limits.clone();
            traversal_limits.max_visits = traversal_limits
                .max_visits
                .min(peer_limits.max_work.saturating_sub(work));
            let mut visits = 0;
            let missing = Store::missing_from(
                &Overlay {
                    store,
                    fetched: &fetched,
                },
                &manifest,
                &traversal_limits,
                &mut visits,
            )?;
            spend(&mut work, visits, peer_limits.max_work, "peer work")?;
            outcomes.push(if missing.is_empty() {
                VerifyOutcome::Verified { missing: 0 }
            } else {
                VerifyOutcome::Stale {
                    missing: missing.len(),
                }
            });
        }
        start = end;
    }
    write_msg(writer, DONE, &[])?;
    Ok(outcomes)
}

struct Overlay<'a> {
    store: &'a Store,
    fetched: &'a BTreeMap<NodeId, Record>,
}
impl RecordBackend for Overlay<'_> {
    fn fetch(&self, id: NodeId) -> Result<Option<Record>> {
        match self.fetched.get(&id) {
            Some(record) => Ok(Some(record.clone())),
            None => self.store.fetch(id),
        }
    }
}

fn missing_nodes(
    store: &Store,
    manifest: &Manifest,
    fetched: &BTreeMap<NodeId, Record>,
) -> Result<Vec<NodeId>> {
    Store::missing_from(
        &Overlay { store, fetched },
        manifest,
        store.limits(),
        &mut 0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> Limits {
        Limits {
            max_object_bytes: 1 << 30,
            max_records: 10_000_000,
            max_record_bytes: 1 << 30,
            max_returned_bytes: 1 << 30,
            max_visits: 100_000_000,
            max_occurrences: 10_000_000,
        }
    }

    fn atoms(n: usize, salt: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(n * 32);
        for i in 0..n {
            for j in 0..32 {
                let v = if j < 8 {
                    i.to_le_bytes()[j]
                } else {
                    ((i * 37 + j * 11 + salt as usize) & 0xFF) as u8
                };
                out.push(v);
            }
        }
        out
    }

    #[test]
    fn closure_excludes_records_from_other_objects() {
        let mut store = Store::new(limits());
        let first = store.fold(&atoms(1000, 1)).expect("fold first").manifest;
        let second = store.fold(&atoms(1000, 2)).expect("fold second").manifest;
        let closure = allowed_ids(&store, &first).expect("closure");
        let foreign = second.roots()[0].0;
        assert!(
            !closure.contains(&foreign),
            "a record from another object must not be in the served closure"
        );
        assert!(!closure.is_empty());
    }
}
