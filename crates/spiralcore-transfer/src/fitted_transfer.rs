//! Shared framing for the two bounded fitted-artifact profiles.
use crate::{
    protocol::{Mode, Reader},
    Error, Result, MAX_CHUNK,
};

pub(crate) fn is_fitted(mode: Mode) -> bool {
    matches!(mode, Mode::SchemaV1 | Mode::ZstdDictionaryV1)
}

pub(crate) fn encoded_cap(mode: Mode, len: usize) -> Result<usize> {
    if len > crate::client::MAX_BATCHED_OBJECT {
        return Err(Error::Limit);
    }
    match mode {
        Mode::SchemaV1 => Ok(len + 5 * len.min(65536) + 48),
        Mode::ZstdDictionaryV1 => crate::zstd_prefix::encoded_cap(len),
        _ => Err(Error::Unsupported),
    }
}

pub(crate) fn chunks(mode: Mode, encoded: &[u8]) -> Result<Vec<Vec<u8>>> {
    if !is_fitted(mode) {
        return Err(Error::Unsupported);
    }
    if encoded.is_empty() || encoded.len().div_ceil(MAX_CHUNK) > 512 {
        return Err(Error::Limit);
    }
    Ok(encoded
        .chunks(MAX_CHUNK)
        .map(|chunk| {
            let mut body = Vec::with_capacity(chunk.len() + 6);
            body.push(mode as u8);
            body.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            body.push(1);
            body.extend_from_slice(chunk);
            body
        })
        .collect())
}

pub(crate) fn append(
    mode: Mode,
    encoded: &mut Vec<u8>,
    count: &mut usize,
    len: usize,
    body: &[u8],
) -> Result<()> {
    let cap = encoded_cap(mode, len)?;
    let mut r = Reader::new(body);
    if r.u8()? != mode as u8 {
        return Err(Error::Malformed);
    }
    let n = r.u32()? as usize;
    if r.u8()? != 1 {
        return Err(Error::Unsupported);
    }
    if n == 0 || n > MAX_CHUNK || *count >= 512 {
        return Err(Error::Limit);
    }
    let chunk = r.take(n)?;
    r.end()?;
    if encoded.len().checked_add(n).ok_or(Error::Limit)? > cap {
        return Err(Error::Limit);
    }
    encoded.extend_from_slice(chunk);
    *count += 1;
    Ok(())
}

pub(crate) fn decode(mode: Mode, artifact: &[u8], encoded: &[u8], len: usize) -> Result<Vec<u8>> {
    match mode {
        Mode::SchemaV1 => crate::schema::decode(artifact, encoded, len),
        Mode::ZstdDictionaryV1 => crate::zstd_dictionary::decode(artifact, encoded, len),
        _ => Err(Error::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_validation_is_atomic_and_separately_bounds_count_and_bytes() {
        for mode in [Mode::SchemaV1, Mode::ZstdDictionaryV1] {
            let body = chunks(mode, &[7]).unwrap().remove(0);
            let mut bytes = vec![];
            let mut count = 0;
            for _ in 0..512 {
                append(mode, &mut bytes, &mut count, 65536, &body).unwrap();
            }
            assert_eq!(
                append(mode, &mut bytes, &mut count, 65536, &body),
                Err(Error::Limit)
            );
            assert_eq!(bytes, vec![7; 512]);
            assert_eq!(count, 512);
            let mut count = 0;
            let mut bytes = vec![0; encoded_cap(mode, 1).unwrap()];
            assert_eq!(
                append(mode, &mut bytes, &mut count, 1, &body),
                Err(Error::Limit)
            );
            assert_eq!(count, 0);
            bytes.clear();
            for bad in [
                body[..body.len() - 1].to_vec(),
                [body.clone(), vec![0]].concat(),
            ] {
                assert_eq!(
                    append(mode, &mut bytes, &mut count, 1, &bad),
                    Err(Error::Malformed)
                );
                assert!(bytes.is_empty());
            }
            let mut bad = body.clone();
            bad[0] = 0;
            assert_eq!(
                append(mode, &mut bytes, &mut count, 1, &bad),
                Err(Error::Malformed)
            );
            bad = body.clone();
            bad[5] = 2;
            assert_eq!(
                append(mode, &mut bytes, &mut count, 1, &bad),
                Err(Error::Unsupported)
            );
            bad = body;
            bad[1..5].fill(0);
            assert_eq!(
                append(mode, &mut bytes, &mut count, 1, &bad),
                Err(Error::Limit)
            );
            assert!(bytes.is_empty());
            assert_eq!(count, 0);
        }
    }
}
