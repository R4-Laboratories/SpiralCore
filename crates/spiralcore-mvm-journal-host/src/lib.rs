//! Host-side mapping for an original, B-addressed SpiralCore Cast. Only
//! the explicit trusted-host local journal profile returns this V2 outcome.
#![forbid(unsafe_code)]
use journal_contract::protocol::mailbox::SignedDestinationReceipt;
use journal_core::mailbox::{
    decode_envelope, encode_envelope,
    frame::{Refusal, send_local_receipted_frame},
};
use mvm_agentd::vsock::{connect_unix_before, run_with_stream_deadline};
use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::transport::{CancellationToken, PreparedMailboxEnvelope, TransportPath};
use spiralcore_mvm_adapter::service::{AuthorizedSend, MailboxBinding, ServiceError, ServiceTime};
use std::{
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

/// Successful socket I/O through verification of the original signed receipt.
pub struct Exchange {
    pub receipt: SignedDestinationReceipt,
    pub bytes_to_journal: usize,
    pub bytes_from_journal: usize,
}

struct Meter<S> {
    stream: S,
    sent: usize,
    received: usize,
}
impl<S: Read> Read for Meter<S> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.stream.read(bytes)?;
        self.received += count;
        Ok(count)
    }
}
impl<S: Write> Write for Meter<S> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self.stream.write(bytes)?;
        self.sent += count;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("original send authorization: {0}")]
    Service(#[from] ServiceError),
    #[error("incompatible canonical MVM representation")]
    Encoding,
    #[error("local journal unavailable or operation timed out")]
    Unavailable,
    #[error("local journal receipt exchange: {0}")]
    Receipt(#[from] Refusal),
}

/// The route authority names B's Unix socket in the signed `unix:/absolute/path`
/// locator. ServiceTime comes from the source endpoint's trusted clock.
/// Keep the prepared envelope on every error, including a lost receipt.
pub fn send_cast(
    binding: &MailboxBinding,
    prepared: &PreparedMailboxEnvelope,
    cancel: &CancellationToken,
    now: ServiceTime,
    deadline: Instant,
) -> Result<Exchange, Error> {
    let started = Instant::now();
    let deadline = deadline.min(started + Duration::from_secs(5));
    binding.with_authorized_transport(prepared, cancel, now, |send| {
        Ok(exchange(send, cancel, now, started, deadline))
    })?
}

fn exchange(
    send: AuthorizedSend<'_>,
    cancel: &CancellationToken,
    now: ServiceTime,
    started: Instant,
    deadline: Instant,
) -> Result<Exchange, Error> {
    if send.header.kind != MailboxMessageKind::Cast
        || send.header.sender == send.header.recipient
        || send.header.content_type.name() != "application/vnd.mvm.guest-message"
        || send.header.content_type.version() != 1
        || send.route.path() != TransportPath::Direct
    {
        return Err(ServiceError::Unauthorized.into());
    }
    let path = send
        .route
        .locator()
        .as_str()
        .strip_prefix("unix:")
        .map(Path::new)
        .filter(|p| p.is_absolute())
        .ok_or(ServiceError::Unauthorized)?;
    let remain = u64::try_from(send.expires_at_unix_ms.saturating_sub(now.unix_ms))
        .map_err(|_| ServiceError::Expired)?;
    let deadline = started
        .checked_add(Duration::from_millis(remain))
        .unwrap_or(deadline)
        .min(deadline);
    if cancel.is_cancelled() {
        return Err(ServiceError::Cancelled.into());
    }

    // Bridge the pinned Rust types through their canonical representations.
    // Neither signing nor encryption occurs here. Refuse a changed encoding.
    let envelope = decode_envelope(send.envelope).map_err(|_| Error::Encoding)?;
    if encode_envelope(&envelope).map_err(|_| Error::Encoding)? != send.envelope {
        return Err(Error::Encoding);
    }
    let signature_input = serde_json::to_vec(send.capability).map_err(|_| Error::Encoding)?;
    let capability = serde_json::from_slice(&signature_input).map_err(|_| Error::Encoding)?;
    if serde_json::to_vec(&capability).map_err(|_| Error::Encoding)? != signature_input {
        return Err(Error::Encoding);
    }
    let header =
        serde_json::from_value(serde_json::to_value(send.header).map_err(|_| Error::Encoding)?)
            .map_err(|_| Error::Encoding)?;
    let mut stream = connect_unix_before(path, deadline).map_err(|_| Error::Unavailable)?;
    run_with_stream_deadline(&mut stream, deadline, |stream| {
        let mut io = Meter {
            stream,
            sent: 0,
            received: 0,
        };
        let result = send_local_receipted_frame(
            &mut io,
            &capability,
            &envelope,
            &header,
            send.destination_key,
        );
        Ok(result.map(|receipt| Exchange {
            receipt,
            bytes_to_journal: io.sent,
            bytes_from_journal: io.received,
        }))
    })
    .map_err(|_| Error::Unavailable)?
    .map_err(Error::Receipt)
}
