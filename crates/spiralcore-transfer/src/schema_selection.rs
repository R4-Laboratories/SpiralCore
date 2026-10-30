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
