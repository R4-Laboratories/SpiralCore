#![deny(unsafe_op_in_unsafe_fn)]
//! Safe file primitives for the private, drive-absolute NTFS storage profile.
//! All Win32 FFI stays in this crate; protocol, codec and endpoint callers
//! retain their independent prohibition on unsafe code.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Storage,
    Unsupported,
    Limit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "private Windows storage: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Storage
    }
}
#[cfg(all(test, windows))]
mod tests;
