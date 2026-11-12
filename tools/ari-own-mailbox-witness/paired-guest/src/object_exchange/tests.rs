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

#[test]
fn only_matching_pending_can_be_polled_and_only_original_v2_can_finish() {
    use mvm_contract::protocol::mailbox::*;
    let pair = [11; 32];
    let id = MessageId::from_bytes([12; 32]);
    let terminal = |message_id, outcome, protocol_version| ReaderResponse::ObjectAcceptedV1 {
        receipt: Box::new(SignedDestinationReceipt {
            receipt: DestinationReceipt {
                protocol_version,
                signature_algorithm: SignatureAlgorithm::ED25519,
                delivery_id: DeliveryId::from_bytes([14; 32]),
                message_id,
                address: MailboxAddress {
                    tenant: TenantId::from_bytes([1; 32]),
                    authorization_domain: AuthorizationDomainId::from_bytes([2; 32]),
                    recipient: PrincipalId::from_bytes([3; 32]),
                    mailbox: MailboxId::from_bytes([4; 32]),
                },
                execution_epoch: ExecutionEpoch::new(1).unwrap(),
                outcome,
                observed_at_unix_ms: 20,
                ciphertext_commitment: [5; 32],
            },
            signature: vec![0; ED25519_SIGNATURE_BYTES],
        }),
    };
    // The guest agent owns cryptographic verification; these fixtures test the
    // application response discriminator across the actual MVM wire codec.
    for (response, accepted) in [
        (
            ReaderResponse::ObjectPendingV1 {
                pair,
                message_id: id,
            },
            Some(false),
        ),
        (
            ReaderResponse::ObjectPendingV1 {
                pair: [0; 32],
                message_id: id,
            },
            None,
        ),
        (
            ReaderResponse::ObjectStagedV1 {
                pair,
                message_id: id,
            },
            None,
        ),
        (
            terminal(
                id,
                DeliveryOutcome::AcceptedLocalDurable,
                ProtocolVersion::V2,
            ),
            Some(true),
        ),
        (
            terminal(
                MessageId::from_bytes([44; 32]),
                DeliveryOutcome::AcceptedLocalDurable,
                ProtocolVersion::V2,
            ),
            None,
        ),
        (
            terminal(id, DeliveryOutcome::AcceptedDurable, ProtocolVersion::V1),
            None,
        ),
        (
            ReaderResponse::Refused {
                reason: mvm_core::mailbox::frame::Refusal::NotAuthorized,
            },
            None,
        ),
    ] {
        let (mut app, mut agent) = UnixStream::pair().unwrap();
        let peer = std::thread::spawn(move || {
            assert_eq!(
                read_application_request(&mut agent).unwrap(),
                ApplicationReaderRequest::ObjectSubmissionStatusV1 {
                    pair,
                    message_id: id
                }
            );
            write_response(&mut agent, &response).unwrap();
        });
        assert_eq!(status_on(&mut app, pair, id).ok(), accepted);
        peer.join().unwrap();
    }
}
