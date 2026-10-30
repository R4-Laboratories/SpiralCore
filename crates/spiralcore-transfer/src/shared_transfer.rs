//! Checkpoint replay followed by exact COPY/LITERAL residuals.
//!
//! This is an application codec, not authorization. The endpoint must admit the
//! verified checkpoint in the transfer's namespace, reserve replay/output work,
//! and verify the final object before publication. Received code is never run.
use crate::{delta, object_id, protocol::Reader, shared_runtime, Error, Result};

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
    let encoded = recipe.encode()?;
    if encoded.len() > 64 {
        return Err(Error::Limit);
    }
    // Decoded byte count is zero: this frame prepares a base, not object output.
    let mut setup = vec![5, 0, 0, 0, 0];
    setup.extend_from_slice(&1u16.to_le_bytes());
    setup.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
    setup.extend_from_slice(&encoded);
    setup.extend_from_slice(&(prediction.len() as u32).to_le_bytes());
    setup.extend_from_slice(&object_id(&prediction));
    let mut frames = vec![setup];
    frames.extend(residual.frames);
    Ok(Some(frames))
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
