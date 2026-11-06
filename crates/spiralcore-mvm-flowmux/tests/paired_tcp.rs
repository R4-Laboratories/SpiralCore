use spiralcore_mvm_flowmux::{Binding, Error, Event, HEADER_LEN, MAX_BLOCK, TcpMapping};
use std::{
    io::Write,
    net::{Shutdown, TcpListener, TcpStream},
    time::{Duration, Instant},
};

fn binding() -> Binding {
    Binding {
        flow: [7; 32],
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    }
}

fn pair(timeout: Duration) -> (TcpStream, TcpMapping) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let raw = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mapped, peer) = listener.accept().unwrap();
    (
        raw,
        TcpMapping::from_connected(mapped, binding(), peer, timeout).unwrap(),
    )
}
fn header(kind: u8, sequence: u64, len: u32) -> Vec<u8> {
    let b = binding();
    let mut bytes = b"SCFLW001".to_vec();
    bytes.extend_from_slice(&[kind, 0, 0, 0]);
    bytes.extend_from_slice(&b.directional_flow(spiralcore_mvm_flowmux::Role::Initiator));
    bytes.extend_from_slice(&b.policy_sha256);
    bytes.extend_from_slice(&b.scope_commitment);
    bytes.extend_from_slice(&sequence.to_be_bytes());
    bytes.extend_from_slice(&len.to_be_bytes());
    assert_eq!(bytes.len(), HEADER_LEN);
    bytes
}

#[test]
fn malformed_unavailable_peer_and_cross_binding_fence_without_payload() {
    for offset in [0, 9, 12, 44, 76] {
        let (mut raw, mapping) = pair(Duration::from_secs(1));
        let (_tx, mut rx) = mapping.split();
        let mut wire = header(2, 0, 0);
        wire[offset] ^= 1;
        raw.write_all(&wire).unwrap();
        let error = rx.receive().unwrap_err();
        assert!(if offset < 12 {
            matches!(error, Error::Protocol)
        } else {
            matches!(error, Error::InvalidBinding)
        });
        assert_eq!(rx.counters().payload_bytes, 0);
        assert_eq!(rx.counters().wire_bytes, HEADER_LEN as u64);
        assert!(matches!(rx.receive(), Err(Error::Cancelled)));
    }
}

#[test]
fn sequence_oversize_and_truncation_are_terminal_and_count_actual_bytes() {
    for (wire, expected) in [
        (header(2, 1, 0), "sequence"),
        (header(1, 0, u32::MAX), "limit"),
        (vec![b'S'; 17], "truncated"),
    ] {
        let (mut raw, mapping) = pair(Duration::from_secs(1));
        let (_tx, mut rx) = mapping.split();
        raw.write_all(&wire).unwrap();
        raw.shutdown(Shutdown::Write).unwrap();
        let error = rx.receive().unwrap_err();
        assert!(match expected {
            "sequence" => matches!(error, Error::Sequence),
            "limit" => matches!(error, Error::Limit),
            _ => matches!(error, Error::Truncated),
        });
        assert_eq!(rx.counters().wire_bytes, wire.len() as u64);
        assert_eq!(rx.counters().payload_bytes, 0);
        assert!(matches!(rx.receive(), Err(Error::Cancelled)));
    }
}

#[test]
fn original_deadline_and_explicit_cancel_bound_idle_receive() {
    let (mut raw, mapping) = pair(Duration::from_millis(70));
    let (_tx, mut rx) = mapping.split();
    raw.write_all(b"S").unwrap();
    let start = Instant::now();
    assert!(matches!(rx.receive(), Err(Error::Timeout)));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(rx.counters().wire_bytes, 1);
    assert!(matches!(rx.receive(), Err(Error::Cancelled)));
    let (_raw, mapping) = pair(Duration::from_secs(1));
    let cancellation = mapping.cancellation();
    let (_tx, mut rx) = mapping.split();
    cancellation.cancel();
    assert!(matches!(rx.receive(), Err(Error::Cancelled)));
    assert_eq!(rx.counters().wire_bytes, 0);
}

#[test]
fn invalid_send_sizes_write_nothing_and_valid_maximum_survives() {
    let (raw, mapping) = pair(Duration::from_secs(1));
    let (mut tx, _rx) = mapping.split();
    assert!(matches!(tx.send(&[]), Err(Error::Limit)));
    assert!(matches!(
        tx.send(&vec![0; MAX_BLOCK + 1]),
        Err(Error::Limit)
    ));
    assert_eq!(tx.counters().wire_bytes, 0);
    let peer = raw.peer_addr().unwrap();
    let (mut peer_tx, mut peer_rx) =
        TcpMapping::from_connected(raw, binding(), peer, Duration::from_secs(1))
            .unwrap()
            .split();
    tx.send(&vec![17; MAX_BLOCK]).unwrap();
    assert_eq!(peer_rx.receive().unwrap(), Event::Data(vec![17; MAX_BLOCK]));
    tx.half_close().unwrap();
    assert_eq!(peer_rx.receive().unwrap(), Event::HalfClose);
    peer_tx.half_close().unwrap();
}

#[test]
fn raw_entropy_roundtrip_and_corrupted_raw_identity_refusal() {
    let (raw, mapping) = pair(Duration::from_secs(1));
    let (mut tx, _rx) = mapping.split();
    let peer = raw.peer_addr().unwrap();
    let (_peer_tx, mut rx) =
        TcpMapping::from_connected(raw, binding(), peer, Duration::from_secs(1))
            .unwrap()
            .split();
    let mut state = 0x1234_5678_9abc_def0u64;
    let bytes: Vec<_> = (0..4096)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    tx.send(&bytes).unwrap();
    assert_eq!(rx.receive().unwrap(), Event::Data(bytes));
    assert_eq!(tx.counters().compressed_records, 0);
    assert_eq!(tx.counters().wire_bytes, rx.counters().wire_bytes);
    let (mut raw, mapping) = pair(Duration::from_secs(1));
    let (_tx, mut rx) = mapping.split();
    let offer = spiralcore_transfer::protocol::Offer::cold(
        b"original",
        spiralcore_transfer::protocol::Mode::Raw,
    )
    .encode()
    .unwrap();
    let mut body = (offer.len() as u16).to_be_bytes().to_vec();
    body.extend_from_slice(&offer);
    body.extend_from_slice(b"tampered");
    raw.write_all(&header(1, 0, body.len() as u32)).unwrap();
    raw.write_all(&body).unwrap();
    assert!(matches!(rx.receive(), Err(Error::Integrity)));
    assert_eq!(rx.counters().payload_bytes, 0);
    assert_eq!(rx.counters().wire_bytes, (HEADER_LEN + body.len()) as u64);
    assert!(matches!(rx.receive(), Err(Error::Cancelled)));
}

#[test]
fn explicit_reset_and_cancel_are_terminal_in_both_directions() {
    for cancel in [false, true] {
        let (raw, mapping) = pair(Duration::from_secs(1));
        let (mut tx, _rx) = mapping.split();
        let peer = raw.peer_addr().unwrap();
        let (mut peer_tx, mut peer_rx) =
            TcpMapping::from_connected(raw, binding(), peer, Duration::from_secs(1))
                .unwrap()
                .split();
        if cancel {
            tx.cancel().unwrap();
        } else {
            tx.reset().unwrap();
        }
        assert_eq!(
            peer_rx.receive().unwrap(),
            if cancel { Event::Cancel } else { Event::Reset }
        );
        assert!(matches!(peer_rx.receive(), Err(Error::Closed)));
        assert!(matches!(
            peer_tx.send(b"after terminal"),
            Err(Error::Cancelled)
        ));
        assert_eq!(peer_tx.counters().wire_bytes, 0);
        assert_eq!(tx.counters().wire_bytes, HEADER_LEN as u64);
    }
}

#[test]
fn closed_peer_buffered_reverse_data_and_fin_remain_exact_under_original_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    // Construct both mappings while the connected socket is fully live. Then
    // join the peer before reading: its queued reverse data and FIN must still
    // be consumed exactly after the last peer descriptor has actually closed.
    let (mut client_tx, mut client_rx) =
        TcpMapping::from_connected(client, binding(), address, Duration::from_secs(2))
            .unwrap()
            .split();
    let (mut server_tx, mut server_rx) =
        TcpMapping::from_connected(server, binding(), peer, Duration::from_secs(2))
            .unwrap()
            .split();
    let worker = std::thread::spawn(move || {
        assert_eq!(server_rx.receive().unwrap(), Event::HalfClose);
        server_tx.send(&vec![0x6b; 3072]).unwrap();
        server_tx.half_close().unwrap();
        (server_tx.counters(), server_rx.counters())
    });
    client_tx.half_close().unwrap();
    let (server_sent, server_received) = worker.join().unwrap();
    assert_eq!(client_rx.receive().unwrap(), Event::Data(vec![0x6b; 3072]));
    assert_eq!(client_rx.receive().unwrap(), Event::HalfClose);
    assert!(matches!(client_rx.receive(), Err(Error::Closed)));
    assert_eq!(client_rx.counters().payload_bytes, 3072);
    assert_eq!(client_rx.counters().wire_bytes, server_sent.wire_bytes);
    assert_eq!(client_tx.counters().wire_bytes, server_received.wire_bytes);
}

#[test]
fn exact_bidirectional_stream_and_reverse_after_half_close_use_real_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    std::thread::scope(|scope| {
        let server = scope.spawn(move || {
            let (mut tx, mut rx) =
                TcpMapping::from_connected(server, binding(), peer, Duration::from_secs(2))
                    .unwrap()
                    .split();
            assert_eq!(rx.receive().unwrap(), Event::Data(vec![b'a'; 4096]));
            assert_eq!(rx.receive().unwrap(), Event::HalfClose);
            assert!(
                rx.receive().is_err(),
                "terminal receive must not read again"
            );
            tx.send(&vec![b'b'; 3072]).unwrap();
            tx.half_close().unwrap();
            (tx.counters(), rx.counters())
        });
        let (mut tx, mut rx) =
            TcpMapping::from_connected(client, binding(), address, Duration::from_secs(2))
                .unwrap()
                .split();
        tx.send(&vec![b'a'; 4096]).unwrap();
        tx.half_close().unwrap();
        assert!(tx.send(b"after FIN").is_err());
        assert_eq!(rx.receive().unwrap(), Event::Data(vec![b'b'; 3072]));
        assert_eq!(rx.receive().unwrap(), Event::HalfClose);
        let (server_tx, server_rx) = server.join().unwrap();
        assert_eq!(tx.counters().payload_bytes, 4096);
        assert_eq!(rx.counters().payload_bytes, 3072);
        assert_eq!(tx.counters().wire_bytes, server_rx.wire_bytes);
        assert_eq!(rx.counters().wire_bytes, server_tx.wire_bytes);
        assert_eq!(tx.counters().compressed_records, 1);
        assert!(tx.counters().wire_bytes < 4096);
        println!(
            "FLOWMUX_COUNTERS tcp-whole payload_tx=4096 payload_rx=3072 wire_tx={} wire_rx={} peer_wire_tx={} peer_wire_rx={}",
            tx.counters().wire_bytes,
            rx.counters().wire_bytes,
            server_tx.wire_bytes,
            server_rx.wire_bytes
        );
    });
}
