//! Object-v1 application operations on the selected MVM durable reader.
use crate::object_reader::ObjectDelivery;
use anyhow::{Result, ensure};
use mvm_contract::protocol::mailbox::{CorrelationId, MessageId};
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

fn call_id(pair: &Id, frame: &Frame) -> MessageId {
    MessageId::from_bytes(operation_id(
        b"spiralcore.mvm.object.call.v1\0",
        pair,
        frame,
    ))
}

fn correlation(pair: &Id, frame: &Frame) -> CorrelationId {
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

fn stage_before(
    pair: Id,
    submission: ObjectSubmissionV1,
    deadline: Instant,
    traffic: &mut ReaderTraffic,
) -> Result<()> {
    let mut io = application_reader_before(deadline, traffic)?;
    stage_on(&mut io, pair, submission)?;
    ensure!(Instant::now() < deadline, "object staging deadline expired");
    Ok(())
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
                stage_before(
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
                    std::thread::sleep(
                        Duration::from_millis(5)
                            .min(deadline.saturating_duration_since(Instant::now())),
                    );
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

/// B stages its computed response against the original signed Call lease.
/// This operation never ACKs that lease. The installed owner must first retain
/// the response and the authority required for its retry across a lost receipt.
pub fn stage_response_before<A: Authority>(
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
    let pair = call.pair();
    let id = hash(
        b"spiralcore.mvm.object.reply.v1\0",
        &[&pair, call.header().message_id.as_bytes()],
    );
    let submission = ObjectSubmissionV1::Reply {
        message_id: MessageId::from_bytes(id),
        original_call: Box::new(call.lease().clone()),
        payload: reply.encode()?,
    };
    stage_before(pair, submission, deadline, traffic)
}
#[cfg(test)]
mod tests {
    use super::*;
    use mvm_core::mailbox::reader_wire::{read_application_request, write_response};
    use std::os::unix::net::UnixStream;

    #[test]
    fn stage_requires_exact_local_pair_and_message_confirmation() {
        let pair = [7; 32];
        let request = Frame {
            kind: 1,
            transfer: [8; 32],
            seq: 0,
            body: vec![0, 9, 0],
        };
        for mismatch in [None, Some("pair"), Some("message")] {
            let (mut app, mut agent) = UnixStream::pair().unwrap();
            let id = call_id(&pair, &request);
            let expected = request.encode().unwrap();
            let peer = std::thread::spawn(move || {
                let ApplicationReaderRequest::StageObjectV1 {
                    pair: got,
                    submission,
                } = read_application_request(&mut agent).unwrap()
                else {
                    panic!("wrong operation")
                };
                assert_eq!(got, pair);
                let ObjectSubmissionV1::Call {
                    message_id,
                    correlation_id,
                    payload,
                } = *submission
                else {
                    panic!("wrong direction")
                };
                assert_eq!(message_id, id);
                assert_ne!(correlation_id.as_bytes(), &[0; 32]);
                assert_eq!(payload, expected);
                write_response(
                    &mut agent,
                    &ReaderResponse::ObjectStagedV1 {
                        pair: if mismatch == Some("pair") {
                            [42; 32]
                        } else {
                            pair
                        },
                        message_id: if mismatch == Some("message") {
                            MessageId::from_bytes([9; 32])
                        } else {
                            id
                        },
                    },
                )
                .unwrap();
            });
            let result = stage_on(&mut app, pair, call_submission(&pair, &request).unwrap());
            peer.join().unwrap();
            assert_eq!(result.is_ok(), mismatch.is_none());
        }
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn replay_uses_the_same_logical_call_and_other_turns_cannot_alias_it() {
        let pair = [7; 32];
        let mut frame = Frame {
            kind: 2,
            transfer: [3; 32],
            seq: 1,
            body: vec![1, 2, 3],
        };
        let first = call_id(&pair, &frame);
        let parent = correlation(&pair, &frame);
        assert_ne!(first.as_bytes(), parent.as_bytes());
        // A conflicting retry stays under the original ID, to be refused by
        // MVM's immutable pending slot rather than issued as a new operation.
        frame.body.push(0);
        assert_eq!(first, call_id(&pair, &frame));
        assert_eq!(parent, correlation(&pair, &frame));
        assert_ne!(first, call_id(&[9; 32], &frame));
        frame.seq += 1;
        assert_ne!(first, call_id(&pair, &frame));
        frame.seq -= 1;
        frame.transfer[0] ^= 1;
        assert_ne!(first, call_id(&pair, &frame));
    }

    #[test]
    fn worker_rejects_oversized_payload_before_mailbox_access() {
        let task = serde_json::json!({"pair":"01".repeat(32),"transfer":"02".repeat(32),
            "payload_hex":"00".repeat(crate::fixture::BODY_LEN+1)});
        let err = run_source(&mut GuestObjectSource::default(), &task, Instant::now()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "object exceeds qualification workload size"
        );
    }
}

#[cfg(test)]
mod settlement_tests {
    use super::*;
    #[test]
    fn retry_retains_exact_lease_and_refuses_another_operation() {
        let pair = [1; 32];
        let transfer = [2; 32];
        let mut pending = Some(PendingReply {
            pair,
            transfer,
            seq: 2,
            delivery: vec![4, 5, 0],
        });
        let original = pending.as_ref().unwrap().delivery.as_ptr();
        settle_original(&mut pending, pair, transfer, 2, |lease| {
            assert_eq!(lease.as_ptr(), original);
            anyhow::bail!("lost ACK response")
        })
        .unwrap_err();
        for (p, t, s) in [
            ([3; 32], transfer, 2),
            (pair, [3; 32], 2),
            (pair, transfer, 1),
        ] {
            settle_original(&mut pending, p, t, s, |_| {
                panic!("unrelated settlement opened reader")
            })
            .unwrap_err();
        }
        settle_original(&mut pending, pair, transfer, 2, |lease| {
            assert_eq!(lease.as_ptr(), original);
            assert_eq!(lease, &[4, 5, 0]);
            Ok(())
        })
        .unwrap();
        assert!(pending.is_none());
        settle_original(&mut pending, pair, transfer, 2, |_| panic!("settled twice")).unwrap_err();
    }
}
