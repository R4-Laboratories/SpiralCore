//! Object-v1 application operations on the selected MVM durable reader.
use crate::object_reader::ObjectDelivery;
use anyhow::{Result, ensure};
use mvm_contract::protocol::mailbox::{CorrelationId, DeliveryOutcome, MessageId, ProtocolVersion};
use mvm_core::mailbox::reader_wire::{
    ApplicationReaderRequest, ObjectSubmissionV1, ReaderResponse, read_response,
    write_application_request,
};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before};
use spiralcore_transfer::{
    Id,
    client::{RawFrameLimits, Stats, transfer_raw_leased},
    endpoint::{Authority, Endpoint},
    hash,
    protocol::Frame,
};
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

fn operation_id(label: &[u8], pair: &Id, frame: &Frame) -> Id {
    hash(label, &[pair, &frame.transfer, &frame.seq.to_le_bytes()])
}

pub(super) fn call_id(pair: &Id, frame: &Frame) -> MessageId {
    MessageId::from_bytes(operation_id(
        b"spiralcore.mvm.object.call.v1\0",
        pair,
        frame,
    ))
}

pub(super) fn correlation(pair: &Id, frame: &Frame) -> CorrelationId {
    CorrelationId::from_bytes(operation_id(
        b"spiralcore.mvm.object.correlation.v1\0",
        pair,
        frame,
    ))
}

fn call_submission(pair: &Id, frame: &Frame) -> Result<ObjectSubmissionV1> {
    Ok(ObjectSubmissionV1::Call {
        message_id: call_id(pair, frame),
        correlation_id: correlation(pair, frame),
        payload: frame.encode()?,
    })
}

fn stage_on(io: &mut (impl Read + Write), pair: Id, submission: ObjectSubmissionV1) -> Result<()> {
    let expected = submission.message_id();
    write_application_request(
        io,
        &ApplicationReaderRequest::StageObjectV1 {
            pair,
            submission: Box::new(submission),
        },
    )?;
    match read_response(io)? {
        ReaderResponse::ObjectStagedV1 {
            pair: got,
            message_id,
        } if got == pair && message_id == expected => Ok(()),
        ReaderResponse::Refused { reason } => Err(reason.into()),
        _ => anyhow::bail!("missing matching local object staging confirmation"),
    }
}

/// The original guest agent checks the destination signature and its signed
/// role. Only its matching V2 terminal allows us to leave the local outbox.
fn status_on(io: &mut (impl Read + Write), pair: Id, id: MessageId) -> Result<bool> {
    write_application_request(
        io,
        &ApplicationReaderRequest::ObjectSubmissionStatusV1 {
            pair,
            message_id: id,
        },
    )?;
    match read_response(io)? {
        ReaderResponse::ObjectPendingV1 {
            pair: p,
            message_id,
        } if p == pair && message_id == id => Ok(false),
        ReaderResponse::ObjectAcceptedV1 { receipt }
            if receipt.receipt.message_id == id
                && receipt.receipt.protocol_version == ProtocolVersion::V2
                && receipt.receipt.outcome == DeliveryOutcome::AcceptedLocalDurable =>
        {
            Ok(true)
        }
        ReaderResponse::Refused { reason } => Err(reason.into()),
        _ => anyhow::bail!("missing original destination acceptance for object submission"),
    }
}

fn wait_to_poll(deadline: Instant) {
    std::thread::sleep(
        Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
    );
}

pub(super) fn submit_before(
    pair: Id,
    submission: ObjectSubmissionV1,
    deadline: Instant,
    traffic: &mut ReaderTraffic,
) -> Result<()> {
    let id = submission.message_id();
    {
        let mut io = application_reader_before(deadline, traffic)?;
        stage_on(&mut io, pair, submission)?;
    }
    loop {
        ensure!(
            Instant::now() < deadline,
            "object submission deadline expired"
        );
        let accepted = {
            let mut io = application_reader_before(deadline, traffic)?;
            status_on(&mut io, pair, id)?
        };
        ensure!(
            Instant::now() < deadline,
            "object submission status deadline expired"
        );
        if accepted {
            return Ok(());
        }
        wait_to_poll(deadline);
    }
}

/// The original mailbox retains durable intent. Keep the last accepted Reply
/// lease if its guest ACK is ambiguous, so the same owner can retry that ACK.
#[derive(Default)]
pub struct GuestObjectSource {
    unsettled: Option<PendingReply<ObjectDelivery>>,
}

struct PendingReply<L> {
    pair: Id,
    transfer: Id,
    seq: u64,
    delivery: L,
}

fn settle_original<L>(
    slot: &mut Option<PendingReply<L>>,
    pair: Id,
    transfer: Id,
    seq: u64,
    acknowledge: impl FnOnce(&L) -> Result<()>,
) -> Result<()> {
    let pending = slot
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no object Reply settlement pending"))?;
    ensure!(
        (pending.pair, pending.transfer, pending.seq) == (pair, transfer, seq),
        "settlement does not identify the pending object Reply"
    );
    acknowledge(&pending.delivery)?;
    *slot = None;
    Ok(())
}

#[derive(Default)]
pub struct ObjectReaderTraffic {
    pub operations: ReaderTraffic,
    pub acknowledgments: ReaderTraffic,
}

impl GuestObjectSource {
    pub fn settle_before(
        &mut self,
        pair: Id,
        transfer: Id,
        seq: u64,
        deadline: Instant,
        traffic: &mut ReaderTraffic,
    ) -> Result<()> {
        ensure!(
            Instant::now() < deadline,
            "object Reply settlement deadline expired"
        );
        settle_original(&mut self.unsettled, pair, transfer, seq, |delivery| {
            delivery.acknowledge_before(deadline, traffic)
        })
    }

    /// A new logical transfer needs its own stable transfer ID. Reusing that
    /// ID reproduces each original Call identity and correlation exactly.
    pub fn send_before(
        &mut self,
        pair: Id,
        transfer: Id,
        bytes: &[u8],
        limits: RawFrameLimits,
        deadline: Instant,
        traffic: &mut ObjectReaderTraffic,
    ) -> Result<Stats> {
        ensure!(
            self.unsettled.is_none(),
            "original object Reply still needs settlement"
        );
        transfer_raw_leased(
            bytes,
            transfer,
            limits,
            |request| -> Result<_> {
                submit_before(
                    pair,
                    call_submission(&pair, request)?,
                    deadline,
                    &mut traffic.operations,
                )?;
                loop {
                    ensure!(
                        Instant::now() < deadline,
                        "waiting for original object Reply timed out"
                    );
                    if let Some(delivery) =
                        ObjectDelivery::receive_before(pair, deadline, &mut traffic.operations)?
                    {
                        let h = delivery.header();
                        ensure!(
                            h.correlation_id == correlation(&pair, request)
                                && h.causation_id.as_bytes() == call_id(&pair, request).as_bytes(),
                            "Reply does not identify the outstanding object Call"
                        );
                        return Ok((delivery.reply_frame()?.clone(), (request.seq, delivery)));
                    }
                    wait_to_poll(deadline);
                }
            },
            |(seq, delivery)| {
                self.unsettled = Some(PendingReply {
                    pair,
                    transfer,
                    seq,
                    delivery,
                });
                self.settle_before(pair, transfer, seq, deadline, &mut traffic.acknowledgments)
                    .map_err(|error| {
                        error.context(format!(
                            "retry original Reply settlement for sequence {seq}"
                        ))
                    })
            },
        )
    }
}

/// Worker qualification entry. Application byte limits here constrain the
/// fixture; the MVM owner independently enforces its signed grants at dispatch.
pub(super) fn run_source(
    source: &mut GuestObjectSource,
    task: &serde_json::Value,
    deadline: Instant,
) -> Result<serde_json::Value> {
    let pair = crate::field(task, "pair")?;
    let transfer = crate::field(task, "transfer")?;
    ensure!(
        task.as_object().is_some_and(|fields| fields.len() == 3),
        "unexpected object sender input"
    );
    let payload = task["payload_hex"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("object payload required"))?;
    ensure!(
        payload.len() <= 2 * crate::fixture::BODY_LEN,
        "object exceeds qualification workload size"
    );
    let bytes = hex::decode(payload)?;
    let mut traffic = ObjectReaderTraffic::default();
    let stats = source.send_before(
        pair,
        transfer,
        &bytes,
        RawFrameLimits {
            request_bytes: 4152,
            reply_bytes: 88,
        },
        deadline,
        &mut traffic,
    )?;
    Ok(serde_json::json!({
        "object_id":hex::encode(stats.object), "object_bytes":bytes.len(),
        "object_request_bytes":stats.request_bytes, "object_reply_bytes":stats.response_bytes,
        "data_bytes":stats.data_bytes, "calls":stats.requests,
        "operations_to_agent":traffic.operations.written, "operations_from_agent":traffic.operations.read,
        "ack_to_agent":traffic.acknowledgments.written, "ack_from_agent":traffic.acknowledgments.read
    }))
}

pub(super) fn retry_settlement(
    source: &mut GuestObjectSource,
    task: &serde_json::Value,
    deadline: Instant,
) -> Result<serde_json::Value> {
    let pair = crate::field(task, "pair")?;
    let transfer = crate::field(task, "transfer")?;
    let seq = task["seq"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("original Reply sequence required"))?;
    ensure!(
        task.as_object().is_some_and(|v| v.len() == 3),
        "unexpected settlement parameters"
    );
    let mut traffic = ReaderTraffic::default();
    source.settle_before(pair, transfer, seq, deadline, &mut traffic)?;
    Ok(
        serde_json::json!({"settled_reply_seq":seq,"ack_to_agent":traffic.written,"ack_from_agent":traffic.read}),
    )
}

/// B submits the computed response against the original signed Call lease and
/// waits for original V2 destination acceptance. The caller retains the Call
/// lease and may now ACK it, retrying that same ACK on ambiguous completion.
pub fn send_response_before<A: Authority>(
    call: &ObjectDelivery,
    endpoint: &mut Endpoint<A>,
    monotonic_seconds: u64,
    deadline: Instant,
    traffic: &mut ReaderTraffic,
) -> Result<()> {
    ensure!(
        Instant::now() < deadline,
        "original Call processing deadline expired"
    );
    let reply = call.handle_call(endpoint, monotonic_seconds)?;
    submit_before(
        call.pair(),
        response_submission(call, &reply)?,
        deadline,
        traffic,
    )
}

pub(super) fn response_submission(
    call: &ObjectDelivery,
    reply: &Frame,
) -> Result<ObjectSubmissionV1> {
    let pair = call.pair();
    let id = hash(
        b"spiralcore.mvm.object.reply.v1\0",
        &[&pair, call.header().message_id.as_bytes()],
    );
    Ok(ObjectSubmissionV1::Reply {
        message_id: MessageId::from_bytes(id),
        original_call: Box::new(call.lease().clone()),
        payload: reply.encode()?,
    })
}
#[cfg(test)]
mod tests;
