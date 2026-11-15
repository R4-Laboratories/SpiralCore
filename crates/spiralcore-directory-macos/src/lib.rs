//! Fixed-buffer directory discovery only; this crate grants no storage authority.
#![deny(unsafe_op_in_unsafe_fn)]

pub const BUFFER_BYTES: usize = 16 * 1024;
pub const BUFFER_WORDS: usize = BUFFER_BYTES / std::mem::size_of::<u64>();

#[cfg(target_os = "macos")]
mod native;
#[cfg(any(target_os = "macos", all(test, unix)))]
mod records;
#[cfg(target_os = "macos")]
pub use native::for_each;
