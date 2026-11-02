//! Exact successful-miss HTTP-body selection. Readiness is caller knowledge,
//! never authorization. Cold selection charges a complete separate Raw seed.
use crate::{
    client, fitted_transfer, object_id,
    protocol::{Mode, Offer},
    schema_zstd, selection, zstd_whole, Error, Id, Result, MAX_CHUNK,
};

#[cfg(test)]
#[path = "../tests/fixtures/candidate_cpu_checks.rs"]
mod candidate_cpu_checks;

#[derive(Debug, Clone)]
pub struct CandidateCost {
    pub mode: Mode,
    pub target_http_body_bytes: u64,
    pub setup_http_body_bytes: u64,
    pub total_http_body_bytes: u64,
}
#[derive(Debug, Clone)]
pub struct Selection {
    pub candidates: [CandidateCost; 3],
    pub selected_mode: Mode,
    pub needs_schema_seed: bool,
}

#[derive(Debug, Clone)]
pub struct PrunedCandidate {
    pub mode: Mode,
    /// Proven minimum, not the size of an encoding that was produced.
    pub target_http_body_lower_bound: u64,
    pub setup_http_body_bytes: u64,
    pub total_http_body_lower_bound: u64,
}

#[derive(Debug, Clone)]
pub struct WarmSelection {
    /// Evaluated exact costs in Raw/Whole/Sparse/Schema tie order. Sparse is
    /// absent if no base was supplied or its bounded encoder preferred Raw.
    pub candidates: Vec<CandidateCost>,
    pub pruned: Vec<PrunedCandidate>,
    pub selected_mode: Mode,
    pub needs_schema_seed: bool,
}

/// Eligible combined candidates under the existing successful-miss HTTP-body
/// cost model. Readiness is the snapshot before selection, not authorization.
/// An unseeded schema is ineligible; no setup or hidden seed is performed here.
#[derive(Debug, Clone)]
pub struct CombinedSelection {
    pub candidates: Vec<CandidateCost>,
    pub pruned: Vec<PrunedCandidate>,
    pub selected_mode: Mode,
    /// Acknowledged full-byte repeat: only the Raw miss cost was evaluated.
    pub exact_repeat_probe: bool,
    pub checkpoint_ready: bool,
    pub schema_ready: bool,
}

pub struct PreparedWarmTransfer {
    pub(crate) offer: Offer,
    pub(crate) payload: Payload,
    pub(crate) warm_probe: bool,
    selection: WarmSelection,
}
impl PreparedWarmTransfer {
    pub fn selection(&self) -> &WarmSelection {
        &self.selection
    }
}

pub struct PreparedTransfer {
    pub(crate) offer: Offer,
    pub(crate) payload: Payload,
    pub(crate) warm_probe: bool,
    selection: Selection,
}
impl PreparedTransfer {
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}
pub(crate) enum Payload {
    Raw(Vec<u8>),
    Encoded(Vec<Vec<u8>>),
}

pub struct PreparedSelector {
    whole: zstd_whole::PreparedEncoder,
    schema: schema_zstd::PreparedEncoder,
}
impl PreparedSelector {
    pub fn new(artifact: &[u8]) -> Result<Self> {
        Ok(Self {
            schema: schema_zstd::PreparedEncoder::new(artifact)?,
            whole: zstd_whole::PreparedEncoder::new()?,
        })
    }
    pub fn artifact(&self) -> &[u8] {
        self.schema.artifact()
    }
    pub fn artifact_id(&self) -> Id {
        self.schema.artifact_id()
    }
    pub fn retained_bytes(&self) -> usize {
        self.schema.retained_bytes() + self.whole.retained_bytes()
    }
    /// Context quota only; caller input and bounded candidate buffers are separate.
    pub fn retained_charge(&self) -> u64 {
        self.schema.retained_charge() + self.whole.retained_charge()
    }

    /// Add an already-admitted prior object as a Sparse candidate. The caller's
    /// base/readiness hints never authorize receiver access. Missing state and
    /// evaluated codec errors propagate; no implicit seed/reoffer is performed.
    /// The minimum concerns successful target-cache misses, including optional
    /// schema setup. Target hits may cost less. All preparation work is local.
    pub fn prepare_warm(
        &mut self,
        target: &[u8],
        base: Option<&[u8]>,
        schema_ready: bool,
        warm_probe: bool,
    ) -> Result<PreparedWarmTransfer> {
        self.prepare_warm_with_identity(
            target,
            base,
            schema_ready,
            warm_probe,
            |bytes| Ok(object_id(bytes)),
            None,
        )
    }

    // Only session-owned ACKed bytes can supply a cached identity. This keeps
    // the ordinary public API and its base hashing behavior unchanged.
    pub(crate) fn prepare_acknowledged(
        &mut self,
        target: &[u8],
        base: Option<&crate::session::AcknowledgedBase>,
        schema_ready: bool,
        warm_probe: bool,
    ) -> Result<PreparedWarmTransfer> {
        self.prepare_warm_with_identity(
            target,
            base.map(crate::session::AcknowledgedBase::bytes),
            schema_ready,
            warm_probe,
            |_| {
                base.map(crate::session::AcknowledgedBase::id)
                    .ok_or(Error::Integrity)
            },
            None,
        )
    }

    pub(crate) fn prepare_combined_acknowledged(
        &mut self,
        target: &[u8],
        base: Option<&crate::session::AcknowledgedBase>,
        prediction: &crate::shared_transfer::PreparedPrediction,
        schema_ready: bool,
        warm_probe: bool,
    ) -> Result<(PreparedWarmTransfer, CombinedSelection)> {
        let prepared = self.prepare_warm_with_identity(
            target,
            base.map(crate::session::AcknowledgedBase::bytes),
            schema_ready,
            warm_probe,
            |_| {
                base.map(crate::session::AcknowledgedBase::id)
                    .ok_or(Error::Integrity)
            },
            Some(prediction),
        )?;
        let selection = CombinedSelection {
            candidates: prepared.selection.candidates.clone(),
            pruned: prepared.selection.pruned.clone(),
            selected_mode: prepared.selection.selected_mode,
            exact_repeat_probe: false,
            checkpoint_ready: true,
            schema_ready,
        };
        Ok((prepared, selection))
    }

    fn prepare_warm_with_identity<F: FnOnce(&[u8]) -> Result<Id>>(
        &mut self,
        target: &[u8],
        base: Option<&[u8]>,
        schema_ready: bool,
        warm_probe: bool,
        base_identity: F,
        prediction: Option<&crate::shared_transfer::PreparedPrediction>,
    ) -> Result<PreparedWarmTransfer> {
        if target.len() > client::MAX_BATCHED_OBJECT
            || base.is_some_and(|b| b.len() > client::MAX_BATCHED_OBJECT)
        {
            return Err(Error::Limit);
        }
        let raw = candidate(Mode::Raw, raw_cost(target, warm_probe)?, 0)?;
        let mut selected = Mode::Raw;
        let mut best_cost = raw.total_http_body_bytes;
        let mut best = None;
        let mut candidates = vec![raw];
        let mut pruned = Vec::new();
        // Full equality, not a digest or hint, proves both residual calls have
        // identical inputs. Keep one recipe and derive Runtime's exact checked
        // cost, including its extra setup frame. Sparse strictly dominates it;
        // no Runtime payload copy is required in this branch.
        let shared_residual = match (prediction, base) {
            (Some(p), Some(b)) if b == p.prediction() => Some(crate::delta::encode(b, target)?),
            _ => None,
        };
        // The fixed prediction owns its setup and identity already. Only its
        // bounded residual is materialized; there is no replay or second session.
        if let Some(prediction) = prediction {
            if let Some(residual) = &shared_residual {
                if let Some(recipe) = residual {
                    let cost = selection::cost(
                        Mode::RuntimeV1,
                        std::iter::once(prediction.setup_len())
                            .chain(recipe.frames.iter().map(Vec::len)),
                        warm_probe,
                    )?;
                    candidates.push(candidate(Mode::RuntimeV1, cost, 0)?);
                }
            } else if let Some(bodies) = prediction.prepare(target)? {
                let cost =
                    selection::cost(Mode::RuntimeV1, bodies.iter().map(Vec::len), warm_probe)?;
                if warm_wins(Mode::RuntimeV1, cost, selected, best_cost) {
                    selected = Mode::RuntimeV1;
                    best_cost = cost;
                    best = Some(bodies);
                }
                candidates.push(candidate(Mode::RuntimeV1, cost, 0)?);
            }
        }
        if let Some(base) = base {
            let residual = match shared_residual {
                Some(residual) => residual,
                None => crate::delta::encode(base, target)?,
            };
            if let Some(recipe) = residual {
                let cost = selection::cost(
                    Mode::SparseV1,
                    recipe.frames.iter().map(Vec::len),
                    warm_probe,
                )?;
                if warm_wins(Mode::SparseV1, cost, selected, best_cost) {
                    selected = Mode::SparseV1;
                    best_cost = cost;
                    best = Some(recipe.frames);
                }
                candidates.push(candidate(Mode::SparseV1, cost, 0)?);
            }
        }
        for mode in [Mode::ZstdWholeV1, Mode::SchemaZstdV1] {
            if prediction.is_some() && mode == Mode::SchemaZstdV1 && !schema_ready {
                continue;
            }
            let setup = if mode == Mode::SchemaZstdV1 && !schema_ready {
                raw_cost(self.artifact(), false)?
            } else {
                0
            };
            // Whole's decoded block count is known. Schema's intermediate is
            // unknown here, but every checksummed frame has at least one block.
            let minimum = crate::zstd_prefix::minimum_encoded_size(if mode == Mode::ZstdWholeV1 {
                target.len()
            } else {
                0
            })?;
            let lower = selection::cost(mode, std::iter::once(minimum + 6), warm_probe)?;
            let total_lower = lower.checked_add(setup).ok_or(Error::Limit)?;
            if !warm_wins(mode, total_lower, selected, best_cost) {
                pruned.push(PrunedCandidate {
                    mode,
                    target_http_body_lower_bound: lower,
                    setup_http_body_bytes: setup,
                    total_http_body_lower_bound: total_lower,
                });
                continue;
            }
            if mode == Mode::ZstdWholeV1
                && first_block_probe_can_exclude(target.len(), warm_probe, selected, best_cost)?
                && !repeated_short_period_samples(target)
            {
                if let Some(bound) = self.whole.first_block_encoded_lower_bound(target)? {
                    let lower = encoded_length_cost(mode, bound, warm_probe)?;
                    if !warm_wins(mode, lower, selected, best_cost) {
                        pruned.push(PrunedCandidate {
                            mode,
                            target_http_body_lower_bound: lower,
                            setup_http_body_bytes: 0,
                            total_http_body_lower_bound: lower,
                        });
                        continue;
                    }
                }
            }
            let encoded = if mode == Mode::ZstdWholeV1 {
                self.whole.encode(target)?
            } else if prediction.is_some() {
                let intermediate = self.schema.prepare_intermediate(target)?;
                if let Some(bound) = self.prune_schema_intermediate(
                    intermediate.bytes(),
                    warm_probe,
                    selected,
                    best_cost,
                    setup,
                )? {
                    pruned.push(bound);
                    continue;
                }
                self.schema.encode_prepared(intermediate)?
            } else {
                self.schema.encode(target)?
            };
            let cost = candidate(mode, encoded_cost(mode, &encoded, warm_probe)?, setup)?;
            if warm_wins(mode, cost.total_http_body_bytes, selected, best_cost) {
                selected = mode;
                best_cost = cost.total_http_body_bytes;
                best = Some(fitted_transfer::chunks(mode, &encoded)?);
            }
            candidates.push(cost);
        }
        candidates.sort_by_key(|c| warm_priority(c.mode));
        pruned.sort_by_key(|c| warm_priority(c.mode));
        let payload = if let Some(bodies) = best {
            Payload::Encoded(bodies)
        } else {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(target.len())
                .map_err(|_| Error::Limit)?;
            bytes.extend_from_slice(target);
            Payload::Raw(bytes)
        };
        Ok(PreparedWarmTransfer {
            offer: Offer {
                mode: selected,
                len: target.len() as u64,
                object: prediction.map_or_else(|| object_id(target), |p| p.object_id_for(target)),
                manifest: None,
                base: match selected {
                    Mode::SparseV1 => Some(base_identity(base.ok_or(Error::Integrity)?)?),
                    Mode::SchemaZstdV1 => Some(self.artifact_id()),
                    Mode::RuntimeV1 => Some(prediction.ok_or(Error::Integrity)?.checkpoint_id()),
                    _ => None,
                },
            },
            payload,
            warm_probe,
            selection: WarmSelection {
                candidates,
                pruned,
                selected_mode: selected,
                needs_schema_seed: selected == Mode::SchemaZstdV1 && !schema_ready,
            },
        })
    }

    fn prune_schema_intermediate(
        &mut self,
        intermediate: &[u8],
        warm_probe: bool,
        incumbent: Mode,
        incumbent_bytes: u64,
        setup: u64,
    ) -> Result<Option<PrunedCandidate>> {
        // A valid schema stream may exceed the whole-object 16 MiB ceiling.
        // Fall back before calling its bound/probe: do not expand the qualified
        // source domain and do not turn a valid schema target into Error::Limit.
        if intermediate.len() > crate::zstd_prefix::MAX_PREFIX_OBJECT {
            return Ok(None);
        }
        let mode = Mode::SchemaZstdV1;
        let receipt = |encoded_len| -> Result<PrunedCandidate> {
            let lower = encoded_length_cost(mode, encoded_len, warm_probe)?;
            Ok(PrunedCandidate {
                mode,
                target_http_body_lower_bound: lower,
                setup_http_body_bytes: setup,
                total_http_body_lower_bound: lower.checked_add(setup).ok_or(Error::Limit)?,
            })
        };
        let lower = receipt(crate::zstd_prefix::minimum_encoded_size(
            intermediate.len(),
        )?)?;
        if !warm_wins(
            mode,
            lower.total_http_body_lower_bound,
            incumbent,
            incumbent_bytes,
        ) {
            return Ok(Some(lower));
        }
        let Some(maximum) = zstd_whole::first_block_maximum_bound(intermediate.len()) else {
            return Ok(None);
        };
        if warm_wins(
            mode,
            receipt(maximum)?.total_http_body_lower_bound,
            incumbent,
            incumbent_bytes,
        ) || repeated_short_period_samples(intermediate)
        {
            return Ok(None);
        }
        // Both prepared contexts use exactly new_encoder(): no dictionary or
        // prefix, same parameters, compress2 resets each frame. Reuse Whole's
        // session-resetting, source-qualified probe on the canonical bytes.
        // This adds no third context and never sends probe output as payload.
        if let Some(bound) = self.whole.first_block_encoded_lower_bound(intermediate)? {
            let lower = receipt(bound)?;
            if !warm_wins(
                mode,
                lower.total_http_body_lower_bound,
                incumbent,
                incumbent_bytes,
            ) {
                return Ok(Some(lower));
            }
        }
        Ok(None)
    }

    pub fn prepare(
        &mut self,
        target: &[u8],
        schema_ready: bool,
        warm_probe: bool,
    ) -> Result<PreparedTransfer> {
        if target.len() > client::MAX_BATCHED_OBJECT {
            return Err(Error::Limit);
        }
        let raw = raw_cost(target, warm_probe)?;
        let whole = self.whole.encode(target)?;
        let whole_cost = encoded_cost(Mode::ZstdWholeV1, &whole, warm_probe)?;
        let mut best = if whole_cost < raw {
            Some(whole)
        } else {
            drop(whole);
            None
        };
        let mut selected = if best.is_some() { 1 } else { 0 };
        let schema = self.schema.encode(target)?;
        let schema_cost = encoded_cost(Mode::SchemaZstdV1, &schema, warm_probe)?;
        let seed = if schema_ready {
            0
        } else {
            raw_cost(self.artifact(), false)?
        };
        let candidates = [
            candidate(Mode::Raw, raw, 0)?,
            candidate(Mode::ZstdWholeV1, whole_cost, 0)?,
            candidate(Mode::SchemaZstdV1, schema_cost, seed)?,
        ];
        // Fixed enumeration and strict comparison preserve Raw/Whole/Schema ties.
        if candidates[2].total_http_body_bytes < candidates[selected].total_http_body_bytes {
            best = Some(schema);
            selected = 2;
        } else {
            drop(schema);
        }
        let mode = candidates[selected].mode;
        let payload = if let Some(encoded) = best {
            Payload::Encoded(fitted_transfer::chunks(mode, &encoded)?)
        } else {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(target.len())
                .map_err(|_| Error::Limit)?;
            bytes.extend_from_slice(target);
            Payload::Raw(bytes)
        };
        Ok(PreparedTransfer {
            offer: Offer {
                mode,
                len: target.len() as u64,
                object: object_id(target),
                manifest: None,
                base: (mode == Mode::SchemaZstdV1).then(|| self.artifact_id()),
            },
            payload,
            warm_probe,
            selection: Selection {
                candidates,
                selected_mode: mode,
                needs_schema_seed: selected == 2 && !schema_ready,
            },
        })
    }
}
// CPU scheduling only: this screen never excludes a codec or supplies a bound.
// If these samples predict an inconclusive probe, use the unchanged full encoder.
fn repeated_short_period_samples(target: &[u8]) -> bool {
    if target.len() <= 128 * 1024 {
        return false;
    }
    let first = &target[..64];
    let short_period = [1, 2, 4, 8]
        .into_iter()
        .any(|period| first[period..] == first[..64 - period]);
    short_period && (1..8).all(|i| target[i * 16384..i * 16384 + 64] == *first)
}
fn warm_priority(mode: Mode) -> u8 {
    match mode {
        Mode::Raw => 0,
        Mode::ZstdWholeV1 => 1,
        Mode::SparseV1 => 2,
        Mode::RuntimeV1 => 3,
        Mode::SchemaZstdV1 => 4,
        _ => u8::MAX,
    }
}
fn warm_wins(mode: Mode, bytes: u64, incumbent: Mode, incumbent_bytes: u64) -> bool {
    (bytes, warm_priority(mode)) < (incumbent_bytes, warm_priority(incumbent))
}
fn candidate(mode: Mode, target: u64, setup: u64) -> Result<CandidateCost> {
    Ok(CandidateCost {
        mode,
        target_http_body_bytes: target,
        setup_http_body_bytes: setup,
        total_http_body_bytes: target.checked_add(setup).ok_or(Error::Limit)?,
    })
}
pub(crate) fn raw_cost(target: &[u8], warm_probe: bool) -> Result<u64> {
    selection::cost(
        Mode::Raw,
        target.chunks(MAX_CHUNK).map(|c| c.len() + 5),
        warm_probe,
    )
}
pub(crate) fn encoded_cost(mode: Mode, encoded: &[u8], warm_probe: bool) -> Result<u64> {
    selection::cost(
        mode,
        encoded.chunks(MAX_CHUNK).map(|c| c.len() + 6),
        warm_probe,
    )
}

fn encoded_length_cost(mode: Mode, len: usize, warm_probe: bool) -> Result<u64> {
    selection::cost(
        mode,
        (0..len)
            .step_by(MAX_CHUNK)
            .map(|offset| (len - offset).min(MAX_CHUNK) + 6),
        warm_probe,
    )
}

fn first_block_probe_can_exclude(
    target_len: usize,
    warm_probe: bool,
    incumbent: Mode,
    incumbent_bytes: u64,
) -> Result<bool> {
    let Some(maximum) = zstd_whole::first_block_maximum_bound(target_len) else {
        return Ok(false);
    };
    // If even the strongest possible certificate can win, no actual probe
    // certificate can exclude Whole. In particular, skip cold Raw incumbents.
    Ok(!warm_wins(
        Mode::ZstdWholeV1,
        encoded_length_cost(Mode::ZstdWholeV1, maximum, warm_probe)?,
        incumbent,
        incumbent_bytes,
    ))
}

#[cfg(test)]
mod probe_eligibility_tests {
    use super::*;

    #[test]
    fn impossible_certificates_skip_probe_and_earlier_whole_ties_still_encode() {
        assert!(!first_block_probe_can_exclude(128 << 10, false, Mode::SparseV1, 600).unwrap());
        let length: usize = 1 << 20;
        let raw = 368 + length as u64 + 133 * length.div_ceil(MAX_CHUNK) as u64;
        assert!(!first_block_probe_can_exclude(length, false, Mode::Raw, raw).unwrap());
        if !zstd_whole::first_block_probe_available() {
            assert!(!first_block_probe_can_exclude(length, false, Mode::SparseV1, 600).unwrap());
            return;
        }
        assert!(first_block_probe_can_exclude(length, false, Mode::SparseV1, 600).unwrap());
        let maximum = zstd_whole::first_block_maximum_bound(length).unwrap();
        let cost = encoded_length_cost(Mode::ZstdWholeV1, maximum, false).unwrap();
        assert!(!first_block_probe_can_exclude(length, false, Mode::SparseV1, cost).unwrap());
        assert!(!first_block_probe_can_exclude(length, false, Mode::SparseV1, cost + 1).unwrap());
        assert!(first_block_probe_can_exclude(length, false, Mode::Raw, cost).unwrap());
        assert!(warm_wins(Mode::ZstdWholeV1, 600, Mode::SparseV1, 600));
        assert!(!warm_wins(Mode::ZstdWholeV1, 600, Mode::Raw, 600));
    }

    #[test]
    fn computed_length_cost_matches_real_chunk_framing_without_materializing_bytes() {
        for length in [0, 1, MAX_CHUNK - 1, MAX_CHUNK, MAX_CHUNK + 1, 131109] {
            let bytes = vec![0; length];
            for probe in [false, true] {
                assert_eq!(
                    encoded_length_cost(Mode::ZstdWholeV1, length, probe),
                    encoded_cost(Mode::ZstdWholeV1, &bytes, probe)
                );
            }
        }
    }
}
