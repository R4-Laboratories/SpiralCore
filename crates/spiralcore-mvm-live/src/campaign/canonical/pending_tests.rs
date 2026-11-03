use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use mvm_core::mailbox::sign_receipt;
use std::os::unix::net::UnixStream;

#[test]
fn two_authenticated_call_receipts_without_reply_retain_pending_identity() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &broker).unwrap();
    let peer_broker = broker_context().unwrap();
    let mut peer = Session::new(&pair.receiver, &peer_broker).unwrap();
    let destination_key = SigningKey::from_bytes(
        &pair
            .receiver
            .service
            .as_ref()
            .unwrap()
            .local_receipt_signing,
    );
    let (socket, peer_socket) = UnixStream::pair().unwrap();
    let stream = StreamTransport::new(
        socket,
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        Duration::from_secs(5),
    )
    .unwrap();
    let peer_stream = StreamTransport::new(
        peer_socket,
        pair.receiver
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        Duration::from_secs(5),
    )
    .unwrap();
    let cancel = CancellationToken::default();
    let peer_worker = std::thread::spawn(move || -> Result<()> {
        let cancel = CancellationToken::default();
        peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
        let request = peer_stream.receive(&cancel, Instant::now() + Duration::from_secs(5))?;
        let (first, second) = peer_broker.with_time(|_, now| -> Result<_> {
            let (accepted, first) =
                peer.returns
                    .admit(&mut peer.service, &peer.incoming, &request, now)?;
            assert_eq!(accepted.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
            // An identical signed receipt is suppressed by ReceiptTracker and
            // would stop the caller before its completion path. A destination
            // can attest the same current accepted delivery at another valid
            // observation time, giving the tracker a second authenticated
            // fingerprint without supplying an application Reply.
            let mut observed_again = accepted.receipt.clone();
            observed_again.observed_at_unix_ms =
                observed_again.observed_at_unix_ms.checked_add(1).unwrap();
            let signed_again = sign_receipt(observed_again, &destination_key)?;
            let second = peer
                .returns
                .reserve(&peer.incoming, &request, now)?
                .seal(&signed_again, now)?;
            Ok((first, second))
        })?;
        for receipt in [&first, &second] {
            let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                Ok((receipt.reserve_send(&cancel, now)?, now))
            })?;
            permit.send(&peer_stream, &cancel, now)?;
        }
        assert_eq!(peer_stream.counters()?.frames_sent, 2);
        Ok(())
    });
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let request = Frame {
        kind: 6,
        transfer: [19; 32],
        seq: 0,
        body: Vec::new(),
    };
    let error = caller.call(&request, &stream, &cancel).unwrap_err();
    peer_worker.join().unwrap().unwrap();
    // The bounded event reader waits for the missing Reply; the peer has now
    // closed. Transport failure must still preserve the original identity.
    assert!(
        error
            .downcast_ref::<spiralcore_mvm_adapter::live::LiveError>()
            .is_some()
    );
    let counters = stream.counters().unwrap();
    assert_eq!((counters.frames_sent, counters.frames_received), (1, 2));
    assert_eq!(
        (caller.calls, caller.pending.is_some()),
        (0, true),
        "authenticated Call admission alone cannot complete the application Call"
    );
    assert!(caller.pending.as_ref().unwrap().send_started());
}
