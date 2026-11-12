//! Guest-application transport of one existing SpiralCore frame per message
//! through the workload's own admitted MVM mailbox. The guest agent owns
//! receipt verification, reader enrollment and signing of delivery ACKs.
#![forbid(unsafe_code)]

use mvm_agentd::mailbox_outbox::{LOCAL_OUTBOX_PATH, write_outbox};
pub use mvm_contract::protocol::mailbox::MessageId;
use mvm_contract::protocol::mailbox::{PrincipalId, SignedMailboxDeliveryLease};
use mvm_core::mailbox::frame::{OWN_MAILBOX_RECIPIENT, OutboxMessage, Refusal};
use mvm_core::mailbox::reader_wire::{
    ApplicationReaderRequest, GUEST_READER_SOCKET_PATH, ReaderResponse, read_response,
    write_application_request,
};
use spiralcore_transfer::protocol::Frame;
use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

mod reader_io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SpiralCore frame: {0}")]
    Frame(#[from] spiralcore_transfer::Error),
    #[error("MVM mailbox: {0}")]
    Mailbox(#[from] Refusal),
    #[error("guest reader socket: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected MVM response to {0}")]
    Response(&'static str),
}

/// The caller can retain the message ID and the original encoded frame before
/// staging it. Retrying this object keeps both the MVM identity and body.
pub struct PendingFrame(OutboxMessage);

impl PendingFrame {
    pub fn new(frame: &Frame) -> Result<Self, Error> {
        Ok(Self(OutboxMessage::new(
            PrincipalId::from_bytes(OWN_MAILBOX_RECIPIENT),
            frame.encode()?,
        )))
    }

    pub fn message_id(&self) -> MessageId {
        self.0.message_id
    }

    /// Recover the original operation after an application restart. Persist
    /// the ID and these exact frame bytes together before the first stage.
    pub fn restore(message_id: MessageId, encoded: Vec<u8>) -> Result<Self, Error> {
        Frame::decode(&encoded)?;
        Ok(Self(OutboxMessage {
            recipient: PrincipalId::from_bytes(OWN_MAILBOX_RECIPIENT),
            message_id,
            body: encoded,
        }))
    }

    pub fn encoded(&self) -> &[u8] {
        &self.0.body
    }

    /// This confirms local staging only. The agent retains the pending send
    /// until MVM verifies the original signed V2 local-durable receipt.
    pub fn stage(&self) -> Result<(), Error> {
        self.stage_at(Path::new(LOCAL_OUTBOX_PATH))
    }

    fn stage_at(&self, path: &Path) -> Result<(), Error> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.is_file() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected MVM durable outbox file",
                )
                .into());
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        Ok(write_outbox(path, &self.0)?)
    }
}

pub struct Delivery {
    frame: Frame,
    lease: Box<SignedMailboxDeliveryLease>,
}

impl Delivery {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn message_id(&self) -> MessageId {
        self.lease.lease.message_id
    }

    /// Call only after the application has committed processing of this frame.
    /// Keep this delivery on error: an interrupted ACK reply is ambiguous.
    pub fn acknowledge(&self) -> Result<(), Error> {
        self.ack_with(request)
    }

    fn ack_with(
        &self,
        send: impl FnOnce(ApplicationReaderRequest) -> Result<ReaderResponse, Error>,
    ) -> Result<(), Error> {
        match send(ApplicationReaderRequest::Ack {
            lease: self.lease.clone(),
        })? {
            ReaderResponse::Acknowledged { ack } if ack.ack.lease == *self.lease => Ok(()),
            ReaderResponse::Refused { reason } => Err(reason.into()),
            _ => Err(Error::Response("ACK")),
        }
    }
}

fn request(value: ApplicationReaderRequest) -> Result<ReaderResponse, Error> {
    request_at(
        value,
        Path::new(GUEST_READER_SOCKET_PATH),
        Instant::now() + Duration::from_secs(12),
    )
}

fn request_at(
    value: ApplicationReaderRequest,
    path: &Path,
    deadline: Instant,
) -> Result<ReaderResponse, Error> {
    let mut socket = reader_io::ReaderIo::connect(path, deadline)?;
    let result =
        write_application_request(&mut socket, &value).and_then(|()| read_response(&mut socket));
    // The wire codec maps I/O failures to protocol refusals. Preserve the
    // original application deadline even after a partial ACK response.
    if Instant::now() >= deadline {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "MVM reader deadline").into());
    }
    Ok(result?)
}

/// Read a delivery without acknowledging it. An empty mailbox returns None.
/// MVM authenticates the local delivery lease before disclosing the message.
pub fn receive() -> Result<Option<Delivery>, Error> {
    decode_delivery(request(ApplicationReaderRequest::Receive)?)
}

fn decode_delivery(response: ReaderResponse) -> Result<Option<Delivery>, Error> {
    match response {
        ReaderResponse::Empty => Ok(None),
        ReaderResponse::Refused { reason } => Err(reason.into()),
        ReaderResponse::Delivery {
            lease,
            content_type,
            payload,
        } if content_type.name() == "application/vnd.mvm.guest-message"
            && content_type.version() == 1 =>
        {
            Ok(Some(Delivery {
                frame: Frame::decode(&payload)?,
                lease,
            }))
        }
        _ => Err(Error::Response("Receive")),
    }
}

#[cfg(test)]
mod tests;
