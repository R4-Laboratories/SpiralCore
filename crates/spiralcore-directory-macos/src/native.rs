use crate::{records, BUFFER_WORDS};
use std::{
    ffi::OsStr,
    io,
    ops::ControlFlow,
    os::fd::{AsRawFd, OwnedFd},
};

/// Consume an independently opened directory offset and discover names only.
/// An early callback stop is returned distinctly from complete enumeration.
/// Private ownership, identity, mount and authorization checks belong to the caller.
pub fn for_each(
    directory: OwnedFd,
    buffer: &mut [u64],
    mut visit: impl FnMut(&OsStr) -> ControlFlow<()>,
) -> io::Result<ControlFlow<()>> {
    if buffer.len() != BUFFER_WORDS {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut attributes = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: records::REQUESTED,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    loop {
        buffer.fill(0);
        // SAFETY: The live directory FD is owned for the call. Both pointers
        // reference initialized, exclusively borrowed storage of the stated sizes;
        // u64 storage provides the ABI's 8-byte alignment. No pointer escapes.
        let count = unsafe {
            libc::getattrlistbulk(
                directory.as_raw_fd(),
                (&mut attributes as *mut libc::attrlist).cast(),
                buffer.as_mut_ptr().cast(),
                std::mem::size_of_val(buffer),
                u64::from(libc::FSOPT_PACK_INVAL_ATTRS),
            )
        };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Ok(ControlFlow::Continue(()));
        }
        // SAFETY: The u64 slice is fully initialized, including unwritten bytes,
        // and remains immutably borrowed through parsing/callbacks. The byte view
        // has the same extent; checked parsing creates no references outside it.
        let bytes = unsafe {
            std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), std::mem::size_of_val(buffer))
        };
        if records::visit_batch(bytes, count as usize, &mut visit)?.is_break() {
            return Ok(ControlFlow::Break(()));
        }
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
