//! Versioned, bounded COPY/LITERAL recipes. See docs/network/sparse-v1.md.
use crate::{protocol::Reader, Error, Result, MAX_CHUNK, MAX_FRAME, MAX_OBJECT};

#[cfg(test)]
#[path = "../tests/fixtures/sender_delta_checks.rs"]
mod sender_delta_checks;

#[cfg(test)]
mod scan_tests {
    use super::*;

    #[test]
    fn bulk_scans_match_scalar_at_every_short_boundary() {
        for n in 0..=257 {
            let a: Vec<u8> = (0..n).map(|i| i as u8).collect();
            for at in 0..=n {
                let mut b = a.clone();
                if at < n {
                    b[at] ^= 1;
                }
                assert_eq!(common_prefix(&a, &b), at);
                assert_eq!(common_suffix(&a, &b), if at < n { n - at - 1 } else { n });
                let mut unequal: Vec<u8> = a.iter().map(|x| x ^ 1).collect();
                if at < n {
                    unequal[at] = a[at];
                }
                assert_eq!(unequal_prefix(&a, &unequal), at);
            }
        }
    }

    #[test]
    fn bulk_scans_handle_different_lengths_and_zero_byte_borrows() {
        for n in [1, 7, 8, 9, 63, 64, 65, 255, 256, 257, 4096] {
            for byte in 0..=255u8 {
                let a = vec![byte; n];
                let b = vec![byte.wrapping_add(1); n + 3];
                assert_eq!(unequal_prefix(&a, &b), n);
                assert_eq!(common_prefix(&a, &b), 0);
                assert_eq!(common_suffix(&a, &b), 0);
                assert_eq!(common_prefix(&a, &a[..n / 2]), n / 2);
                assert_eq!(common_suffix(&a, &a[..n / 2]), n / 2);
            }
        }
    }
}

pub const MAX_OPS: usize = 8192;
const FRAME_OPS: usize = 256;
#[derive(Debug)]
pub struct Recipe {
    pub frames: Vec<Vec<u8>>,
    pub operations: usize,
}
#[derive(Clone, Copy)]
enum Op {
    Copy(usize, usize),
    Literal(usize, usize),
}

/// Compare two bounded linear recipes; None selects a full raw transfer.
/// The estimate includes Data requests/replies and the extra base ID in Begin.
pub fn encode(base: &[u8], target: &[u8]) -> Result<Option<Recipe>> {
    if base.len() as u64 > MAX_OBJECT || target.len() as u64 > MAX_OBJECT {
        return Err(Error::Limit);
    }
    if target.is_empty() {
        return Ok(None);
    }
    let prefix = common_prefix(base, target);
    let suffix = common_suffix(&base[prefix..], &target[prefix..]);
    let mut simple = Vec::new();
    if prefix > 0 {
        simple.push(Op::Copy(0, prefix));
    }
    let middle = target.len() - prefix - suffix;
    if middle > 0 {
        simple.push(Op::Literal(prefix, middle));
    }
    if suffix > 0 {
        simple.push(Op::Copy(base.len() - suffix, suffix));
    }
    let mut aligned = Vec::new();
    let mut i = 0;
    let mut literal = 0;
    let common = base.len().min(target.len());
    while i < common && aligned.len() <= MAX_OPS {
        i += unequal_prefix(&base[i..common], &target[i..common]);
        if i == common {
            break;
        }
        let start = i;
        // Reuse the prefix already established above rather than scanning a
        // full exact-repeat object twice. Later runs use the same bulk scan.
        i += if i == 0 {
            prefix
        } else {
            common_prefix(&base[i..common], &target[i..common])
        };
        if i - start >= 32 {
            if literal < start {
                aligned.push(Op::Literal(literal, start - literal));
            }
            aligned.push(Op::Copy(start, i - start));
            literal = i;
        }
    }
    if literal < target.len() {
        aligned.push(Op::Literal(literal, target.len() - literal));
    }
    let mut best = pack_candidate(&simple, target)?;
    if aligned.len() <= MAX_OPS {
        if let Some(candidate) = pack_candidate(&aligned, target)? {
            if best.as_ref().is_none_or(|b| cost(&candidate) < cost(b)) {
                best = Some(candidate);
            }
        }
    }
    let raw_cost = target.len() + target.len().div_ceil(MAX_CHUNK) * 133;
    Ok(best.filter(|r| cost(r) + 32 < raw_cost))
}

// Slice equality permits the compiler/library's word/vector comparison while
// the scalar remainder locates the exact first mismatch, preserving recipes.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    while n - i >= 64 && a[i..i + 64] == b[i..i + 64] {
        i += 64;
    }
    while i < n && a[i] == b[i] {
        i += 1;
    }
    i
}

fn common_suffix(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    while n - i >= 64 && a[a.len() - i - 64..a.len() - i] == b[b.len() - i - 64..b.len() - i] {
        i += 64;
    }
    while i < n && a[a.len() - i - 1] == b[b.len() - i - 1] {
        i += 1;
    }
    i
}

fn unequal_prefix(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    while n - i >= 8 {
        let x = u64::from_le_bytes(a[i..i + 8].try_into().unwrap())
            ^ u64::from_le_bytes(b[i..i + 8].try_into().unwrap());
        // Zero-byte detection answers only "any equal byte?"; borrow can mark
        // extra positions, so locate the first actual equality scalarly below.
        if x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 != 0 {
            break;
        }
        i += 8;
    }
    while i < n && a[i] != b[i] {
        i += 1;
    }
    i
}
fn cost(r: &Recipe) -> usize {
    r.frames.iter().map(|b| b.len() + 128).sum()
}
fn pack_candidate(ops: &[Op], target: &[u8]) -> Result<Option<Recipe>> {
    // A whole-target literal costs N + 142 * ceil(N / MAX_CHUNK), plus
    // the 32-byte base ID. Raw costs N + 133 * ceil(N / MAX_CHUNK).
    // It cannot survive the final Raw filter, so do not allocate its frames.
    if matches!(ops, [Op::Literal(0, n)] if *n > 0 && *n == target.len()) {
        return Ok(None);
    }
    pack(ops, target)
}
fn header() -> Vec<u8> {
    vec![3, 0, 0, 0, 0, 1, 0, 0, 0]
}
fn finish(frames: &mut Vec<Vec<u8>>, body: &mut Vec<u8>, count: &mut usize, decoded: &mut usize) {
    if *count == 0 {
        return;
    }
    body[1..5].copy_from_slice(&(*decoded as u32).to_le_bytes());
    body[7..9].copy_from_slice(&(*count as u16).to_le_bytes());
    frames.push(std::mem::replace(body, header()));
    *count = 0;
    *decoded = 0;
}
fn pack(ops: &[Op], target: &[u8]) -> Result<Option<Recipe>> {
    let mut frames = Vec::new();
    let mut body = header();
    let mut count = 0;
    let mut decoded = 0;
    let mut literals = 0;
    let mut total = 0;
    for op in ops {
        let (mut start, mut left, copy) = match *op {
            Op::Copy(s, n) => (s, n, true),
            Op::Literal(s, n) => (s, n, false),
        };
        while left > 0 {
            let overhead = if copy { 13 } else { 5 };
            if count == FRAME_OPS
                || body.len() + overhead + usize::from(!copy) > MAX_FRAME - 56
                || (!copy && literals == MAX_CHUNK)
            {
                finish(&mut frames, &mut body, &mut count, &mut decoded);
                literals = 0;
            }
            let n = if copy {
                left
            } else {
                left.min(MAX_CHUNK - literals)
                    .min(MAX_FRAME - 56 - body.len() - 5)
            };
            total += 1;
            if total > MAX_OPS {
                return Ok(None);
            }
            body.push(if copy { 0 } else { 1 });
            if copy {
                body.extend_from_slice(&(start as u64).to_le_bytes());
            }
            body.extend_from_slice(&(n as u32).to_le_bytes());
            if !copy {
                body.extend_from_slice(&target[start..start + n]);
                literals += n;
            }
            decoded += n;
            count += 1;
            start += n;
            left -= n;
        }
    }
    finish(&mut frames, &mut body, &mut count, &mut decoded);
    Ok(Some(Recipe {
        frames,
        operations: total,
    }))
}

/// Validate a complete frame before changing bytes or accepted work. COPY reads base only.
pub fn apply(
    base: &[u8],
    output: &mut Vec<u8>,
    target_len: usize,
    operations: &mut usize,
    body: &[u8],
) -> Result<()> {
    if base.len() as u64 > MAX_OBJECT
        || target_len as u64 > MAX_OBJECT
        || body.len() > MAX_FRAME - 56
    {
        return Err(Error::Limit);
    }
    let mut r = Reader::new(body);
    if r.u8()? != 3 {
        return Err(Error::Malformed);
    }
    let declared = r.u32()? as usize;
    if r.u16()? != 1 {
        return Err(Error::Unsupported);
    }
    let count = r.u16()? as usize;
    if count == 0 {
        return Err(Error::Malformed);
    }
    if count > FRAME_OPS || operations.checked_add(count).ok_or(Error::Limit)? > MAX_OPS {
        return Err(Error::Limit);
    }
    let mut expanded = 0usize;
    let mut literals = 0usize;
    for _ in 0..count {
        let (len, literal) = match r.u8()? {
            0 => {
                let start = r.u64()?;
                let len = r.u32()? as usize;
                if start.checked_add(len as u64).ok_or(Error::Limit)? > base.len() as u64 {
                    return Err(Error::Limit);
                }
                (len, false)
            }
            1 => {
                let len = r.u32()? as usize;
                r.take(len)?;
                (len, true)
            }
            _ => return Err(Error::Malformed),
        };
        if len == 0 {
            return Err(Error::Malformed);
        }
        expanded = expanded.checked_add(len).ok_or(Error::Limit)?;
        if literal {
            literals = literals.checked_add(len).ok_or(Error::Limit)?;
        }
    }
    r.end()?;
    if expanded != declared {
        return Err(Error::Malformed);
    }
    if literals > MAX_CHUNK || output.len().checked_add(expanded).ok_or(Error::Limit)? > target_len
    {
        return Err(Error::Limit);
    }
    output
        .try_reserve_exact(expanded)
        .map_err(|_| Error::Limit)?;
    let mut r = Reader::new(&body[9..]);
    for _ in 0..count {
        match r.u8()? {
            0 => {
                let start = r.u64()? as usize;
                let len = r.u32()? as usize;
                output.extend_from_slice(&base[start..start + len]);
            }
            1 => {
                let len = r.u32()? as usize;
                output.extend_from_slice(r.take(len)?);
            }
            _ => unreachable!("first pass validated operation tags"),
        }
    }
    *operations += count;
    Ok(())
}
