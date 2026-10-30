//! Checkpoint replay followed by exact COPY/LITERAL residuals.
//!
//! This is an application codec, not authorization. The endpoint must admit the
//! verified checkpoint in the transfer's namespace, reserve replay/output work,
//! and verify the final object before publication. Received code is never run.
use crate::{delta, object_id, protocol::Reader, shared_runtime, Error, Result};

/// An immutable sender-owned prediction, reusable across related target objects.
///
/// Construction pays replay and prediction hashing once. Each send still runs
/// byte-first selection and exact residual encoding; the receiver still checks
/// authorization and reconstructs/verifies the object. Possession is not a
/// capability. There is no implicit global cache or unbounded recipe search.
/// Callers own retention and should charge [`Self::retained_bytes`] to their
/// memory budget. Each value retains at most MAX_OUTPUT + MAX_CHECKPOINT + 69
/// bytes of buffers; constructing multiple values requires a caller-wide budget.
pub struct PreparedPrediction {
    checkpoint: Box<[u8]>,
    prediction: Box<[u8]>,
    setup: Box<[u8]>,
}

impl PreparedPrediction {
    pub fn new(checkpoint: &[u8], recipe: &shared_runtime::Recipe) -> Result<Self> {
        if checkpoint.len() > shared_runtime::MAX_CHECKPOINT {
            return Err(Error::Limit);
        }
        let prediction = shared_runtime::replay(checkpoint, recipe, shared_runtime::MAX_OUTPUT)?;
        let setup = setup_frame(recipe, &prediction)?;
        Ok(Self {
            checkpoint: checkpoint.into(),
            prediction: prediction.into_boxed_slice(),
            setup: setup.into_boxed_slice(),
        })
    }

    pub fn checkpoint(&self) -> &[u8] {
        &self.checkpoint
    }

    pub fn prediction(&self) -> &[u8] {
        &self.prediction
    }

    /// Retained buffer bytes, excluding allocator/struct metadata and send scratch.
    pub fn retained_bytes(&self) -> usize {
        self.checkpoint.len() + self.prediction.len() + self.setup.len()
    }

    /// Encode a target against the prepared prediction without replaying it.
    pub fn prepare(&self, target: &[u8]) -> Result<Option<Vec<Vec<u8>>>> {
        if target.len() > shared_runtime::MAX_OUTPUT {
            return Err(Error::Limit);
        }
        let Some(residual) = delta::encode(&self.prediction, target)? else {
            return Ok(None);
        };
        let mut frames = Vec::with_capacity(residual.frames.len() + 1);
        frames.push(self.setup.to_vec());
        frames.extend(residual.frames);
        Ok(Some(frames))
    }
}

/// Prepare the replay and residual locally, without contacting a receiver.
/// None means the existing bounded residual encoder already prefers raw bytes.
pub fn prepare(
    checkpoint: &[u8],
    target: &[u8],
    recipe: &shared_runtime::Recipe,
) -> Result<Option<Vec<Vec<u8>>>> {
    if checkpoint.len() > shared_runtime::MAX_CHECKPOINT
        || target.len() > shared_runtime::MAX_OUTPUT
    {
        return Err(Error::Limit);
    }
    let prediction = shared_runtime::replay(checkpoint, recipe, shared_runtime::MAX_OUTPUT)?;
    let Some(residual) = delta::encode(&prediction, target)? else {
        return Ok(None);
    };
    // One-shot callers need not retain/copy the checkpoint or hash a prediction
    // when the residual encoder rejects it. Prepared callers pay this once.
    let mut frames = Vec::with_capacity(residual.frames.len() + 1);
    frames.push(setup_frame(recipe, &prediction)?);
    frames.extend(residual.frames);
    Ok(Some(frames))
}

fn setup_frame(recipe: &shared_runtime::Recipe, prediction: &[u8]) -> Result<Vec<u8>> {
    let encoded = recipe.encode()?;
    // Decoded byte count is zero: setup prepares a base, not object output.
    let mut setup = vec![5, 0, 0, 0, 0];
    setup.extend_from_slice(&1u16.to_le_bytes());
    setup.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
    setup.extend_from_slice(&encoded);
    setup.extend_from_slice(&(prediction.len() as u32).to_le_bytes());
    setup.extend_from_slice(&object_id(prediction));
    Ok(setup)
}

/// Validate and regenerate one bounded prediction. Caller owns authorization.
pub fn replay_frame(checkpoint: &[u8], body: &[u8]) -> Result<Vec<u8>> {
    if checkpoint.len() > shared_runtime::MAX_CHECKPOINT {
        return Err(Error::Limit);
    }
    let mut r = Reader::new(body);
    if r.u8()? != 5 || r.u32()? != 0 {
        return Err(Error::Malformed);
    }
    if r.u16()? != 1 {
        return Err(Error::Unsupported);
    }
    let n = r.u16()? as usize;
    if n > 64 {
        return Err(Error::Limit);
    }
    let recipe = shared_runtime::Recipe::decode(r.take(n)?)?;
    let len = r.u32()? as usize;
    if len > shared_runtime::MAX_OUTPUT {
        return Err(Error::Limit);
    }
    let expected = r.id()?;
    r.end()?;
    let prediction = shared_runtime::replay(checkpoint, &recipe, len)?;
    if prediction.len() != len || object_id(&prediction) != expected {
        return Err(Error::Integrity);
    }
    Ok(prediction)
}
