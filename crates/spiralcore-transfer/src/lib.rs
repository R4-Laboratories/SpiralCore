#![forbid(unsafe_code)]

pub mod client;
pub mod dag;
pub mod endpoint;
pub mod protocol;
pub mod wan;
use sha2::{Digest, Sha256};
pub type Id = [u8; 32];
pub type Result<T> = std::result::Result<T, Error>;
pub const MAX_OBJECT: u64 = 64 * 1024 * 1024;
pub const MAX_FRAME: usize = 65536;
pub const MAX_CHUNK: usize = 60 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Error {
    Malformed = 1,
    Unauthorized = 2,
    Limit = 3,
    Integrity = 4,
    Missing = 5,
    Sequence = 6,
    Unknown = 7,
    Expired = 8,
    Storage = 9,
    Unsupported = 10,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Storage
    }
}
pub fn hash(domain: &[u8], parts: &[&[u8]]) -> Id {
    let mut h = Sha256::new();
    h.update(domain);
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}
pub fn object_id(bytes: &[u8]) -> Id {
    hash(
        b"spiralcore.object.bytes.v1\0",
        &[&(bytes.len() as u64).to_le_bytes(), bytes],
    )
}
pub fn hex(id: &Id) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
