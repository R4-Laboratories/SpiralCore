use super::*;
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
    time::Duration,
};

const RECEIVER: &str = "cooperator.mvm.peer:443";

fn fixture() -> Vec<u8> {
    let mut bytes = b"{\"kind\":\"flowmux\",\"value\":1}\n".repeat(200);
    bytes.extend_from_slice(&[0, 255, 0, 0]);
    bytes
}

fn serve(
    response: &'static [u8],
    echo: impl FnOnce(&mut TcpStream) + Send + 'static,
) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    // The test stub can decline before connecting. Bound this fixture too.
    listener.set_nonblocking(true).unwrap();
    let peer = thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock && Instant::now() < until => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("fixture accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let expected = format!("CONNECT {RECEIVER} HTTP/1.1\r\nHost: {RECEIVER}\r\n\r\n");
        let mut head = vec![0; expected.len()];
        stream.read_exact(&mut head).unwrap();
        assert_eq!(head, expected.as_bytes());
        stream.write_all(response).unwrap();
        echo(&mut stream);
    });
    (addr, peer)
}

fn run(addr: SocketAddr, allow: bool, c: &mut Count) -> Result<()> {
    exchange(
        addr,
        RECEIVER,
        allow,
        Instant::now() + Duration::from_secs(3),
        c,
    )
}

#[test]
fn accepts_only_loopback_http_and_dns_authorities() {
    assert_eq!(
        proxy("http://127.0.0.1:1080").unwrap(),
        "127.0.0.1:1080".parse().unwrap()
    );
    for v in [
        "http://127.0.0.1:1080@evil",
        "http://user@127.0.0.1:1080",
        "http://127.0.0.1:0",
        "socks5h://127.0.0.1:1080",
        "http://example.test:80",
        "http://192.0.2.1:80",
        "http://127.0.0.1:1080/",
    ] {
        assert!(proxy(v).is_err());
    }
    target(RECEIVER).unwrap();
    for t in [
        "localhost:80",
        "127.0.0.1:80",
        "a.test:0",
        "a.test:65536",
        "a.test:80\r\nHost: other",
        "user@a.test:80",
        "a.test/path:80",
        "a..test:80",
        "-a.test:80",
        "a.test:080",
    ] {
        assert!(target(t).is_err(), "{t:?}");
    }
}

#[test]
fn genuine_403_writes_zero_application_bytes() {
    let (addr, peer) = serve(DENIED, |s| {
        s.shutdown(Shutdown::Write).unwrap();
        assert_eq!(s.read(&mut [0; 1]).unwrap(), 0);
    });
    let mut c = Count::default();
    run(addr, false, &mut c).unwrap();
    peer.join().unwrap();
    assert_eq!((c.body_tx, c.body_rx, c.head_rx), (0, 0, DENIED.len()));
}

#[test]
fn wrong_status_and_nonempty_denial_cannot_pass() {
    for head in [
        &b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"[..],
        GOOD,
        &b"HTTP/1.1 403 Forbidden\r\n\r\n"[..],
        DENIED,
    ] {
        let (addr, peer) = serve(head, move |s| {
            if head == DENIED {
                s.write_all(b"x").unwrap();
            }
            s.shutdown(Shutdown::Write).unwrap();
            match s.read(&mut [0; 1]) {
                Ok(0) => {}
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => {}
                other => panic!("unexpected client application data: {other:?}"),
            }
        });
        let mut c = Count::default();
        assert!(run(addr, false, &mut c).is_err());
        assert_eq!(c.body_tx, 0);
        peer.join().unwrap();
    }
}

#[test]
fn half_close_precedes_exact_binary_echo_and_clean_eof() {
    let (addr, peer) = serve(GOOD, |s| {
        let mut body = Vec::new();
        s.take(5805).read_to_end(&mut body).unwrap();
        assert_eq!(body, fixture());
        for part in body.chunks(511) {
            s.write_all(part).unwrap();
        }
        s.shutdown(Shutdown::Write).unwrap();
    });
    let mut c = Count::default();
    run(addr, true, &mut c).unwrap();
    peer.join().unwrap();
    assert_eq!((c.body_tx, c.body_rx, c.head_rx), (5804, 5804, GOOD.len()));
    assert_eq!(
        c.head_tx,
        format!("CONNECT {RECEIVER} HTTP/1.1\r\nHost: {RECEIVER}\r\n\r\n").len()
    );
}

#[test]
fn truncated_changed_and_extra_echo_are_refused() {
    for kind in 0..3 {
        let (addr, peer) = serve(GOOD, move |s| {
            let mut body = Vec::new();
            s.take(5805).read_to_end(&mut body).unwrap();
            assert_eq!(body, fixture());
            match kind {
                0 => {
                    body.pop();
                }
                1 => {
                    body[5803] = 1;
                }
                _ => body.push(0),
            }
            s.write_all(&body).unwrap();
            s.shutdown(Shutdown::Write).unwrap();
        });
        let mut c = Count::default();
        assert!(run(addr, true, &mut c).is_err());
        peer.join().unwrap();
        assert_eq!(c.body_tx, 5804);
        assert_eq!(c.body_rx, [5803, 5804, 5805][kind]);
    }
}

#[test]
fn only_registered_echo_peer_can_receive_the_fixture() {
    targets("denied.mvm.peer:443", RECEIVER).unwrap();
    for (denied, allowed) in [
        ("denied.mvm.peer:443", "other.mvm.peer:443"),
        ("cooperator.mvm.peer:443", RECEIVER),
        ("cooperator.mvm.peer:80", RECEIVER),
        ("outside.test:443", RECEIVER),
    ] {
        assert!(targets(denied, allowed).is_err());
    }
}

#[test]
fn an_expired_deadline_cannot_open_a_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut c = Count::default();
    assert!(
        exchange(
            listener.local_addr().unwrap(),
            RECEIVER,
            true,
            Instant::now(),
            &mut c
        )
        .is_err()
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!((c.head_tx, c.body_tx), (0, 0));
}

#[test]
fn a_refusal_on_the_allowed_turn_sends_no_body() {
    let (addr, peer) = serve(DENIED, |s| {
        let mut first = [0; 1];
        match s.read(&mut first) {
            Ok(0) => (),
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => (),
            x => panic!("unexpected upload: {x:?}"),
        }
    });
    let mut c = Count::default();
    assert!(run(addr, true, &mut c).is_err());
    assert_eq!(c.body_tx, 0);
    peer.join().unwrap();
}
