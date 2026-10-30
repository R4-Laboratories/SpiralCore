//! Exact successful-miss HTTP-body selection. Readiness is caller knowledge,
//! never authorization. Cold selection charges a complete separate Raw seed.
use crate::{
    client, fitted_transfer, object_id,
    protocol::{Mode, Offer},
    schema_zstd, selection, zstd_whole, Error, Id, Result, MAX_CHUNK,
};

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
        if let Some(base) = base {
            if let Some(recipe) = crate::delta::encode(base, target)? {
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
            let encoded = if mode == Mode::ZstdWholeV1 {
                self.whole.encode(target)?
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
                object: object_id(target),
                manifest: None,
                base: match selected {
                    Mode::SparseV1 => Some(object_id(base.ok_or(Error::Integrity)?)),
                    Mode::SchemaZstdV1 => Some(self.artifact_id()),
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
fn warm_priority(mode: Mode) -> u8 {
    match mode {
        Mode::Raw => 0,
        Mode::ZstdWholeV1 => 1,
        Mode::SparseV1 => 2,
        Mode::SchemaZstdV1 => 3,
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
