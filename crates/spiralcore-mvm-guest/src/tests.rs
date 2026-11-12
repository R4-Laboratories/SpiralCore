use super::*;

#[test]
fn local_stage_preserves_binary_frame_id_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let slot = dir.path().join("outbox-local");
    let frame = Frame {
        kind: 3,
        transfer: [7; 32],
        seq: 42,
        body: (0..=255).chain([0]).collect(),
    };
    let pending = PendingFrame::new(&frame).unwrap();
    pending.stage_at(&slot).unwrap();
    let original = std::fs::read(&slot).unwrap();
    let staged = OutboxMessage::decode(&original).unwrap();
    assert_eq!(staged.recipient.as_bytes(), &OWN_MAILBOX_RECIPIENT);
    assert_eq!(staged.message_id, pending.message_id());
    assert_eq!(Frame::decode(&staged.body).unwrap(), frame);
    PendingFrame::restore(pending.message_id(), pending.encoded().to_vec())
        .unwrap()
        .stage_at(&slot)
        .unwrap();
    assert!(PendingFrame::new(&frame).unwrap().stage_at(&slot).is_err());
    assert_eq!(std::fs::read(&slot).unwrap(), original);
    assert!(PendingFrame::restore(pending.message_id(), vec![0; 56]).is_err());
}

#[test]
fn empty_and_refused_reader_responses_are_distinct() {
    assert!(decode_delivery(ReaderResponse::Empty).unwrap().is_none());
    assert!(matches!(
        decode_delivery(ReaderResponse::Refused {
            reason: Refusal::NotAuthorized
        }),
        Err(Error::Mailbox(Refusal::NotAuthorized))
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn standard_mailbox_application_stages_receives_and_explicitly_acks_exact_frame() {
    use ed25519_dalek::SigningKey;
    use mvm_contract::protocol::mailbox::*;
    use mvm_core::mailbox::reader::{sign_delivery_ack, sign_delivery_lease};
    use mvm_core::mailbox::reader_wire::{read_application_request, write_response};
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::mpsc;

    struct Meter {
        stream: UnixStream,
        read: usize,
        wrote: usize,
    }
    impl Read for Meter {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let n = self.stream.read(b)?;
            self.read += n;
            Ok(n)
        }
    }
    impl Write for Meter {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            let n = self.stream.write(b)?;
            self.wrote += n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.stream.flush()
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receive.sock");
    let slot = dir.path().join("outbox-local");
    let listener = UnixListener::bind(&path).unwrap();
    let (staged_tx, staged_rx) = mpsc::channel();
    let expected = Frame {
        kind: 2,
        seq: 1,
        transfer: [7; 32],
        body: (0..=255).chain([0]).collect(),
    };
    let worker = std::thread::spawn(move || {
        let cpu = cpu_time::ThreadTime::now();
        let start = Instant::now();
        mvm_agentd::workload_seccomp::admit_from_cmdline("mvm.seccomp=standard mvm.mailbox=1")
            .unwrap();
        mvm_agentd::workload_seccomp::install_admitted().unwrap();
        let pending = PendingFrame::new(&expected).unwrap();
        pending.stage_at(&slot).unwrap();
        staged_tx.send(pending.message_id()).unwrap();
        let request = |value| request_at(value, &path, Instant::now() + Duration::from_secs(2));
        let delivery = decode_delivery(request(ApplicationReaderRequest::Receive).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(delivery.frame(), &expected);
        assert_eq!(delivery.message_id(), pending.message_id());
        delivery.ack_with(request).unwrap();
        (cpu.elapsed(), start.elapsed())
    });
    let id = staged_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let mail =
        OutboxMessage::decode(&std::fs::read(dir.path().join("outbox-local")).unwrap()).unwrap();
    assert_eq!(mail.message_id, id);
    let lease = sign_delivery_lease(
        MailboxDeliveryLease {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            address: MailboxAddress {
                tenant: TenantId::from_bytes([1; 32]),
                authorization_domain: AuthorizationDomainId::from_bytes([2; 32]),
                mailbox: MailboxId::from_bytes([3; 32]),
                recipient: PrincipalId::from_bytes([4; 32]),
            },
            sender: PrincipalId::from_bytes([5; 32]),
            sender_boot: BootId::from_bytes([6; 32]),
            message_id: id,
            delivery_id: DeliveryId::from_bytes([8; 32]),
            execution_epoch: ExecutionEpoch::new(1).unwrap(),
            consumer: PrincipalId::from_bytes([4; 32]),
            consumer_session: ConsumerSessionId::from_bytes([11; 32]),
            receive_capability_id: CapabilityId::from_bytes([12; 32]),
            policy_epoch: 1,
            ciphertext_commitment: [13; 32],
            issued_at_unix_ms: 100,
            expires_at_unix_ms: 200,
            lease_nonce: DeliveryLeaseNonce::from_bytes([14; 16]),
        },
        &SigningKey::from_bytes(&[9; 32]),
    )
    .unwrap();
    let signed_ack = sign_delivery_ack(
        MailboxDeliveryAck {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            lease: lease.clone(),
            ack_nonce: DeliveryAckNonce::from_bytes([15; 16]),
            acknowledged_at_unix_ms: 150,
        },
        &SigningKey::from_bytes(&[10; 32]),
    )
    .unwrap();
    let mut operations = Vec::new();
    let unix_setup = Instant::now();
    listener.set_nonblocking(true).unwrap();
    for op in [
        ApplicationReaderRequest::Receive,
        ApplicationReaderRequest::Ack {
            lease: Box::new(lease.clone()),
        },
    ] {
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        && unix_setup.elapsed() < Duration::from_secs(5) =>
                {
                    std::thread::sleep(Duration::from_millis(1))
                }
                Err(error) => panic!("reader fixture accept: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut meter = Meter {
            stream,
            read: 0,
            wrote: 0,
        };
        assert_eq!(read_application_request(&mut meter).unwrap(), op);
        let response = match op {
            ApplicationReaderRequest::Receive => ReaderResponse::Delivery {
                lease: Box::new(lease.clone()),
                content_type: ContentType::parse("application/vnd.mvm.guest-message", 1).unwrap(),
                payload: mail.body.clone(),
            },
            ApplicationReaderRequest::Ack { .. } => ReaderResponse::Acknowledged {
                ack: Box::new(signed_ack.clone()),
            },
        };
        write_response(&mut meter, &response).unwrap();
        operations.push((meter.read, meter.wrote));
    }
    let (cpu, wall) = worker.join().unwrap();
    println!(
        "frame_body_bytes=257 encoded_bytes={} local_outbox_file_bytes={} receive_request_bytes={} delivery_response_bytes={} ack_request_bytes={} ack_response_bytes={} application_thread_cpu_ns={} application_wall_ns={}",
        mail.body.len(),
        mail.encode().len(),
        operations[0].0,
        operations[0].1,
        operations[1].0,
        operations[1].1,
        cpu.as_nanos(),
        wall.as_nanos()
    );

    let delivery = Delivery {
        frame: Frame::decode(&mail.body).unwrap(),
        lease: Box::new(lease.clone()),
    };
    let mut unrelated = signed_ack;
    unrelated.ack.lease.lease.delivery_id = DeliveryId::from_bytes([99; 32]);
    assert!(matches!(
        delivery.ack_with(|_| Ok(ReaderResponse::Acknowledged {
            ack: Box::new(unrelated)
        })),
        Err(Error::Response("ACK"))
    ));
    assert!(matches!(
        decode_delivery(ReaderResponse::Delivery {
            lease: Box::new(lease),
            content_type: ContentType::parse("application/vnd.mvm.guest-message", 1).unwrap(),
            payload: vec![0, 1, 0],
        }),
        Err(Error::Frame(_))
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn reader_deadline_preserves_ambiguous_partial_response() {
    use std::{io::Write, os::unix::net::UnixListener};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receive.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (done, stop) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        mvm_core::mailbox::reader_wire::read_application_request(&mut stream).unwrap();
        stream.write_all(&[0]).unwrap();
        stop.recv_timeout(Duration::from_secs(3)).unwrap();
    });
    let deadline = Instant::now() + Duration::from_millis(120);
    let response = request_at(ApplicationReaderRequest::Receive, &path, deadline);
    done.send(()).unwrap();
    server.join().unwrap();
    assert!(matches!(response, Err(Error::Io(ref e)) if e.kind()==io::ErrorKind::TimedOut));
}
