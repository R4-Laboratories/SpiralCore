//! No-artifact whole-object zstd, using the compressed-schema parameter profile.
use crate::{zstd_dictionary, zstd_prefix, Error, Result};
use zstd::zstd_safe::{self, CCtx, DCtx, InBuffer, OutBuffer, ResetDirective};

pub const ENCODER_ALLOWANCE: u64 = zstd_dictionary::ENCODER_ALLOWANCE;
pub const DECODER_ALLOWANCE: u64 = zstd_dictionary::DECODER_ALLOWANCE;

pub fn encoded_cap(len: usize) -> Result<usize> {
    zstd_prefix::encoded_cap(len)
}

const PROBE_BLOCK: usize = 128 * 1024;
const PROBE_HEADER: usize = 12;

/// Available only for the source-qualified bundled profile. Numeric version
/// alone is insufficient: build.rs also checks the exact bundled source tree.
pub fn first_block_probe_available() -> bool {
    qualified_probe_version(zstd_safe::version_number())
}
fn qualified_probe_version(version: u32) -> bool {
    cfg!(spiralcore_bundled_zstd_probe) && version == 10_507
}

// An upper bound on the strongest certificate this probe can ever produce.
// The caller skips probing when even this cannot exclude its incumbent.
pub(crate) fn first_block_maximum_bound(target_len: usize) -> Option<usize> {
    if !first_block_probe_available()
        || !(PROBE_BLOCK + 1..=zstd_prefix::MAX_PREFIX_OBJECT).contains(&target_len)
    {
        return None;
    }
    Some(probe_bound(PROBE_BLOCK, target_len))
}
fn probe_bound(payload: usize, target_len: usize) -> usize {
    PROBE_HEADER + payload + 4 + 3 * (target_len - PROBE_BLOCK).div_ceil(PROBE_BLOCK)
}
fn first_block_bound(header: &[u8], consumed: usize, target_len: usize) -> Option<usize> {
    if !(PROBE_BLOCK + 1..=zstd_prefix::MAX_PREFIX_OBJECT).contains(&target_len)
        || header.len() != PROBE_HEADER
        || consumed != PROBE_BLOCK
        || header[..5] != [0x28, 0xb5, 0x2f, 0xfd, 0xa4]
        || u32::from_le_bytes(header[5..9].try_into().ok()?) as usize != target_len
    {
        return None;
    }
    let block = u32::from_le_bytes([header[9], header[10], header[11], 0]);
    let size = (block >> 3) as usize;
    if block & 1 != 0 || size == 0 || size > PROBE_BLOCK {
        return None;
    }
    let payload = match (block >> 1) & 3 {
        0 if size == PROBE_BLOCK => size,
        1 if size == PROBE_BLOCK => 1,
        2 => size,
        _ => return None,
    };
    Some(probe_bound(payload, target_len))
}

pub struct PreparedEncoder {
    encoder: Option<CCtx<'static>>,
}
impl PreparedEncoder {
    pub fn new() -> Result<Self> {
        let encoder = zstd_dictionary::new_encoder()?;
        if encoder.sizeof() as u64 > ENCODER_ALLOWANCE {
            return Err(Error::Limit);
        }
        Ok(Self {
            encoder: Some(encoder),
        })
    }
    /// zstd-reported workspace; excludes Rust/allocator bookkeeping.
    pub fn retained_bytes(&self) -> usize {
        self.encoder.as_ref().map_or(0, CCtx::sizeof)
    }
    pub fn retained_charge(&self) -> u64 {
        ENCODER_ALLOWANCE
    }
    /// The first block only; no returned bytes can be sent as a candidate.
    /// Inconclusive shape returns None; actual codec/reset/resource errors fail.
    pub(crate) fn first_block_encoded_lower_bound(
        &mut self,
        target: &[u8],
    ) -> Result<Option<usize>> {
        encoded_cap(target.len())?;
        if first_block_maximum_bound(target.len()).is_none() {
            return Ok(None);
        }
        let encoder = self.encoder.as_mut().ok_or(Error::Limit)?;
        encoder
            .reset(ResetDirective::SessionOnly)
            .map_err(|_| Error::Malformed)?;
        let mut header = [0; PROBE_HEADER];
        let mut input = InBuffer::around(target);
        let (result, written) = {
            let mut output = OutBuffer::around(header.as_mut_slice());
            let result = encoder.compress_stream2(
                &mut output,
                &mut input,
                zstd_safe::zstd_sys::ZSTD_EndDirective::ZSTD_e_end,
            );
            (result, output.pos())
        };
        // Discard an unfinished frame even when the probe errored. Never reuse
        // an error state through another streaming call without this reset.
        let reset = encoder.reset(ResetDirective::SessionOnly);
        if encoder.sizeof() as u64 > ENCODER_ALLOWANCE {
            self.encoder = None;
            return Err(Error::Limit);
        }
        reset.map_err(|_| Error::Malformed)?;
        let remaining = result.map_err(|_| Error::Malformed)?;
        if remaining == 0 {
            return Ok(None);
        }
        Ok(first_block_bound(
            &header[..written],
            input.pos,
            target.len(),
        ))
    }
    pub fn encode(&mut self, target: &[u8]) -> Result<Vec<u8>> {
        let cap = encoded_cap(target.len())?;
        let encoder = self.encoder.as_mut().ok_or(Error::Limit)?;
        let mut encoded = Vec::new();
        encoded.try_reserve_exact(cap).map_err(|_| Error::Limit)?;
        let result = encoder.compress2(&mut encoded, target);
        if encoder.sizeof() as u64 > ENCODER_ALLOWANCE {
            self.encoder = None;
            return Err(Error::Limit);
        }
        result.map_err(|_| Error::Malformed)?;
        zstd_prefix::validate_frame(&encoded, target.len())?;
        Ok(encoded)
    }
}

pub struct PreparedDecoder {
    decoder: Option<DCtx<'static>>,
}
impl PreparedDecoder {
    pub fn new() -> Result<Self> {
        let decoder = DCtx::try_create().ok_or(Error::Limit)?;
        if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
            return Err(Error::Limit);
        }
        Ok(Self {
            decoder: Some(decoder),
        })
    }
    pub fn retained_bytes(&self) -> usize {
        self.decoder.as_ref().map_or(0, DCtx::sizeof)
    }
    pub fn retained_charge(&self) -> u64 {
        DECODER_ALLOWANCE
    }
    pub fn decode(&mut self, encoded: &[u8], len: usize) -> Result<Vec<u8>> {
        zstd_prefix::validate_frame(encoded, len)?;
        let decoder = self.decoder.as_mut().ok_or(Error::Limit)?;
        decoder
            .reset(ResetDirective::SessionOnly)
            .map_err(|_| Error::Malformed)?;
        let mut output = Vec::new();
        output.try_reserve_exact(len).map_err(|_| Error::Limit)?;
        output.resize(len, 0);
        let result = decoder.decompress(output.as_mut_slice(), encoded);
        if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
            self.decoder = None;
            return Err(Error::Limit);
        }
        if result.map_err(|_| Error::Integrity)? != len {
            return Err(Error::Integrity);
        }
        Ok(output)
    }
}

pub fn encode(target: &[u8]) -> Result<Vec<u8>> {
    PreparedEncoder::new()?.encode(target)
}
pub fn decode(encoded: &[u8], len: usize) -> Result<Vec<u8>> {
    PreparedDecoder::new()?.decode(encoded, len)
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    fn random(n: usize, seed: u64) -> Vec<u8> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }
    fn header(target: usize, kind: u32, size: usize) -> [u8; PROBE_HEADER] {
        let mut h = [0; PROBE_HEADER];
        h[..5].copy_from_slice(&[0x28, 0xb5, 0x2f, 0xfd, 0xa4]);
        h[5..9].copy_from_slice(&(target as u32).to_le_bytes());
        let block = (kind << 1) | ((size as u32) << 3);
        h[9..].copy_from_slice(&block.to_le_bytes()[..3]);
        h
    }

    #[test]
    fn certificate_parser_counts_only_actual_first_payload_and_required_remainder() {
        let n = 1 << 20;
        for (kind, size, payload) in [
            (0, PROBE_BLOCK, PROBE_BLOCK),
            (1, PROBE_BLOCK, 1),
            (2, 1, 1),
            (2, PROBE_BLOCK, PROBE_BLOCK),
        ] {
            assert_eq!(
                first_block_bound(&header(n, kind, size), PROBE_BLOCK, n),
                Some(12 + payload + 4 + 3 * 7)
            );
        }
        let h = header(n, 0, PROBE_BLOCK);
        for end in 0..PROBE_HEADER {
            assert_eq!(first_block_bound(&h[..end], PROBE_BLOCK, n), None);
        }
        let mut long = h.to_vec();
        long.push(0);
        assert_eq!(first_block_bound(&long, PROBE_BLOCK, n), None);
        for index in 0..9 {
            let mut bad = h;
            bad[index] ^= 1;
            assert_eq!(first_block_bound(&bad, PROBE_BLOCK, n), None);
        }
        for (kind, size) in [
            (3, 1),
            (0, 0),
            (0, PROBE_BLOCK - 1),
            (1, 1),
            (2, 0),
            (2, PROBE_BLOCK + 1),
        ] {
            assert_eq!(
                first_block_bound(&header(n, kind, size), PROBE_BLOCK, n),
                None
            );
        }
        let mut final_block = h;
        final_block[9] |= 1;
        assert_eq!(first_block_bound(&final_block, PROBE_BLOCK, n), None);
        for consumed in [0, PROBE_BLOCK - 1, PROBE_BLOCK + 1, n] {
            assert_eq!(first_block_bound(&h, consumed, n), None);
        }
        for n in [
            0,
            PROBE_BLOCK,
            zstd_prefix::MAX_PREFIX_OBJECT + 1,
            usize::MAX,
        ] {
            assert_eq!(first_block_bound(&h, PROBE_BLOCK, n), None);
        }
    }

    #[test]
    fn qualified_first_header_and_bound_match_uncapped_encoder_then_reset_exactly() {
        let mut encoder = PreparedEncoder::new().unwrap();
        let mut streaming = zstd_dictionary::new_encoder().unwrap();
        for len in [
            PROBE_BLOCK + 1,
            2 * PROBE_BLOCK - 1,
            2 * PROBE_BLOCK,
            2 * PROBE_BLOCK + 1,
            1 << 20,
        ] {
            let mut mixed = random(len, 0xb920_fa65_3d17_4491);
            mixed[PROBE_BLOCK / 2..].fill(0);
            let mut repeated = vec![0; len];
            let record = b"{\"key\":1,\"value\":\"exact\"}\n";
            for (i, byte) in repeated.iter_mut().enumerate() {
                *byte = record[i % record.len()];
            }
            for target in [
                random(len, 0xf15b_a843_62cd_7091),
                vec![0; len],
                repeated,
                mixed,
            ] {
                let reference = encode(&target).unwrap();
                if first_block_probe_available() {
                    streaming.reset(ResetDirective::SessionOnly).unwrap();
                    let mut actual_header = [0; PROBE_HEADER];
                    let mut input = InBuffer::around(&target);
                    let mut output = OutBuffer::around(actual_header.as_mut_slice());
                    let remaining = streaming
                        .compress_stream2(
                            &mut output,
                            &mut input,
                            zstd_safe::zstd_sys::ZSTD_EndDirective::ZSTD_e_end,
                        )
                        .unwrap();
                    assert!(remaining > 0);
                    assert_eq!(output.pos(), PROBE_HEADER);
                    assert_eq!(input.pos, PROBE_BLOCK);
                    assert_eq!(actual_header.as_slice(), &reference[..PROBE_HEADER]);
                    streaming.reset(ResetDirective::SessionOnly).unwrap();
                    let expected =
                        first_block_bound(&reference[..PROBE_HEADER], PROBE_BLOCK, len).unwrap();
                    assert!(expected <= reference.len());
                    for _ in 0..2 {
                        assert_eq!(
                            encoder.first_block_encoded_lower_bound(&target).unwrap(),
                            Some(expected)
                        );
                        assert_eq!(encoder.encode(&target).unwrap(), reference);
                    }
                } else {
                    assert_eq!(
                        encoder.first_block_encoded_lower_bound(&target).unwrap(),
                        None
                    );
                    assert_eq!(encoder.encode(&target).unwrap(), reference);
                }
                assert!(encoder.retained_bytes() as u64 <= encoder.retained_charge());
            }
        }
    }

    #[test]
    fn guard_version_size_and_unavailable_context_do_not_authorize_a_bound() {
        assert!(!qualified_probe_version(10_506));
        assert!(!qualified_probe_version(10_508));
        assert_eq!(
            qualified_probe_version(10_507),
            cfg!(spiralcore_bundled_zstd_probe)
        );
        for len in [
            0,
            1,
            PROBE_BLOCK,
            zstd_prefix::MAX_PREFIX_OBJECT + 1,
            usize::MAX,
        ] {
            assert_eq!(first_block_maximum_bound(len), None);
        }
        let mut unavailable = PreparedEncoder { encoder: None };
        for len in [0, 1, PROBE_BLOCK] {
            assert_eq!(
                unavailable
                    .first_block_encoded_lower_bound(&vec![0; len])
                    .unwrap(),
                None
            );
        }
        let target = vec![0; PROBE_BLOCK + 1];
        if first_block_probe_available() {
            assert_eq!(
                first_block_maximum_bound(target.len()),
                Some(12 + PROBE_BLOCK + 4 + 3)
            );
            assert_eq!(
                unavailable.first_block_encoded_lower_bound(&target),
                Err(Error::Limit)
            );
        } else {
            assert_eq!(
                unavailable
                    .first_block_encoded_lower_bound(&target)
                    .unwrap(),
                None
            );
        }
        assert_eq!(
            unavailable
                .first_block_encoded_lower_bound(&vec![0; zstd_prefix::MAX_PREFIX_OBJECT + 1]),
            Err(Error::Limit)
        );
    }
}
