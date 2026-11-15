//! Parsing for the requested Darwin NAME/RETURNED_ATTRS/ERROR ABI only.
//! ABI source: apple-oss-distributions/xnu at f6217f891ac0bb64f3d375211650a4c1ff8ca1ea,
//! bsd/sys/attr.h and bsd/man/man2/getattrlistbulk.2. No implementation is copied.
use std::{ffi::OsStr, io, ops::ControlFlow, os::unix::ffi::OsStrExt};

pub(crate) const NAME: u32 = 0x0000_0001;
pub(crate) const ENTRY_ERROR: u32 = 0x2000_0000;
pub(crate) const RETURNED: u32 = 0x8000_0000;
pub(crate) const REQUESTED: u32 = NAME | ENTRY_ERROR | RETURNED;
const PREFIX: usize = 36;

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn word(bytes: &[u8], at: usize) -> io::Result<u32> {
    let end = at.checked_add(4).ok_or_else(invalid)?;
    let value = bytes.get(at..end).ok_or_else(invalid)?;
    Ok(u32::from_ne_bytes(value.try_into().map_err(|_| invalid())?))
}

pub(crate) fn visit_batch(
    bytes: &[u8],
    count: usize,
    visit: &mut impl FnMut(&OsStr) -> ControlFlow<()>,
) -> io::Result<ControlFlow<()>> {
    if count > bytes.len() / (PREFIX + 2) {
        return Err(invalid());
    }
    let mut at = 0usize;
    for index in 0..count {
        if !at.is_multiple_of(8) {
            return Err(invalid());
        }
        let length = usize::try_from(word(bytes, at)?).map_err(|_| invalid())?;
        if length < PREFIX + 2 {
            return Err(invalid());
        }
        let end = at.checked_add(length).ok_or_else(invalid)?;
        let record = bytes.get(at..end).ok_or_else(invalid)?;
        if index + 1 != count && !length.is_multiple_of(8) {
            return Err(invalid());
        }
        if word(record, 4)? != REQUESTED
            || [8, 12, 16, 20]
                .into_iter()
                .any(|offset| word(record, offset).ok() != Some(0))
            || word(record, 24)? != 0
        {
            return Err(invalid());
        }
        let relative = word(record, 28)? as i32;
        let relative = usize::try_from(relative).map_err(|_| invalid())?;
        let start = 28usize.checked_add(relative).ok_or_else(invalid)?;
        let length = usize::try_from(word(record, 32)?).map_err(|_| invalid())?;
        if start < PREFIX || !(2..=1024).contains(&length) {
            return Err(invalid());
        }
        let end = start.checked_add(length).ok_or_else(invalid)?;
        let name = record.get(start..end).ok_or_else(invalid)?;
        if name.last() != Some(&0) {
            return Err(invalid());
        }
        let name = &name[..name.len() - 1];
        if name.contains(&0) || name.contains(&b'/') {
            return Err(invalid());
        }
        if visit(OsStr::from_bytes(name)).is_break() {
            return Ok(ControlFlow::Break(()));
        }
        at = at.checked_add(record.len()).ok_or_else(invalid)?;
    }
    Ok(ControlFlow::Continue(()))
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
