use crate::{Binding, Error, HEADER_LEN, MAX_BLOCK, Result};
use spiralcore_transfer::{
    object_id,
    protocol::{Mode, Offer},
    zstd_whole,
};

pub(crate) const DATA: u8 = 1;
pub(crate) const FIN: u8 = 2;
pub(crate) const RESET: u8 = 3;
pub(crate) const CANCEL: u8 = 4;
pub(crate) const OBJECT: u8 = 6;
pub(crate) const ARTIFACT: u8 = 7;
pub(crate) const APP_FIN: u8 = 8;
const MAGIC: &[u8; 8] = b"SCFLW001";

pub(crate) fn body_limit() -> Result<usize> {
    let actual_offer = Offer::cold(&[], Mode::Raw).encode()?.len();
    actual_offer.checked_add(2 + MAX_BLOCK).ok_or(Error::Limit)
}
pub(crate) fn header(
    binding: Binding,
    sequence: u64,
    kind: u8,
    len: usize,
    role: u8,
) -> Result<Vec<u8>> {
    let len = u32::try_from(len).map_err(|_| Error::Limit)?;
    let mut bytes = Vec::with_capacity(HEADER_LEN);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&[kind, role, 0, 0]);
    bytes.extend_from_slice(&spiralcore_transfer::hash(
        b"spiralcore.flow.direction.v1\0",
        &[&binding.flow, &[role]],
    ));
    bytes.extend_from_slice(&binding.policy_sha256);
    bytes.extend_from_slice(&binding.scope_commitment);
    bytes.extend_from_slice(&sequence.to_be_bytes());
    bytes.extend_from_slice(&len.to_be_bytes());
    Ok(bytes)
}
pub(crate) fn parse_header(
    bytes: &[u8; HEADER_LEN],
    binding: Binding,
    sequence: u64,
    cap: usize,
    data_kind: u8,
    role: u8,
) -> Result<(u8, usize)> {
    if &bytes[..8] != MAGIC
        || bytes[9] != role
        || bytes[10..12] != [0; 2]
        || !(bytes[8] == data_kind
            || matches!(bytes[8], FIN | RESET | CANCEL)
            || data_kind == OBJECT && matches!(bytes[8], ARTIFACT | APP_FIN))
    {
        return Err(Error::Protocol);
    }
    if bytes[12..44]
        != spiralcore_transfer::hash(b"spiralcore.flow.direction.v1\0", &[&binding.flow, &[role]])
        || bytes[44..76] != binding.policy_sha256
        || bytes[76..108] != binding.scope_commitment
    {
        return Err(Error::InvalidBinding);
    }
    if bytes[108..116] != sequence.to_be_bytes() {
        return Err(Error::Sequence);
    }
    let len = u32::from_be_bytes(bytes[116..120].try_into().expect("fixed header")) as usize;
    if len > cap {
        return Err(Error::Limit);
    }
    if (bytes[8] == data_kind || data_kind == OBJECT && bytes[8] == ARTIFACT) == (len == 0) {
        return Err(Error::Protocol);
    }
    Ok((bytes[8], len))
}
pub(crate) fn encode(
    encoder: &mut zstd_whole::PreparedEncoder,
    bytes: &[u8],
) -> Result<(Vec<u8>, bool)> {
    if bytes.is_empty() || bytes.len() > MAX_BLOCK {
        return Err(Error::Limit);
    }
    let compressed = encoder.encode(bytes)?;
    let use_zstd = compressed.len() < bytes.len();
    let (mode, payload) = if use_zstd {
        (Mode::ZstdWholeV1, compressed.as_slice())
    } else {
        (Mode::Raw, bytes)
    };
    let offer = Offer::cold(bytes, mode).encode()?;
    let mut body = Vec::with_capacity(2 + offer.len() + payload.len());
    body.extend_from_slice(
        &u16::try_from(offer.len())
            .map_err(|_| Error::Limit)?
            .to_be_bytes(),
    );
    body.extend_from_slice(&offer);
    body.extend_from_slice(payload);
    Ok((body, use_zstd))
}
pub(crate) fn decode(
    decoder: &mut zstd_whole::PreparedDecoder,
    body: &[u8],
) -> Result<(Vec<u8>, bool)> {
    let len = body.get(..2).ok_or(Error::Protocol)?;
    let n = usize::from(u16::from_be_bytes([len[0], len[1]]));
    let end = 2usize.checked_add(n).ok_or(Error::Limit)?;
    let offer = Offer::decode(body.get(2..end).ok_or(Error::Protocol)?)?;
    let decoded_len = usize::try_from(offer.len).map_err(|_| Error::Limit)?;
    if !(1..=MAX_BLOCK).contains(&decoded_len) || offer.manifest.is_some() || offer.base.is_some() {
        return Err(Error::Limit);
    }
    let payload = body.get(end..).ok_or(Error::Protocol)?;
    let compressed = offer.mode == Mode::ZstdWholeV1;
    let decoded = match offer.mode {
        Mode::Raw if payload.len() == decoded_len => payload.to_vec(),
        Mode::ZstdWholeV1 if payload.len() < decoded_len => decoder.decode(payload, decoded_len)?,
        _ => return Err(Error::Protocol),
    };
    if object_id(&decoded) != offer.object {
        return Err(Error::Integrity);
    }
    Ok((decoded, compressed))
}
