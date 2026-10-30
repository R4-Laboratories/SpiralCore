//! Versioned, bounded COPY/LITERAL recipes. See docs/network/sparse-v1.md.
use crate::{protocol::Reader, Error, Result, MAX_CHUNK, MAX_FRAME, MAX_OBJECT};

pub const MAX_OPS: usize = 8192;
const FRAME_OPS: usize = 256;
const FRAME_HEADER_BYTES: usize = 9;
const COPY_BYTES: usize = 13;
const LITERAL_HEADER_BYTES: usize = 5;
// Data request, Accepted reply and their two length words, excluding the body.
const FRAME_EXCHANGE_BYTES: usize = 128;
const BASE_ID_BYTES: usize = 32;
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
    let prefix = base.iter().zip(target).take_while(|(a, b)| a == b).count();
    let suffix = base[prefix..]
        .iter()
        .rev()
        .zip(target[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
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
        if base[i] != target[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < common && base[i] == target[i] {
            i += 1;
        }
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
    let raw_cost = target.len() + target.len().div_ceil(MAX_CHUNK) * 133;
    let eligible_cost = raw_cost.saturating_sub(BASE_ID_BYTES);
    let mut best = if lower_bound(&simple)? < eligible_cost {
        pack(&simple, target)?
    } else {
        None
    };
    if aligned.len() <= MAX_OPS {
        let ceiling = best
            .as_ref()
            .map_or(eligible_cost, |b| cost(b).min(eligible_cost));
        if lower_bound(&aligned)? < ceiling {
            if let Some(candidate) = pack(&aligned, target)? {
                if best.as_ref().is_none_or(|b| cost(&candidate) < cost(b)) {
                    best = Some(candidate);
                }
            }
        }
    }
    Ok(best.filter(|r| cost(r) + BASE_ID_BYTES < raw_cost))
}
fn cost(r: &Recipe) -> usize {
    r.frames
        .iter()
        .map(|b| b.len() + FRAME_EXCHANGE_BYTES)
        .sum()
}
// Generated operations have positive lengths. Splitting literals can only add
// prefixes and frames: B + 13*C + 5*L + 137*max(ceil(B/MAX_CHUNK), ceil(ops/256))
// is therefore a lower bound, never an estimate used to admit a candidate.
fn lower_bound(ops: &[Op]) -> Result<usize> {
    let mut literals = 0usize;
    let mut prefixes = 0usize;
    for op in ops {
        let overhead = match *op {
            Op::Copy(_, _) => COPY_BYTES,
            Op::Literal(_, n) => {
                literals = literals.checked_add(n).ok_or(Error::Limit)?;
                LITERAL_HEADER_BYTES
            }
        };
        prefixes = prefixes.checked_add(overhead).ok_or(Error::Limit)?;
    }
    let frames = literals
        .div_ceil(MAX_CHUNK)
        .max(ops.len().div_ceil(FRAME_OPS));
    let frame_bytes = frames
        .checked_mul(FRAME_HEADER_BYTES + FRAME_EXCHANGE_BYTES)
        .ok_or(Error::Limit)?;
    literals
        .checked_add(prefixes)
        .and_then(|n| n.checked_add(frame_bytes))
        .ok_or(Error::Limit)
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
            let overhead = if copy {
                COPY_BYTES
            } else {
                LITERAL_HEADER_BYTES
            };
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
                    .min(MAX_FRAME - 56 - body.len() - LITERAL_HEADER_BYTES)
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

#[cfg(test)]
#[path = "delta/pruning_tests.rs"]
mod pruning_tests;
