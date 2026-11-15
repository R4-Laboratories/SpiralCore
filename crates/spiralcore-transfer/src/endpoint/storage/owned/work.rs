//! Retained, non-growing caller buffers; names convey no filesystem authority.
use super::io;
use crate::{Error, Result};
use rustix::fs::FileType;
use std::{ffi::OsStr, os::fd::BorrowedFd};

pub(super) const BUFFER_BYTES: usize = 16 * 1024;
#[cfg(target_os = "linux")]
type Word = std::mem::MaybeUninit<u8>;
#[cfg(target_os = "macos")]
type Word = u64;
const WORDS: usize = BUFFER_BYTES / std::mem::size_of::<Word>();

pub(super) struct Buffer(Vec<Word>);
pub(super) struct Buffers {
    pub(super) root: Buffer,
    pub(super) child: Buffer,
}

fn check_capacity(actual: usize) -> Result<()> {
    // try_reserve_exact permits excess capacity. Refuse rather than retain an
    // unfunded payload or invoke an infallible shrinking allocation.
    if actual != WORDS {
        return Err(Error::Limit);
    }
    Ok(())
}
impl Buffer {
    fn new() -> Result<Self> {
        let mut words = Vec::new();
        words.try_reserve_exact(WORDS).map_err(|_| Error::Limit)?;
        check_capacity(words.capacity())?;
        #[cfg(target_os = "linux")]
        words.resize_with(WORDS, std::mem::MaybeUninit::uninit);
        #[cfg(target_os = "macos")]
        words.resize(WORDS, 0);
        Ok(Self(words))
    }

    pub(super) fn scan(
        &mut self,
        fd: BorrowedFd<'_>,
        mut visit: impl FnMut(&OsStr) -> Result<()>,
    ) -> Result<()> {
        use std::os::unix::ffi::OsStrExt;
        let directory = io::iterator_fd(fd)?;
        #[cfg(target_os = "linux")]
        {
            let mut entries = rustix::fs::RawDir::new(directory, &mut self.0);
            while let Some(entry) = entries.next() {
                let entry = entry.map_err(|_| Error::Storage)?;
                let bytes = entry.file_name().to_bytes();
                if bytes != b"." && bytes != b".." {
                    visit(OsStr::from_bytes(bytes))?;
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            use std::ops::ControlFlow;
            let mut outcome = Ok(());
            let flow = spiralcore_directory_macos::for_each(directory, &mut self.0, |name| {
                if name.as_bytes() != b"." && name.as_bytes() != b".." {
                    if let Err(error) = visit(name) {
                        outcome = Err(error);
                        return ControlFlow::Break(());
                    }
                }
                ControlFlow::Continue(())
            });
            // Preserve the original visitor error, including a quota Limit.
            outcome?;
            if flow.map_err(|_| Error::Storage)?.is_break() {
                return Err(Error::Storage);
            }
        }
        io::private(&io::held(fd)?, FileType::Directory)
    }
}
impl Buffers {
    #[cfg(test)]
    pub(super) fn identity(&self) -> (usize, usize, usize) {
        (
            self.root.0.as_ptr() as usize,
            self.child.0.as_ptr() as usize,
            (self.root.0.capacity() + self.child.0.capacity()) * std::mem::size_of::<Word>(),
        )
    }

    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            root: Buffer::new()?,
            child: Buffer::new()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_retained_capacity_must_match_the_funded_payload() {
        assert_eq!(check_capacity(WORDS), Ok(()));
        assert_eq!(check_capacity(WORDS + 1), Err(Error::Limit));
        let buffers = Buffers::new().unwrap();
        assert_eq!(
            buffers.root.0.capacity() * std::mem::size_of::<Word>(),
            BUFFER_BYTES
        );
        assert_eq!(
            buffers.child.0.capacity() * std::mem::size_of::<Word>(),
            BUFFER_BYTES
        );
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;
    use std::{
        collections::BTreeSet,
        fs,
        os::{
            fd::AsFd,
            unix::{ffi::OsStrExt, fs::DirBuilderExt},
        },
    };
    #[test]
    fn fixed_buffer_refills_and_fresh_offsets_preserve_long_names() {
        let path = std::env::temp_dir().join(format!(
            "sc-work-refill-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let expected: BTreeSet<_> = (0..101)
            .map(|i| {
                let name = format!("{i:03}-{}", "x".repeat(251));
                fs::File::create(path.join(&name)).unwrap();
                name.into_bytes()
            })
            .collect();
        assert!(expected.iter().map(|name| name.len()).sum::<usize>() > BUFFER_BYTES);
        let directory = fs::File::open(&path).unwrap();
        let mut buffer = Buffer::new().unwrap();
        let capacity = buffer.0.capacity();
        let address = buffer.0.as_ptr();
        for _ in 0..2 {
            let mut actual = BTreeSet::new();
            buffer
                .scan(directory.as_fd(), |name| {
                    assert!(actual.insert(name.as_bytes().to_vec()));
                    Ok(())
                })
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(buffer.0.capacity(), capacity);
            assert_eq!(buffer.0.as_ptr(), address);
        }
        assert_eq!(
            buffer.scan(directory.as_fd(), |_| Err(Error::Limit)),
            Err(Error::Limit)
        );
        assert_eq!(buffer.0.as_ptr(), address);
        drop(directory);
        fs::remove_dir_all(path).unwrap();
    }
}
