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
pub struct PrunedCandidate {
    pub mode: Mode,
    /// Proven minimum successful-miss HTTP body bytes, not an encoded size.
    /// The candidate cannot beat the incumbent even under the original ties.
    pub http_body_lower_bound: u64,
}

#[derive(Debug, Clone)]
pub struct Selection {
    /// Scored eligible encodings in tie order. Without a base, sparse/prefix
    /// are not evaluated. With a base, an omitted sparse entry means its
    /// bounded encoder returned None (no recipe cheaper than Raw), unless its
    /// lower-bound receipt is in `pruned`. Evaluated encoder errors propagate.
    /// For an exact-repeat probe only Raw is evaluated.
    pub candidates: Vec<CandidateCost>,
    /// Runtime senders may skip encodings proved unable to win. Their exact
    /// costs are unknown and are never inserted into `candidates`.
    pub pruned: Vec<PrunedCandidate>,
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
        if wins(
            mode,
            http_body_bytes,
            self.selection.selected,
            self.selection.expected_http_body_bytes,
        ) {
            self.selection.selected = mode;
            self.selection.expected_http_body_bytes = http_body_bytes;
            self.bodies = Some(bodies);
        }
        Ok(())
    }

    fn prune(&mut self, mode: Mode, lower_bound: u64) -> bool {
        if wins(
            mode,
            lower_bound,
            self.selection.selected,
            self.selection.expected_http_body_bytes,
        ) {
            return false;
        }
        self.selection.pruned.push(PrunedCandidate {
            mode,
            http_body_lower_bound: lower_bound,
        });
        true
    }
}

fn priority(mode: Mode) -> u8 {
    match mode {
        Mode::Raw => 0,
        Mode::Zstd => 1,
        Mode::SparseV1 => 2,
        Mode::ZstdPrefixV1 => 3,
        Mode::RuntimeV1 => 4,
        Mode::Dag => 5,      // Not eligible for this selector.
        Mode::SchemaV1 => 6, // Explicit prototype sender only.
        Mode::ZstdDictionaryV1 => 7,
    }
}

fn wins(mode: Mode, bytes: u64, incumbent: Mode, incumbent_bytes: u64) -> bool {
    (bytes, priority(mode)) < (incumbent_bytes, priority(incumbent))
}

fn initial(target: &[u8], base: Option<&[u8]>, warm_probe: bool) -> Result<Prepared> {
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
    Ok(Prepared {
        selection: Selection {
            candidates: vec![CandidateCost {
                mode: Mode::Raw,
                http_body_bytes: raw,
            }],
            pruned: Vec::new(),
            selected: Mode::Raw,
            expected_http_body_bytes: raw,
            exact_repeat_probe: warm_probe && base == Some(target),
        },
        bodies: None,
    })
}

pub(crate) fn prepare(target: &[u8], base: Option<&[u8]>, warm_probe: bool) -> Result<Prepared> {
    let mut best = initial(target, base, warm_probe)?;
    if best.selection.exact_repeat_probe {
        return Ok(best);
    }
    conventional(&mut best, target, base, warm_probe, false)?;
    Ok(best)
}

pub(crate) fn prepare_runtime<P>(
    target: &[u8],
    checkpoint: &[u8],
    warm_probe: bool,
    runtime: P,
) -> Result<Prepared>
where
    P: FnOnce() -> Result<Option<Vec<Vec<u8>>>>,
{
    let mut best = initial(target, Some(checkpoint), warm_probe)?;
    if best.selection.exact_repeat_probe {
        return Ok(best);
    }
    if let Some(bodies) = runtime()? {
        best.consider(Mode::RuntimeV1, bodies, warm_probe)?;
    }
    conventional(&mut best, target, Some(checkpoint), warm_probe, true)?;
    Ok(best)
}

// Every nonempty chunk needs its five-byte header, >=1 payload byte and 128
// bytes of Data/Accepted framing. Raw fallback cannot invalidate this bound.
fn zstd_lower_bound(target_len: usize, warm_probe: bool) -> u64 {
    368 + u64::from(warm_probe) * 40 + target_len.div_ceil(MAX_CHUNK) as u64 * 134
}

// Nonempty targets require >=1 frame (128+9). COPY costs13 and emits <=base_len;
// LITERAL costs5+n >=n. Thus total op bytes >=ceil(13*N/max(base_len,13)).
fn sparse_lower_bound(target_len: usize, base_len: usize, warm_probe: bool) -> u64 {
    400 + u64::from(warm_probe) * 40
        + 137
        + (13 * target_len as u64).div_ceil(base_len.max(13) as u64)
}

fn prefix_lower_bound(target_len: usize, warm_probe: bool) -> Result<u64> {
    // At least one Data body: six codec bytes +128 request/reply framing.
    Ok(400
        + u64::from(warm_probe) * 40
        + 134
        + crate::zstd_prefix::minimum_encoded_size(target_len)? as u64)
}

fn conventional(
    best: &mut Prepared,
    target: &[u8],
    base: Option<&[u8]>,
    warm_probe: bool,
    pruning: bool,
) -> Result<()> {
    if !pruning || !best.prune(Mode::Zstd, zstd_lower_bound(target.len(), warm_probe)) {
        best.consider(Mode::Zstd, compressed_chunks(target)?, warm_probe)?;
    }
    if let Some(base) = base {
        let prune_sparse = pruning
            && !target.is_empty()
            && best.prune(
                Mode::SparseV1,
                sparse_lower_bound(target.len(), base.len(), warm_probe),
            );
        if !prune_sparse {
            if let Some(recipe) = crate::delta::encode(base, target)? {
                best.consider(Mode::SparseV1, recipe.frames, warm_probe)?;
            }
        }
        let prune_prefix = pruning
            && best.prune(
                Mode::ZstdPrefixV1,
                prefix_lower_bound(target.len(), warm_probe)?,
            );
        if !prune_prefix {
            let encoded = crate::zstd_prefix::encode(base, target)?;
            // Cost the contiguous encoded candidate first. Only materialize its
            // transport bodies when it wins, avoiding another full losing copy.
            let prefix_cost = cost(
                Mode::ZstdPrefixV1,
                encoded.chunks(MAX_CHUNK).map(|c| c.len() + 6),
                warm_probe,
            )?;
            if wins(
                Mode::ZstdPrefixV1,
                prefix_cost,
                best.selection.selected,
                best.selection.expected_http_body_bytes,
            ) {
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
    }
    best.selection
        .candidates
        .sort_by_key(|candidate| priority(candidate.mode));
    Ok(())
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

#[cfg(test)]
mod runtime_selection_tests {
    use super::*;
    use crate::{
        shared_runtime::{self, OutputKind, Recipe, State},
        shared_transfer,
    };

    #[test]
    fn runtime_first_preserves_exhaustive_winner_and_body_bytes() {
        let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
        let recipe = Recipe {
            steps: 81,
            payload: 1.0,
            output: OutputKind::Trace,
        };
        let prediction =
            shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT).unwrap();
        let mut edited = prediction.clone();
        let middle = edited.len() / 2;
        edited[middle] ^= 7;
        let mut unrelated = vec![0; prediction.len()];
        let mut random = 0x1234_5678_9abc_def0u64;
        for byte in &mut unrelated {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            *byte = random as u8;
        }
        let cases = [
            Vec::new(),
            vec![1],
            vec![0; 512],
            checkpoint.clone(),
            prediction,
            edited,
            unrelated,
        ];
        for (case, target) in cases.iter().enumerate() {
            for warm_probe in [false, true] {
                let runtime = || shared_transfer::prepare(&checkpoint, target, &recipe);
                let mut expected = prepare(target, Some(&checkpoint), warm_probe).unwrap();
                if !expected.selection.exact_repeat_probe {
                    if let Some(bodies) = runtime().unwrap() {
                        expected
                            .consider(Mode::RuntimeV1, bodies, warm_probe)
                            .unwrap();
                    }
                }
                let actual = prepare_runtime(target, &checkpoint, warm_probe, runtime).unwrap();
                assert_eq!(
                    actual.selection.selected, expected.selection.selected,
                    "case={case}, warm={warm_probe}"
                );
                assert_eq!(
                    actual.selection.expected_http_body_bytes,
                    expected.selection.expected_http_body_bytes
                );
                assert_eq!(actual.bodies, expected.bodies);
                for pruned in &actual.selection.pruned {
                    if let Some(exact) = expected
                        .selection
                        .candidates
                        .iter()
                        .find(|c| c.mode == pruned.mode)
                    {
                        assert!(pruned.http_body_lower_bound <= exact.http_body_bytes);
                    }
                    assert!(!wins(
                        pruned.mode,
                        pruned.http_body_lower_bound,
                        actual.selection.selected,
                        actual.selection.expected_http_body_bytes
                    ));
                    assert!(!actual
                        .selection
                        .candidates
                        .iter()
                        .any(|c| c.mode == pruned.mode));
                }
                if case == 4 || case == 5 {
                    assert!(actual.selection.pruned.iter().any(|c| c.mode == Mode::Zstd));
                    assert!(actual
                        .selection
                        .pruned
                        .iter()
                        .any(|c| c.mode == Mode::SparseV1));
                    assert!(actual
                        .selection
                        .candidates
                        .iter()
                        .any(|c| c.mode == Mode::ZstdPrefixV1));
                }
            }
        }
    }

    #[test]
    fn lower_bound_ties_preserve_original_codec_priority() {
        let order = [
            Mode::Raw,
            Mode::Zstd,
            Mode::SparseV1,
            Mode::ZstdPrefixV1,
            Mode::RuntimeV1,
        ];
        for (i, mode) in order.iter().enumerate() {
            for (j, incumbent) in order.iter().enumerate() {
                assert_eq!(wins(*mode, 700, *incumbent, 700), i < j);
                assert!(wins(*mode, 699, *incumbent, 700));
                assert!(!wins(*mode, 701, *incumbent, 700));
            }
        }
        let mut prepared = initial(&[1; 1024], None, false).unwrap();
        prepared
            .consider(Mode::RuntimeV1, vec![vec![0; 172]], false)
            .unwrap();
        assert_eq!(prepared.selection.expected_http_body_bytes, 700);
        assert!(!prepared.prune(Mode::Zstd, 700));
        prepared
            .consider(Mode::Zstd, vec![vec![0; 204]], false)
            .unwrap();
        assert_eq!(prepared.selection.selected, Mode::Zstd);
        assert!(prepared.prune(Mode::SparseV1, 700));
        assert_eq!(prepared.selection.pruned[0].http_body_lower_bound, 700);
    }

    #[test]
    fn encoded_lower_bounds_hold_for_empty_small_and_chunk_boundary_inputs() {
        for warm in [false, true] {
            for len in [
                0,
                1,
                12,
                13,
                14,
                31,
                32,
                MAX_CHUNK - 1,
                MAX_CHUNK,
                MAX_CHUNK + 1,
            ] {
                let target: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
                let zstd = compressed_chunks(&target).unwrap();
                assert!(
                    zstd_lower_bound(len, warm)
                        <= cost(Mode::Zstd, zstd.iter().map(Vec::len), warm).unwrap()
                );
                if len == 0 {
                    continue;
                }
                for base_len in [0, 1, 12, 13, 14, 32, len] {
                    let base: Vec<u8> = (0..base_len).map(|i| (i % 251) as u8).collect();
                    if let Some(sparse) = crate::delta::encode(&base, &target).unwrap() {
                        assert!(
                            sparse_lower_bound(len, base_len, warm)
                                <= cost(Mode::SparseV1, sparse.frames.iter().map(Vec::len), warm)
                                    .unwrap()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn runtime_repeat_shortcut_and_errors_keep_pre_begin_behavior() {
        let checkpoint = State::seeded(1).unwrap().encode().unwrap();
        let result = prepare_runtime(&checkpoint, &checkpoint, true, || {
            panic!("exact repeat must not replay")
        })
        .unwrap();
        assert_eq!(result.selection.selected, Mode::Raw);
        assert!(result.selection.exact_repeat_probe);
        assert!(result.selection.pruned.is_empty());
        assert!(matches!(
            prepare_runtime(b"target", &checkpoint, false, || Err(Error::Malformed)),
            Err(Error::Malformed)
        ));
        let oversized = vec![0; client::MAX_BATCHED_OBJECT + 1];
        assert!(matches!(
            prepare_runtime(&oversized, &checkpoint, false, || panic!(
                "reject size first"
            )),
            Err(Error::Limit)
        ));
    }

    #[test]
    fn prefix_structure_bound_prunes_large_runtime_without_changing_bytes() {
        let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
        let recipe = Recipe {
            steps: 4096,
            payload: 1.0,
            output: OutputKind::Trace,
        };
        let target =
            shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT).unwrap();
        let runtime = || shared_transfer::prepare(&checkpoint, &target, &recipe);
        let mut expected = prepare(&target, Some(&checkpoint), false).unwrap();
        expected
            .consider(Mode::RuntimeV1, runtime().unwrap().unwrap(), false)
            .unwrap();
        let actual = prepare_runtime(&target, &checkpoint, false, runtime).unwrap();
        assert_eq!(actual.selection.selected, expected.selection.selected);
        assert_eq!(
            actual.selection.expected_http_body_bytes,
            expected.selection.expected_http_body_bytes
        );
        assert_eq!(actual.bodies, expected.bodies);
        let prefix = actual
            .selection
            .pruned
            .iter()
            .find(|c| c.mode == Mode::ZstdPrefixV1)
            .expect("format minimum proves prefix cannot win this runtime transfer");
        assert_eq!(prefix.http_body_lower_bound, 886);
        let exact = expected
            .selection
            .candidates
            .iter()
            .find(|c| c.mode == Mode::ZstdPrefixV1)
            .unwrap();
        assert!(prefix.http_body_lower_bound <= exact.http_body_bytes);
        assert!(actual
            .selection
            .candidates
            .iter()
            .all(|c| c.mode != Mode::ZstdPrefixV1));
    }
}
