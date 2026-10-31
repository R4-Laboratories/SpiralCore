//! Bounded SpiralCore frames inside the pinned canonical MVM mailbox envelope.
//!
//! This supplies a plaintext endpoint seam, not a guest service or a durable inbox.
//! Policy is trusted control-plane input. Payloads cannot mint endpoint authority.
#![forbid(unsafe_code)]

#[cfg(feature = "conformance")]
pub mod conformance;
mod direction;
mod policy;

#[cfg(feature = "live")]
pub use direction::live;
pub use direction::{Counters, Direction, DirectionConfig, VerifiedFrame};
pub use policy::{MailboxAuthority, Policy};
pub const MAX_FRAME_BYTES: usize = spiralcore_transfer::MAX_FRAME;
pub const MAX_ENVELOPE_BYTES: usize = 262_144;
pub const MAX_REPLAY_IDENTITIES: usize = 4096;
pub const CONTENT_TYPE: &str = "application/spiralcore-object";
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Policy,
    Limit,
    ReplayCapacity,
    Mailbox(mvm_core::mailbox::MailboxError),
    Transport(mvm_core::mailbox::transport::TransportError),
    Route(mvm_core::mailbox::transport::TransportBindingError),
    Outbound(mvm_core::mailbox::transport::OutboundAuthorizationError),
    Frame(spiralcore_transfer::Error),
    State,
    Io(std::io::Error),
    Serialization,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<spiralcore_transfer::Error> for Error {
    fn from(value: spiralcore_transfer::Error) -> Self {
        Self::Frame(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

fn content_type() -> Result<mvm_contract::protocol::mailbox::ContentType> {
    mvm_contract::protocol::mailbox::ContentType::parse(CONTENT_TYPE, 1).map_err(|_| Error::Policy)
}
