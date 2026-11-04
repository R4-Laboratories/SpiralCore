#![forbid(unsafe_code)]
//! Initial bounded TCP transport baseline for explicitly cooperating FlowMux peers.
//! This grants no route/peer authority. Full object-v1 warm/Combined integration
//! and the canonical post-admission endpoint hook remain separate required work.
mod duplex;
mod stream;
pub use stream::MappedStream;
mod object;
pub use duplex::{BlockReceipt, Duplex, DuplexReader, DuplexWriter, JoinGuard, Role};
mod record;
pub mod scope;
mod tcp;
pub mod tls;
pub use object::{ObjectChannel, ServeEvent};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Application,
    Artifact,
}
pub(crate) enum ObjectRecord {
    Frame(Purpose, spiralcore_transfer::protocol::Frame),
    ApplicationHalfClose,
    TransportFin,
}
pub use tcp::{Cancellation, Counters, Event, Rx, TcpMapping, Tx};
pub const MAX_BLOCK: usize = 16 * 1024;
pub const HEADER_LEN: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub flow: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub scope_commitment: [u8; 32],
}
impl Binding {
    pub fn directional_flow(self, role: Role) -> [u8; 32] {
        spiralcore_transfer::hash(
            b"spiralcore.flow.direction.v1\0",
            &[&self.flow, &[role.tag()]],
        )
    }
    fn validate(self) -> Result<()> {
        if self.flow == [0; 32] || self.policy_sha256 == [0; 32] || self.scope_commitment == [0; 32]
        {
            return Err(Error::InvalidBinding);
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Core(spiralcore_transfer::Error),
    InvalidBinding,
    PeerMismatch,
    Limit,
    Protocol,
    Sequence,
    Integrity,
    Truncated,
    Timeout,
    Cancelled,
    Closed,
    Worker,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flowmux mapping: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<spiralcore_transfer::Error> for Error {
    fn from(error: spiralcore_transfer::Error) -> Self {
        Self::Core(error)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
