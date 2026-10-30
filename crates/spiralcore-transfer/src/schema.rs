//! Bounded, data-only flat JSON templates with exact raw-token and literal output.
//! Authorization, artifact admission and final object verification belong to the
//! endpoint. A matching layout is neither permission nor a prediction of values.
use crate::{object_id, protocol::Reader, Error, Id, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MAX_SCHEMA: usize = 64 * 1024;
pub const MAX_TARGET: usize = 16 * 1024 * 1024;
pub const MAX_TRAINING_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TRAINING_RECORDS: usize = 4096;
pub const MAX_TEMPLATES: usize = 64;
pub const MAX_SLOTS: usize = 32;
pub const MAX_TEMPLATE_RECORD: usize = 64 * 1024;
pub const MAX_RECORDS: usize = 65536;
pub const MAX_ENCODED: usize = MAX_TARGET + 5 * MAX_RECORDS + 48;
const MAX_MATCH_WORK: usize = 64 * 1024 * 1024;
const ARTIFACT_MAGIC: &[u8; 8] = b"SCSCH001";
const STREAM_MAGIC: &[u8; 8] = b"SCVAL001";

struct Template<'a> {
    fragments: Vec<&'a [u8]>,
    literal_len: usize,
    key: Id,
}

impl<'a> Template<'a> {
    fn new(fragments: Vec<&'a [u8]>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"spiralcore.schema.layout.v1\0");
        hasher.update((fragments.len() as u16).to_le_bytes());
        for fragment in &fragments {
            hasher.update((fragment.len() as u32).to_le_bytes());
            hasher.update(fragment);
        }
        Self {
            literal_len: fragments.iter().map(|f| f.len()).sum(),
            fragments,
            key: hasher.finalize().into(),
        }
    }

    fn slots(&self) -> usize {
        self.fragments.len() - 1
    }

    fn encoded_len(&self) -> usize {
        2 + 4 * self.fragments.len() + self.literal_len
    }
}

struct Record<'a> {
    template: Template<'a>,
    values: Vec<&'a [u8]>,
}

struct Templates<'a> {
    items: Vec<Template<'a>>,
    index: BTreeMap<Id, Vec<usize>>,
}

impl<'a> Templates<'a> {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            index: BTreeMap::new(),
        }
    }

    fn push(&mut self, template: Template<'a>) {
        self.index
            .entry(template.key)
            .or_default()
            .push(self.items.len());
        self.items.push(template);
    }

    fn find(&self, template: &Template<'_>, work: &mut usize) -> Option<usize> {
        for &index in self.index.get(&template.key)? {
            let candidate = &self.items[index];
            let charge = template.literal_len + candidate.literal_len;
            if charge > MAX_MATCH_WORK.saturating_sub(*work) {
                *work = MAX_MATCH_WORK;
                return None;
            }
            *work += charge;
            // The fingerprint indexes candidates; byte equality authorizes use.
            if candidate.fragments == template.fragments {
                return Some(index);
            }
        }
        None
    }
}

/// Fit only the supplied, ordered training samples. New target data never edits
/// this artifact. Unsupported records are ignored, including nested values.
pub fn fit(training: &[&[u8]]) -> Result<Vec<u8>> {
    if training.len() > MAX_TRAINING_RECORDS {
        return Err(Error::Limit);
    }
    let mut bytes = 0usize;
    let mut records = 0usize;
    for sample in training {
        bytes = bytes.checked_add(sample.len()).ok_or(Error::Limit)?;
        if bytes > MAX_TRAINING_BYTES {
            return Err(Error::Limit);
        }
        for _ in sample.split_inclusive(|&b| b == b'\n') {
            records += 1;
            if records > MAX_TRAINING_RECORDS {
                return Err(Error::Limit);
            }
        }
    }
    let mut templates = Templates::new();
    let mut encoded_len = 12;
    let mut work = 0;
    'samples: for sample in training {
        for bytes in sample.split_inclusive(|&b| b == b'\n') {
            if templates.items.len() == MAX_TEMPLATES || work == MAX_MATCH_WORK {
                break 'samples;
            }
            let Some(record) = parse_record(bytes) else {
                continue;
            };
            if templates.find(&record.template, &mut work).is_some() {
                continue;
            }
            if work == MAX_MATCH_WORK {
                break 'samples;
            }
            let added = record.template.encoded_len();
            if added <= MAX_SCHEMA - encoded_len {
                encoded_len += added;
                templates.push(record.template);
            }
        }
    }
    let mut artifact = Vec::new();
    artifact
        .try_reserve_exact(encoded_len)
        .map_err(|_| Error::Limit)?;
    artifact.extend_from_slice(ARTIFACT_MAGIC);
    artifact.extend_from_slice(&1u16.to_le_bytes());
    artifact.extend_from_slice(&(templates.items.len() as u16).to_le_bytes());
    for template in &templates.items {
        artifact.extend_from_slice(&(template.slots() as u16).to_le_bytes());
        for fragment in &template.fragments {
            artifact.extend_from_slice(&(fragment.len() as u32).to_le_bytes());
            artifact.extend_from_slice(fragment);
        }
    }
    Ok(artifact)
}

fn read_artifact(artifact: &[u8]) -> Result<Templates<'_>> {
    if artifact.len() > MAX_SCHEMA {
        return Err(Error::Limit);
    }
    let mut r = Reader::new(artifact);
    if r.take(8)? != ARTIFACT_MAGIC || r.u16()? != 1 {
        return Err(Error::Unsupported);
    }
    let count = r.u16()? as usize;
    if count > MAX_TEMPLATES {
        return Err(Error::Limit);
    }
    let mut templates = Templates::new();
    let mut work = 0;
    for _ in 0..count {
        let slots = r.u16()? as usize;
        if slots > MAX_SLOTS {
            return Err(Error::Limit);
        }
        let mut fragments = Vec::with_capacity(slots + 1);
        for _ in 0..=slots {
            let n = r.u32()? as usize;
            fragments.push(r.take(n)?);
        }
        let template = Template::new(fragments);
        let sample_len = template
            .literal_len
            .checked_add(slots * 4)
            .ok_or(Error::Limit)?;
        if sample_len > MAX_TEMPLATE_RECORD {
            return Err(Error::Limit);
        }
        let mut sample = Vec::with_capacity(sample_len);
        for (i, fragment) in template.fragments.iter().enumerate() {
            sample.extend_from_slice(fragment);
            if i < slots {
                sample.extend_from_slice(b"null");
            }
        }
        let parsed = parse_record(&sample).ok_or(Error::Malformed)?;
        if parsed.template.fragments != template.fragments || parsed.values.len() != slots {
            return Err(Error::Malformed);
        }
        if templates.find(&template, &mut work).is_some() {
            return Err(Error::Malformed);
        }
        if work == MAX_MATCH_WORK {
            return Err(Error::Limit);
        }
        templates.push(template);
    }
    r.end()?;
    Ok(templates)
}

/// Encode exact bytes using frozen layouts or explicit literal records. This
/// may exceed raw bytes; sender selection must include actual protocol costs.
pub fn encode(artifact: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    let templates = read_artifact(artifact)?;
    if target.len() > MAX_TARGET {
        return Err(Error::Limit);
    }
    let count = target
        .split_inclusive(|&b| b == b'\n')
        .take(MAX_RECORDS + 1)
        .count();
    let whole_literal = count > MAX_RECORDS;
    let records = if whole_literal { 1 } else { count };
    let capacity = 48 + 5 * records + target.len();
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(capacity)
        .map_err(|_| Error::Limit)?;
    encoded.extend_from_slice(STREAM_MAGIC);
    encoded.extend_from_slice(&object_id(artifact));
    encoded.extend_from_slice(&(target.len() as u32).to_le_bytes());
    encoded.extend_from_slice(&(records as u32).to_le_bytes());
    if whole_literal {
        write_literal(&mut encoded, target);
        return Ok(encoded);
    }
    let mut work = 0;
    for bytes in target.split_inclusive(|&b| b == b'\n') {
        let record = if work < MAX_MATCH_WORK {
            parse_record(bytes)
        } else {
            None
        };
        let matched = record
            .as_ref()
            .and_then(|r| templates.find(&r.template, &mut work));
        if let (Some(record), Some(index)) = (record, matched) {
            let size = 3 + record.values.iter().map(|v| 4 + v.len()).sum::<usize>();
            if size < 5 + bytes.len() {
                encoded.push(1);
                encoded.extend_from_slice(&(index as u16).to_le_bytes());
                for value in record.values {
                    encoded.extend_from_slice(&(value.len() as u32).to_le_bytes());
                    encoded.extend_from_slice(value);
                }
                continue;
            }
        }
        write_literal(&mut encoded, bytes);
    }
    Ok(encoded)
}

fn write_literal(encoded: &mut Vec<u8>, bytes: &[u8]) {
    encoded.push(0);
    encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    encoded.extend_from_slice(bytes);
}

/// Preflight all lengths/tokens before allocating output. The endpoint verifies
/// the final whole-object digest and durable publication independently.
pub fn decode(artifact: &[u8], encoded: &[u8], target_len: usize) -> Result<Vec<u8>> {
    let templates = read_artifact(artifact)?;
    if target_len > MAX_TARGET || encoded.len() > MAX_ENCODED {
        return Err(Error::Limit);
    }
    let mut r = Reader::new(encoded);
    if r.take(8)? != STREAM_MAGIC {
        return Err(Error::Unsupported);
    }
    if r.id()? != object_id(artifact) || r.u32()? as usize != target_len {
        return Err(Error::Integrity);
    }
    let count = r.u32()? as usize;
    if count > MAX_RECORDS {
        return Err(Error::Limit);
    }
    let records = r.remaining();
    visit_records(&templates, records, count, target_len, |_| {})?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(target_len)
        .map_err(|_| Error::Limit)?;
    visit_records(&templates, records, count, target_len, |bytes| {
        output.extend_from_slice(bytes)
    })?;
    Ok(output)
}

fn visit_records(
    templates: &Templates<'_>,
    bytes: &[u8],
    count: usize,
    target_len: usize,
    mut emit: impl FnMut(&[u8]),
) -> Result<()> {
    let mut r = Reader::new(bytes);
    let mut total = 0usize;
    for _ in 0..count {
        match r.u8()? {
            0 => {
                let n = r.u32()? as usize;
                if n == 0 || n > target_len.saturating_sub(total) {
                    return Err(Error::Limit);
                }
                let literal = r.take(n)?;
                total += n;
                emit(literal);
            }
            1 => {
                let template = templates
                    .items
                    .get(r.u16()? as usize)
                    .ok_or(Error::Malformed)?;
                let mut record_len = template.literal_len;
                if record_len > target_len.saturating_sub(total) {
                    return Err(Error::Limit);
                }
                for (i, fragment) in template.fragments.iter().enumerate() {
                    emit(fragment);
                    if i < template.slots() {
                        let n = r.u32()? as usize;
                        record_len = record_len.checked_add(n).ok_or(Error::Limit)?;
                        if record_len > MAX_TEMPLATE_RECORD
                            || record_len > target_len.saturating_sub(total)
                        {
                            return Err(Error::Limit);
                        }
                        let value = r.take(n)?;
                        if !valid_token(value) {
                            return Err(Error::Malformed);
                        }
                        emit(value);
                    }
                }
                total += record_len;
            }
            _ => return Err(Error::Unsupported),
        }
    }
    r.end()?;
    if total != target_len {
        return Err(Error::Integrity);
    }
    Ok(())
}

fn parse_record(bytes: &[u8]) -> Option<Record<'_>> {
    if bytes.len() > MAX_TEMPLATE_RECORD || std::str::from_utf8(bytes).is_err() {
        return None;
    }
    let mut at = whitespace(bytes, 0);
    if bytes.get(at) != Some(&b'{') {
        return None;
    }
    at = whitespace(bytes, at + 1);
    if bytes.get(at) == Some(&b'}') {
        return (whitespace(bytes, at + 1) == bytes.len()).then(|| Record {
            template: Template::new(vec![bytes]),
            values: Vec::new(),
        });
    }
    let mut fragments = Vec::new();
    let mut values = Vec::new();
    let mut last_end = 0;
    loop {
        if values.len() == MAX_SLOTS {
            return None;
        }
        at = whitespace(bytes, string_end(bytes, at)?);
        if bytes.get(at) != Some(&b':') {
            return None;
        }
        at = whitespace(bytes, at + 1);
        fragments.push(&bytes[last_end..at]);
        let end = primitive_end(bytes, at)?;
        values.push(&bytes[at..end]);
        last_end = end;
        at = whitespace(bytes, end);
        match bytes.get(at)? {
            b'}' => {
                if whitespace(bytes, at + 1) != bytes.len() {
                    return None;
                }
                fragments.push(&bytes[last_end..]);
                return Some(Record {
                    template: Template::new(fragments),
                    values,
                });
            }
            b',' => at = whitespace(bytes, at + 1),
            _ => return None,
        }
    }
}

fn whitespace(bytes: &[u8], mut at: usize) -> usize {
    while bytes
        .get(at)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        at += 1;
    }
    at
}

fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut at = start + 1;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'"' => return Some(at + 1),
            b'\\' => match bytes.get(at + 1)? {
                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => at += 2,
                b'u' if bytes.get(at + 2..at + 6)?.iter().all(u8::is_ascii_hexdigit) => at += 6,
                _ => return None,
            },
            0..=31 => return None,
            _ => at += 1,
        }
    }
    None
}

fn primitive_end(bytes: &[u8], start: usize) -> Option<usize> {
    let tail = bytes.get(start..)?;
    match *tail.first()? {
        b'"' => return string_end(bytes, start),
        b't' if tail.starts_with(b"true") => return Some(start + 4),
        b'f' if tail.starts_with(b"false") => return Some(start + 5),
        b'n' if tail.starts_with(b"null") => return Some(start + 4),
        b'-' | b'0'..=b'9' => {}
        _ => return None,
    }
    let mut at = start + usize::from(bytes[start] == b'-');
    match *bytes.get(at)? {
        b'0' => at += 1,
        b'1'..=b'9' => {
            at += 1;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let first = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if first == at {
            return None;
        }
    }
    if bytes.get(at).is_some_and(|b| matches!(b, b'e' | b'E')) {
        at += 1;
        if bytes.get(at).is_some_and(|b| matches!(b, b'+' | b'-')) {
            at += 1;
        }
        let first = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if first == at {
            return None;
        }
    }
    Some(at)
}

fn valid_token(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok() && primitive_end(bytes, 0) == Some(bytes.len())
}
