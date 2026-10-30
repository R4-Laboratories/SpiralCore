//! No-artifact whole-object zstd, using the compressed-schema parameter profile.
use crate::{zstd_dictionary, zstd_prefix, Error, Result};
use zstd::zstd_safe::{CCtx, DCtx, ResetDirective};

pub const ENCODER_ALLOWANCE: u64 = zstd_dictionary::ENCODER_ALLOWANCE;
pub const DECODER_ALLOWANCE: u64 = zstd_dictionary::DECODER_ALLOWANCE;

pub fn encoded_cap(len: usize) -> Result<usize> {
    zstd_prefix::encoded_cap(len)
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
