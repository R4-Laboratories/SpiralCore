//! Versioned, genuinely trained zstd dictionaries. Artifacts are inert bytes;
//! authority and final object identity are enforced by the transfer endpoint.
use crate::{protocol::Reader, zstd_prefix, Error, Result};
use zstd::zstd_safe::{self, CCtx, CParameter, DCtx};

pub const MAX_ARTIFACT: usize = 64 * 1024;
pub const MAX_TRAINING: usize = 4 * 1024 * 1024;
pub const MAX_SAMPLES: usize = 4096;

/// Deterministic with the pinned trainer, ordered samples and dictionary limit.
/// Training/serialization is explicit setup work, never performed on a target.
pub fn train(samples: &[&[u8]], max_size: usize) -> Result<Vec<u8>> {
    if samples.len() > MAX_SAMPLES || !(256..=MAX_ARTIFACT - 12).contains(&max_size) {
        return Err(Error::Limit);
    }
    let total = samples
        .iter()
        .try_fold(0usize, |n, s| n.checked_add(s.len()).ok_or(Error::Limit))?;
    if total > MAX_TRAINING {
        return Err(Error::Limit);
    }
    if total == 0 {
        return Err(Error::Malformed);
    }
    let dictionary = zstd::dict::from_samples(samples, max_size).map_err(|_| Error::Malformed)?;
    let mut artifact = b"SCZDICT1".to_vec();
    artifact.extend_from_slice(&(dictionary.len() as u32).to_le_bytes());
    artifact.extend_from_slice(&dictionary);
    parse(&artifact)?;
    Ok(artifact)
}

fn parse(artifact: &[u8]) -> Result<&[u8]> {
    if artifact.len() > MAX_ARTIFACT {
        return Err(Error::Limit);
    }
    let mut r = Reader::new(artifact);
    if r.take(8)? != b"SCZDICT1" {
        return Err(Error::Malformed);
    }
    let n = r.u32()? as usize;
    let dictionary = r.take(n)?;
    r.end()?;
    // A raw prefix is not a trained dictionary. Loading below also validates
    // entropy tables; this header check alone is not an integrity claim.
    if zstd_safe::get_dict_id_from_dict(dictionary).is_none() {
        return Err(Error::Malformed);
    }
    Ok(dictionary)
}

pub fn encode(artifact: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    let dictionary = parse(artifact)?;
    let cap = zstd_prefix::encoded_cap(target.len())?;
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
    encoder
        .load_dictionary(dictionary)
        .map_err(|_| Error::Malformed)?;
    encoder
        .compress2(&mut encoded, target)
        .map_err(|_| Error::Malformed)?;
    zstd_prefix::validate_frame(&encoded, target.len())?;
    Ok(encoded)
}

pub fn decode(artifact: &[u8], encoded: &[u8], target_len: usize) -> Result<Vec<u8>> {
    let dictionary = parse(artifact)?;
    zstd_prefix::validate_frame(encoded, target_len)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(target_len)
        .map_err(|_| Error::Limit)?;
    output.resize(target_len, 0);
    let mut decoder = DCtx::try_create().ok_or(Error::Limit)?;
    // Pinned one-shot decoder: <=64KiB copied dictionary + fixed entropy
    // tables, no streaming window allocation. Endpoint admits 4MiB before use.
    decoder
        .load_dictionary(dictionary)
        .map_err(|_| Error::Malformed)?;
    if decoder.sizeof() as u64 > zstd_prefix::DECODER_ALLOWANCE {
        return Err(Error::Limit);
    }
    let written = decoder
        .decompress(output.as_mut_slice(), encoded)
        .map_err(|_| Error::Integrity)?;
    if written != target_len {
        return Err(Error::Integrity);
    }
    if decoder.sizeof() as u64 > zstd_prefix::DECODER_ALLOWANCE {
        return Err(Error::Limit);
    }
    Ok(output)
}
