//! First observed local failure on an object stream. The fields are bounded
//! protocol categories. No errors, object identities or payloads are retained.
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureSite {
    SenderTxLock,
    ObjectWrite,
    SenderReplyWait,
    SenderOperation,
    ReceiverHandle,
    ReceiverReadback,
    ReceiverDelivery,
    ReceiverReplyLock,
    ObjectRead,
    WorkerExit,
    CallerRead,
    CallerWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    Timeout,
    Io(std::io::ErrorKind),
    Core(spiralcore_transfer::Error),
    Protocol,
    Sequence,
    Integrity,
    Truncated,
    Limit,
    Binding,
    Worker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailureObservation {
    pub site: FailureSite,
    pub reason: FailureReason,
    /// The original object-v1 frame kind when available, never its contents.
    pub frame_kind: Option<u8>,
}

impl FailureReason {
    pub(crate) fn observed(error: &Error) -> Option<Self> {
        Some(match error {
            Error::Closed | Error::Cancelled => return None,
            Error::Timeout => Self::Timeout,
            Error::Io(e) => Self::Io(e.kind()),
            Error::Core(e) => Self::Core(*e),
            Error::InvalidBinding | Error::PeerMismatch => Self::Binding,
            Error::Protocol => Self::Protocol,
            Error::Sequence => Self::Sequence,
            Error::Integrity => Self::Integrity,
            Error::Truncated => Self::Truncated,
            Error::Limit => Self::Limit,
            Error::Worker => Self::Worker,
        })
    }
}
