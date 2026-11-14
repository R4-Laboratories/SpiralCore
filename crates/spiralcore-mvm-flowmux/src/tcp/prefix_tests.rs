use super::*;
use spiralcore_transfer::protocol::Frame;
use std::net::TcpListener;

fn pair() -> (TcpStream, Tx, Rx, Binding) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    let binding = Binding {
        flow: [1; 32],
        policy_sha256: [2; 32],
        scope_commitment: [3; 32],
    };
    let mapping = TcpMapping::with_carrier(
        Carrier::Tcp(server),
        binding,
        Duration::from_secs(2),
        true,
        1,
        0,
        None,
    )
    .unwrap();
    let (tx, rx) = mapping.split();
    (client, tx, rx, binding)
}
fn frame() -> Frame {
    Frame {
        kind: 101,
        transfer: [8; 32],
        seq: 7,
        body: vec![19; 1024],
    }
}
fn send(peer: &mut TcpStream, binding: Binding, sequence: u64, kind: u8, body: &[u8]) {
    peer.write_all(&record::header(binding, sequence, kind, body.len(), 0).unwrap())
        .unwrap();
    peer.write_all(body).unwrap();
}
#[test]
fn fragmented_prefix_resumes_once_with_exact_counters_and_sequence() {
    let (mut peer, _tx, mut rx, binding) = pair();
    let original = frame();
    let body = original.encode().unwrap();
    let header = record::header(binding, 0, record::OBJECT, body.len(), 0).unwrap();
    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            for byte in header.iter().chain(body.iter()) {
                peer.write_all(&[*byte]).unwrap();
            }
        });
        let until = Instant::now() + Duration::from_secs(2);
        assert!(rx.prepare_object(until).unwrap());
        assert_eq!(rx.object_deadline(), Some(until));
        assert_eq!(rx.counters().wire_bytes, (HEADER_LEN + 9) as u64);
        assert_eq!(rx.counters().records, 0);
        assert_eq!(rx.sequence, 0);
        assert!(rx.poll_input().unwrap());
        match rx.receive_object(until).unwrap() {
            crate::ObjectRecord::Frame(crate::Purpose::Application, value) => {
                assert_eq!(value, original)
            }
            _ => panic!("original object expected"),
        }
        assert_eq!(rx.counters().wire_bytes, (HEADER_LEN + body.len()) as u64);
        assert_eq!(rx.counters().payload_bytes, body.len() as u64);
        assert_eq!(rx.counters().records, 1);
        assert_eq!(rx.sequence, 1);
        writer.join().unwrap();
    });
}
#[test]
fn truncated_prefix_and_short_inner_frame_refuse_without_committing_record() {
    for length in [4, 8] {
        let (mut peer, _tx, mut rx, binding) = pair();
        let body = frame().encode().unwrap();
        if length == 4 {
            peer.write_all(&record::header(binding, 0, record::OBJECT, body.len(), 0).unwrap())
                .unwrap();
            peer.write_all(&body[..4]).unwrap();
            peer.shutdown(Shutdown::Write).unwrap();
            assert!(matches!(
                rx.prepare_object(Instant::now() + Duration::from_secs(1)),
                Err(Error::Truncated)
            ));
        } else {
            send(&mut peer, binding, 0, record::OBJECT, &body[..8]);
            let until = Instant::now() + Duration::from_secs(1);
            assert!(!rx.prepare_object(until).unwrap());
            assert!(matches!(rx.receive_object(until), Err(Error::Core(_))));
        }
        assert_eq!(rx.counters().records, 0);
        assert_eq!(rx.sequence, 0);
        assert!(rx.control.stopped.load(Ordering::Acquire));
    }
}
#[test]
fn deferred_controls_do_not_read_the_following_frame() {
    for kind in [record::APP_FIN, record::FIN] {
        let (mut peer, _tx, mut rx, binding) = pair();
        send(&mut peer, binding, 0, kind, &[]);
        send(
            &mut peer,
            binding,
            1,
            record::OBJECT,
            &frame().encode().unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(1);
        assert!(!rx.prepare_object(until).unwrap());
        assert_eq!(rx.counters().wire_bytes, HEADER_LEN as u64);
        assert!(rx.poll_input().unwrap());
        let result = rx.receive_object(until).unwrap();
        assert!(matches!(
            (kind, result),
            (record::APP_FIN, crate::ObjectRecord::ApplicationHalfClose)
                | (record::FIN, crate::ObjectRecord::TransportFin)
        ));
        assert_eq!(rx.sequence, 1);
        assert_eq!(rx.counters().records, 1);
        assert_eq!(rx.counters().wire_bytes, HEADER_LEN as u64);
    }
}
#[test]
fn deferred_prefix_cannot_refresh_deadline_or_bypass_cancellation() {
    for cancel in [false, true] {
        let (mut peer, _tx, mut rx, binding) = pair();
        send(
            &mut peer,
            binding,
            0,
            record::OBJECT,
            &frame().encode().unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(1);
        rx.prepare_object(until).unwrap();
        if cancel {
            rx.control.stop();
        } else {
            rx.object_prefix.as_mut().unwrap().until = Instant::now();
        }
        let result = rx.receive_object(Instant::now() + Duration::from_secs(2));
        assert!(if cancel {
            matches!(result, Err(Error::Cancelled))
        } else {
            matches!(result, Err(Error::Timeout))
        });
        assert_eq!(rx.counters().records, 0);
        assert_eq!(rx.sequence, 0);
    }
}
