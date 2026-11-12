//! Object-v1 at the guest application boundary. MVM's paired reader owns
//! enrollment, lease verification, correlation and the guest's ACK key.
use anyhow::{Result, ensure};
use mvm_contract::protocol::mailbox::{
    MailboxEnvelopeHeader, MailboxMessageKind, SignedMailboxDeliveryLease,
};
use mvm_core::mailbox::reader_wire::{
    ApplicationReaderRequest, ReaderResponse, read_response, write_application_request,
};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before};
use spiralcore_transfer::{
    endpoint::{Authority, Context, Endpoint},
    protocol::Frame,
};
use std::{
    io::{Read, Write},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// Obtained from the canonical per-workload socket, after the guest agent verified
/// the destination signature, original boot, independent Receive scope and projection.
pub struct ObjectDelivery {
    pair: [u8; 32],
    header: Box<MailboxEnvelopeHeader>,
    lease: Box<SignedMailboxDeliveryLease>,
    frame: Frame,
}
impl ObjectDelivery {
    pub fn receive_before(
        pair: [u8; 32],
        deadline: Instant,
        traffic: &mut ReaderTraffic,
    ) -> Result<Option<Self>> {
        let mut io = application_reader_before(deadline, traffic)?;
        Self::receive_on(&mut io, pair)
    }
    fn receive_on(io: &mut (impl Read + Write), pair: [u8; 32]) -> Result<Option<Self>> {
        write_application_request(io, &ApplicationReaderRequest::ReceiveObjectPairV1 { pair })?;
        match read_response(io)? {
            ReaderResponse::Empty => Ok(None),
            ReaderResponse::ObjectDeliveryV1 {
                lease,
                header,
                payload,
            } => {
                ensure!(
                    lease.lease.object_projection_v1
                        == Some(mvm_core::mailbox::reader::object_delivery_commitment(
                            &pair, &header, &payload
                        )?),
                    "object lease projection changed"
                );
                let frame = Frame::decode(&payload)?;
                ensure!(
                    match header.kind {
                        MailboxMessageKind::Call => matches!(frame.kind, 1..=6),
                        MailboxMessageKind::Reply => matches!(frame.kind, 101..=105 | 127),
                        _ => false,
                    },
                    "wrong object frame for original operation"
                );
                Ok(Some(Self {
                    pair,
                    header,
                    lease,
                    frame,
                }))
            }
            ReaderResponse::Refused { reason } => Err(reason.into()),
            _ => anyhow::bail!("expected an original object-pair delivery"),
        }
    }
    pub fn header(&self) -> &MailboxEnvelopeHeader {
        &self.header
    }
    /// A current application Authority must still authorize the same scope for
    /// cache lookup, storage and readback in the shared receiver.
    pub fn object_context(&self) -> Context {
        let h = &self.header;
        Context {
            tenant: hex::encode(h.tenant.as_bytes()),
            domain: hex::encode(h.authorization_domain.as_bytes()),
            sender: hex::encode(h.sender.as_bytes()),
            recipient: hex::encode(h.recipient.as_bytes()),
            mailbox: hex::encode(h.mailbox.as_bytes()),
            capability: hex::encode(h.capability_id.as_bytes()),
            authorization_scope: hex::encode(self.pair),
            policy_epoch: h.policy_epoch,
            execution_epoch: h.execution_epoch.get(),
        }
    }
    fn live(&self) -> Result<()> {
        let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        ensure!(
            (self.lease.lease.issued_at_unix_ms..self.lease.lease.expires_at_unix_ms)
                .contains(&now)
                && (self.header.issued_at_unix_ms..self.header.expires_at_unix_ms).contains(&now),
            "original object lease expired"
        );
        Ok(())
    }
    /// Compute B's application verdict. Its transport and the subsequent
    /// original lease ACK remain independently admitted operations.
    pub fn handle_call<A: Authority>(
        &self,
        endpoint: &mut Endpoint<A>,
        monotonic_seconds: u64,
    ) -> Result<Frame> {
        self.live()?;
        ensure!(
            self.header.kind == MailboxMessageKind::Call,
            "expected an admitted Call"
        );
        let reply = match endpoint.handle(&self.object_context(), &self.frame, monotonic_seconds) {
            Ok(reply) => reply,
            Err(error) => self.frame.reply(127, (error as u16).to_le_bytes().to_vec()),
        };
        self.live()?;
        Ok(reply)
    }
    /// A's original pending Call must already have passed MVM's fresh
    /// correlation checks at lease issuance. The shared sender checks the frame.
    pub fn reply_frame(&self) -> Result<&Frame> {
        self.live()?;
        ensure!(
            self.header.kind == MailboxMessageKind::Reply,
            "expected a correlated Reply"
        );
        Ok(&self.frame)
    }
}
