//! Bounded version-one whole-base zstd prefix codec.
//!
//! This module does not authorize a base or establish an object identity. Endpoint
//! callers must reserve the buffers, verify the authorized base and verify the
//! resulting whole-object ID before publication. The decoder uses one-shot zstd,
//! never streaming decompression with an attacker-sized internal window buffer.
use crate::{protocol::Reader, Error, Result, MAX_CHUNK};
use zstd::zstd_safe::{self, CCtx, CParameter, DCtx};
pub const MAX_PREFIX_OBJECT: usize = 16 * 1024 * 1024;
pub const MAX_PREFIX_CHUNKS: usize = 512;
pub const DECODER_ALLOWANCE: u64 = 4 * 1024 * 1024;
const MAX_WINDOW: u64 = 32 * 1024 * 1024;

pub fn encoded_cap(target_len: usize) -> Result<usize> {
    if target_len > MAX_PREFIX_OBJECT {
        return Err(Error::Limit);
    }
    Ok(zstd_safe::compress_bound(target_len))
}

/// Prepare one frame against a whole immutable local base, using fixed settings.
/// The caller includes this work in sender CPU/wall measurements, including when
/// the frame is subsequently discarded in favor of Raw before Begin.
pub fn encode(base: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    let cap = encoded_cap(target.len())?;
    if base.len() > MAX_PREFIX_OBJECT {
        return Err(Error::Limit);
    }
    let mut encoded = Vec::new();
    encoded.try_reserve_exact(cap).map_err(|_| Error::Limit)?;
    let mut encoder = CCtx::try_create().ok_or(Error::Limit)?;
    for p in [
        CParameter::CompressionLevel(3),
        CParameter::WindowLog(25),
        CParameter::EnableLongDistanceMatching(true),
        CParameter::NbWorkers(0),
        CParameter::ContentSizeFlag(true),
        CParameter::ChecksumFlag(true),
        CParameter::DictIdFlag(false),
    ] {
        encoder.set_parameter(p).map_err(|_| Error::Unsupported)?;
    }
    encoder.ref_prefix(base).map_err(|_| Error::Malformed)?;
    encoder
        .compress2(&mut encoded, target)
        .map_err(|_| Error::Malformed)?;
    validate_frame(&encoded, target.len())?;
    Ok(encoded)
}

fn validate_frame(encoded: &[u8], target_len: usize) -> Result<()> {
    if encoded.len() > encoded_cap(target_len)? {
        return Err(Error::Limit);
    }
    if encoded.get(..4) != Some(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Err(Error::Malformed);
    }
    let descriptor = *encoded.get(4).ok_or(Error::Malformed)?;
    // Reserved/unused bits and any dictionary-ID field are forbidden; this
    // profile always includes a checksum. Prefix identity is carried in Offer.
    if descriptor & 0x1b != 0 || descriptor & 4 == 0 {
        return Err(Error::Malformed);
    }
    if descriptor & 0x20 == 0 {
        let wd = *encoded.get(5).ok_or(Error::Malformed)?;
        let base = 1u64
            .checked_shl(10 + u32::from(wd >> 3))
            .ok_or(Error::Limit)?;
        let window = base + (base >> 3) * u64::from(wd & 7);
        if window > MAX_WINDOW {
            return Err(Error::Limit);
        }
    }
    let size = zstd_safe::get_frame_content_size(encoded).map_err(|_| Error::Malformed)?;
    if size != Some(target_len as u64) {
        return Err(Error::Integrity);
    }
    // A single-segment frame uses its content size as the window. The checked
    // target bound is smaller than MAX_WINDOW. No context is allocated above.
    if zstd_safe::find_frame_compressed_size(encoded).map_err(|_| Error::Malformed)?
        != encoded.len()
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// Decode exactly one profile frame into a bounded output. Whole-object identity
/// and authorization remain the endpoint's responsibility.
pub fn decode(base: &[u8], encoded: &[u8], target_len: usize) -> Result<Vec<u8>> {
    if base.len() > MAX_PREFIX_OBJECT {
        return Err(Error::Limit);
    }
    validate_frame(encoded, target_len)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(target_len)
        .map_err(|_| Error::Limit)?;
    output.resize(target_len, 0);
    // Pinned zstd 1.5.7: DCtx is fixed-sized, raw-content refPrefix creates a
    // by-reference fixed DDict, and decompressDCtx writes to our output. It
    // never allocates the streaming inBuff/outBuff based on the frame window.
    // The 4 MiB admitted allowance is conservative for these fixed tables;
    // sizeof is an additional drift guard, not a post-hoc allocator quota.
    let mut decoder = DCtx::try_create().ok_or(Error::Limit)?;
    decoder.ref_prefix(base).map_err(|_| Error::Malformed)?;
    if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
        return Err(Error::Limit);
    }
    // Pass the exact length slice, not allocator capacity, as zstd's output cap.
    let written = decoder
        .decompress(output.as_mut_slice(), encoded)
        .map_err(|_| Error::Integrity)?;
    if written != target_len || output.len() != target_len {
        return Err(Error::Integrity);
    }
    if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
        return Err(Error::Limit);
    }
    Ok(output)
}

pub(crate) fn chunks(encoded: &[u8]) -> Result<Vec<Vec<u8>>> {
    if encoded.is_empty() || encoded.len().div_ceil(MAX_CHUNK) > MAX_PREFIX_CHUNKS {
        return Err(Error::Limit);
    }
    Ok(encoded
        .chunks(MAX_CHUNK)
        .map(|chunk| {
            let mut body = Vec::with_capacity(chunk.len() + 6);
            body.push(4);
            body.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            body.push(1);
            body.extend_from_slice(chunk);
            body
        })
        .collect())
}

pub(crate) fn append(
    encoded: &mut Vec<u8>,
    count: &mut usize,
    target_len: usize,
    body: &[u8],
) -> Result<()> {
    let mut r = Reader::new(body);
    if r.u8()? != 4 {
        return Err(Error::Malformed);
    }
    let n = r.u32()? as usize;
    if r.u8()? != 1 {
        return Err(Error::Unsupported);
    }
    if n == 0 || n > MAX_CHUNK || *count >= MAX_PREFIX_CHUNKS {
        return Err(Error::Limit);
    }
    let chunk = r.take(n)?;
    r.end()?;
    if encoded.len().checked_add(n).ok_or(Error::Limit)? > encoded_cap(target_len)? {
        return Err(Error::Limit);
    }
    encoded.extend_from_slice(chunk);
    *count += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_validation_is_atomic_and_count_and_encoded_bounds_are_independent() {
        let good = chunks(&[7]).unwrap().remove(0);
        let mut encoded = Vec::new();
        let mut count = 0;
        for _ in 0..MAX_PREFIX_CHUNKS {
            append(&mut encoded, &mut count, 4096, &good).unwrap();
        }
        let saved = encoded.clone();
        assert_eq!(
            append(&mut encoded, &mut count, 4096, &good),
            Err(Error::Limit)
        );
        assert_eq!(encoded, saved);
        assert_eq!(count, MAX_PREFIX_CHUNKS);
        let mut encoded = Vec::new();
        let mut count = 0;
        let cap = encoded_cap(0).unwrap();
        append(
            &mut encoded,
            &mut count,
            0,
            &chunks(&vec![7; cap]).unwrap()[0],
        )
        .unwrap();
        assert_eq!(
            append(&mut encoded, &mut count, 0, &good),
            Err(Error::Limit)
        );
        assert_eq!(encoded.len(), cap);
        assert_eq!(count, 1);
        for bad in [
            vec![4, 2, 0, 0, 0, 1, 7],
            vec![4, 1, 0, 0, 0, 2, 7],
            vec![4, 1, 0, 0, 0, 1, 7, 8],
            vec![0, 1, 0, 0, 0, 1, 7],
        ] {
            let mut encoded = vec![9];
            let mut count = 1;
            assert!(append(&mut encoded, &mut count, 4096, &bad).is_err());
            assert_eq!(encoded, vec![9]);
            assert_eq!(count, 1);
        }
        assert_eq!(
            encoded_cap(MAX_PREFIX_OBJECT).unwrap(),
            MAX_PREFIX_OBJECT + 65536
        );
    }
}
