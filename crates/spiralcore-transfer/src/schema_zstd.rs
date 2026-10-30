//! One strict checksummed zstd frame over the existing exact SCVAL001 stream.
//! Prepared contexts retain no previous target as a dictionary or implicit base.
use crate::{schema, zstd_dictionary, Error, Id, Result};
use zstd::zstd_safe::{self, CCtx, DCtx, ResetDirective};

pub const DECODER_ALLOWANCE: u64 = zstd_dictionary::DECODER_ALLOWANCE;
pub const ENCODER_ALLOWANCE: u64 = zstd_dictionary::ENCODER_ALLOWANCE;
const MAX_WINDOW: u64 = 32 * 1024 * 1024;

pub fn encoded_cap(target_len: usize) -> Result<usize> {
    Ok(zstd_safe::compress_bound(schema::encoded_cap(target_len)?))
}

pub struct PreparedEncoder {
    schema: schema::PreparedSchema,
    encoder: Option<CCtx<'static>>,
}

impl PreparedEncoder {
    pub fn new(artifact: &[u8]) -> Result<Self> {
        let schema = schema::PreparedSchema::new(artifact)?;
        let encoder = zstd_dictionary::new_encoder()?;
        if encoder.sizeof() as u64 > ENCODER_ALLOWANCE {
            return Err(Error::Limit);
        }
        Ok(Self {
            schema,
            encoder: Some(encoder),
        })
    }
    pub fn artifact(&self) -> &[u8] {
        self.schema.artifact()
    }
    pub fn artifact_id(&self) -> Id {
        self.schema.artifact_id()
    }
    /// Owned payload bytes plus zstd-reported context size, excluding Rust and
    /// allocator metadata. The conservative quota is retained_charge().
    pub fn retained_bytes(&self) -> usize {
        self.schema.retained_bytes() + self.encoder.as_ref().map_or(0, CCtx::sizeof)
    }
    pub fn retained_charge(&self) -> u64 {
        schema::PREPARED_CHARGE + ENCODER_ALLOWANCE
    }
    pub fn encode(&mut self, target: &[u8]) -> Result<Vec<u8>> {
        let intermediate = self.schema.encode(target)?;
        if intermediate.len() > schema::encoded_cap(target.len())? {
            return Err(Error::Limit);
        }
        let encoder = self.encoder.as_mut().ok_or(Error::Limit)?;
        let mut encoded = Vec::new();
        encoded
            .try_reserve_exact(zstd_safe::compress_bound(intermediate.len()))
            .map_err(|_| Error::Limit)?;
        let result = encoder.compress2(&mut encoded, &intermediate);
        if encoder.sizeof() as u64 > ENCODER_ALLOWANCE {
            self.encoder = None;
            return Err(Error::Limit);
        }
        result.map_err(|_| Error::Malformed)?;
        if frame_size(&encoded, target.len())? != intermediate.len() {
            return Err(Error::Integrity);
        }
        Ok(encoded)
    }
}

pub struct PreparedDecoder {
    schema: schema::PreparedSchema,
    decoder: Option<DCtx<'static>>,
}

impl PreparedDecoder {
    pub fn new(artifact: &[u8]) -> Result<Self> {
        let schema = schema::PreparedSchema::new(artifact)?;
        let decoder = DCtx::try_create().ok_or(Error::Limit)?;
        if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
            return Err(Error::Limit);
        }
        Ok(Self {
            schema,
            decoder: Some(decoder),
        })
    }
    pub fn artifact(&self) -> &[u8] {
        self.schema.artifact()
    }
    pub fn artifact_id(&self) -> Id {
        self.schema.artifact_id()
    }
    /// Owned payload bytes plus zstd-reported context size, excluding Rust and
    /// allocator metadata. The conservative quota is retained_charge().
    pub fn retained_bytes(&self) -> usize {
        self.schema.retained_bytes() + self.decoder.as_ref().map_or(0, DCtx::sizeof)
    }
    pub fn retained_charge(&self) -> u64 {
        schema::PREPARED_CHARGE + DECODER_ALLOWANCE
    }
    pub fn decode(&mut self, encoded: &[u8], target_len: usize) -> Result<Vec<u8>> {
        let len = frame_size(encoded, target_len)?;
        let decoder = self.decoder.as_mut().ok_or(Error::Limit)?;
        decoder
            .reset(ResetDirective::SessionOnly)
            .map_err(|_| Error::Malformed)?;
        let mut intermediate = Vec::new();
        intermediate
            .try_reserve_exact(len)
            .map_err(|_| Error::Limit)?;
        intermediate.resize(len, 0);
        let result = decoder.decompress(intermediate.as_mut_slice(), encoded);
        if decoder.sizeof() as u64 > DECODER_ALLOWANCE {
            self.decoder = None;
            return Err(Error::Limit);
        }
        if result.map_err(|_| Error::Integrity)? != len {
            return Err(Error::Integrity);
        }
        self.schema.decode(&intermediate, target_len)
    }
}

pub fn encode(artifact: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    PreparedEncoder::new(artifact)?.encode(target)
}

pub fn decode(artifact: &[u8], encoded: &[u8], target_len: usize) -> Result<Vec<u8>> {
    PreparedDecoder::new(artifact)?.decode(encoded, target_len)
}

// Keep this target-specific intermediate profile separate from legacy prefix's
// 16MiB decoded-object cap. Validate every bound before allocating intermediate.
fn frame_size(encoded: &[u8], target_len: usize) -> Result<usize> {
    let cap = schema::encoded_cap(target_len)?;
    if encoded.len() > zstd_safe::compress_bound(cap) {
        return Err(Error::Limit);
    }
    if encoded.get(..4) != Some(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Err(Error::Malformed);
    }
    let descriptor = *encoded.get(4).ok_or(Error::Malformed)?;
    if descriptor & 0x1b != 0 || descriptor & 4 == 0 {
        return Err(Error::Malformed);
    }
    if descriptor & 0x20 == 0 {
        let wd = *encoded.get(5).ok_or(Error::Malformed)?;
        let base = 1u64
            .checked_shl(10 + u32::from(wd >> 3))
            .ok_or(Error::Limit)?;
        if base + (base >> 3) * u64::from(wd & 7) > MAX_WINDOW {
            return Err(Error::Limit);
        }
    }
    let len = zstd_safe::get_frame_content_size(encoded)
        .map_err(|_| Error::Malformed)?
        .ok_or(Error::Malformed)?;
    if len < 48 || len > cap as u64 {
        return Err(Error::Limit);
    }
    let len = len as usize;
    if encoded.len() > zstd_safe::compress_bound(len) {
        return Err(Error::Limit);
    }
    if zstd_safe::find_frame_compressed_size(encoded).map_err(|_| Error::Malformed)?
        != encoded.len()
    {
        return Err(Error::Malformed);
    }
    Ok(len)
}
