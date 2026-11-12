use super::*;
use std::cell::RefCell;

thread_local! {
    static AFTER_WRITE: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(super) fn after_record_write() {
    if let Some(f) = AFTER_WRITE.take() {
        f();
    }
}

#[test]
fn peer_closes_after_receiving_final_application_fin_before_sender_returns() {
    let (sender, peer) = crate::tls::readiness_tests::carriers();
    let mut observer = sender.try_clone().unwrap();
    let binding = Binding {
        flow: [1; 32],
        policy_sha256: [2; 32],
        scope_commitment: [3; 32],
    };
    let mapping = |tls, role: crate::Role| {
        TcpMapping::with_carrier(
            Carrier::Tls(tls),
            binding,
            Duration::from_secs(2),
            true,
            role.tag(),
            role.remote().tag(),
            None,
        )
        .unwrap()
    };
    let sender = mapping(sender, crate::Role::Initiator);
    let remote = mapping(peer, crate::Role::Acceptor);
    let remote_control = remote.cancellation();
    let (mut local_tx, mut local_rx) = sender.split();
    let (mut peer_tx, mut peer_rx) = remote.split();
    let until = Instant::now() + Duration::from_secs(2);
    peer_tx.application_half_close(until).unwrap();
    assert!(matches!(
        local_rx.receive_object(until).unwrap(),
        crate::ObjectRecord::ApplicationHalfClose
    ));
    AFTER_WRITE.set(Some(Box::new(move || {
        assert!(matches!(
            peer_rx.receive_object(until).unwrap(),
            crate::ObjectRecord::ApplicationHalfClose
        ));
        remote_control.complete();
        let eof = observer.peek(&mut [0]);
        assert!(
            matches!(eof, Ok(0))
                || matches!(eof, Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
            "original receive termination: {eof:?}"
        );
    })));
    local_tx
        .application_half_close(until)
        .expect("peer received our actual final FIN");
    assert_eq!(local_tx.counters().records, 1);
}

#[test]
fn fatal_cancel_after_fin_write_counts_bytes_without_completing_the_record() {
    let (sender, _peer) = crate::tls::readiness_tests::carriers();
    let binding = Binding {
        flow: [1; 32],
        policy_sha256: [2; 32],
        scope_commitment: [3; 32],
    };
    let mapping = TcpMapping::with_carrier(
        Carrier::Tls(sender),
        binding,
        Duration::from_secs(2),
        true,
        crate::Role::Initiator.tag(),
        crate::Role::Acceptor.tag(),
        None,
    )
    .unwrap();
    let cancel = mapping.cancellation();
    let (mut tx, _rx) = mapping.split();
    AFTER_WRITE.set(Some(Box::new(move || cancel.cancel())));
    let result = tx.application_half_close(Instant::now() + Duration::from_secs(2));
    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(tx.counters().wire_bytes, HEADER_LEN as u64);
    assert_eq!(tx.counters().records, 0);
}
