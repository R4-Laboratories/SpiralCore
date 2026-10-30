//! Opt-in byte-first selection among the existing bounded transfer encodings.
//!
//! Costs describe a successful target-cache miss using the WAN HTTP body
//! envelopes, not TLS/IP traffic or elapsed time. All candidate work is local.
use crate::{client, protocol::Mode, Error, Result, MAX_CHUNK, MAX_FRAME};

#[derive(Debug, Clone)]
pub struct CandidateCost {
    pub mode: Mode,
    pub http_body_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct Selection {
    /// Scored eligible encodings in tie order. Without a base, sparse/prefix
    /// are not evaluated. With a base, an omitted sparse entry means its
    /// bounded encoder returned None (no recipe cheaper than Raw). Encoder
    /// errors propagate. For an exact-repeat probe only Raw is evaluated.
    pub candidates: Vec<CandidateCost>,
    pub selected: Mode,
    /// Successful full-transfer body cost. An early Published/error reply can
    /// change actual traffic. The exact-repeat shortcut reports the Raw miss
    /// cost, not a minimum over encodings it deliberately did not evaluate.
    pub expected_http_body_bytes: u64,
    pub exact_repeat_probe: bool,
}

pub(crate) struct Prepared {
    pub selection: Selection,
    // None borrows the caller's target for Raw. Only the incumbent and the
    // candidate being evaluated coexist; rejected candidate buffers are dropped.
    pub bodies: Option<Vec<Vec<u8>>>,
}

/// Exact successful-miss cost at the current one-batch object ceiling. Reject
/// any candidate that violates that assumption before contacting the receiver.
fn cost(mode: Mode, sizes: impl Iterator<Item = usize>, warm_probe: bool) -> Result<u64> {
    let base = usize::from(matches!(
        mode,
        Mode::SparseV1 | Mode::ZstdPrefixV1 | Mode::RuntimeV1
    )) * 32;
    // Begin/Ready/Finish/Published frame bytes: 102 + 66 + 56 + 88.
    // Four frame length words: 16. SCWBAT01 header: 12. SCWREP01 header: 28.
    let mut total = 368 + base + usize::from(warm_probe) * 40;
    let mut request = 12 + 4 + 102 + base + 4 + 56;
    let mut frames = 2usize;
    for size in sizes {
        if size > MAX_FRAME - 56 {
            return Err(Error::Limit);
        }
        frames = frames.checked_add(1).ok_or(Error::Limit)?;
        request = request.checked_add(60 + size).ok_or(Error::Limit)?;
        // Data header56 + Accepted64 + two length words8.
        total = total.checked_add(128 + size).ok_or(Error::Limit)?;
    }
    if frames > client::MAX_BATCH_FRAMES || request > client::MAX_BATCH_BYTES {
        return Err(Error::Limit);
    }
    Ok(total as u64)
}

impl Prepared {
    pub(crate) fn consider(
        &mut self,
        mode: Mode,
        bodies: Vec<Vec<u8>>,
        warm_probe: bool,
    ) -> Result<()> {
        let http_body_bytes = cost(mode, bodies.iter().map(Vec::len), warm_probe)?;
        self.selection.candidates.push(CandidateCost {
            mode,
            http_body_bytes,
        });
        // Stable ties prefer Raw, then chunked Zstd, Sparse and Prefix.
        if http_body_bytes < self.selection.expected_http_body_bytes {
            self.selection.selected = mode;
            self.selection.expected_http_body_bytes = http_body_bytes;
            self.bodies = Some(bodies);
        }
        Ok(())
    }
}

pub(crate) fn prepare(target: &[u8], base: Option<&[u8]>, warm_probe: bool) -> Result<Prepared> {
    if target.len() > client::MAX_BATCHED_OBJECT
        || base.is_some_and(|b| b.len() > client::MAX_BATCHED_OBJECT)
    {
        return Err(Error::Limit);
    }
    let raw = cost(
        Mode::Raw,
        target.chunks(MAX_CHUNK).map(|c| c.len() + 5),
        warm_probe,
    )?;
    let mut best = Prepared {
        selection: Selection {
            candidates: vec![CandidateCost {
                mode: Mode::Raw,
                http_body_bytes: raw,
            }],
            selected: Mode::Raw,
            expected_http_body_bytes: raw,
            exact_repeat_probe: warm_probe && base == Some(target),
        },
        bodies: None,
    };
    if best.selection.exact_repeat_probe {
        return Ok(best);
    }
    best.consider(Mode::Zstd, compressed_chunks(target)?, warm_probe)?;
    if let Some(base) = base {
        if let Some(recipe) = crate::delta::encode(base, target)? {
            best.consider(Mode::SparseV1, recipe.frames, warm_probe)?;
        }
        let encoded = crate::zstd_prefix::encode(base, target)?;
        // Cost the contiguous encoded candidate first. Only materialize its
        // transport bodies when it wins, avoiding another full losing copy.
        let prefix_cost = cost(
            Mode::ZstdPrefixV1,
            encoded.chunks(MAX_CHUNK).map(|c| c.len() + 6),
            warm_probe,
        )?;
        if prefix_cost < best.selection.expected_http_body_bytes {
            best.consider(
                Mode::ZstdPrefixV1,
                crate::zstd_prefix::chunks(&encoded)?,
                warm_probe,
            )?;
        } else {
            best.selection.candidates.push(CandidateCost {
                mode: Mode::ZstdPrefixV1,
                http_body_bytes: prefix_cost,
            });
        }
    }
    Ok(best)
}

fn compressed_chunks(target: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut bodies = Vec::new();
    bodies
        .try_reserve_exact(target.len().div_ceil(MAX_CHUNK))
        .map_err(|_| Error::Limit)?;
    for chunk in target.chunks(MAX_CHUNK) {
        let compressed = zstd::bulk::compress(chunk, 1).map_err(|_| Error::Malformed)?;
        let (encoding, payload) = if compressed.len() < chunk.len() {
            (1, compressed.as_slice())
        } else {
            (0, chunk)
        };
        let mut body = Vec::new();
        body.try_reserve_exact(5 + payload.len())
            .map_err(|_| Error::Limit)?;
        body.push(encoding);
        body.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        body.extend_from_slice(payload);
        bodies.push(body);
    }
    Ok(bodies)
}
