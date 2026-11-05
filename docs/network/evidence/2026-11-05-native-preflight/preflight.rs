use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let target = 34567;
    let header = format!("CONNECT 127.0.0.1:{target} HTTP/1.1\r\nHost: 127.0.0.1:{target}\r\n\r\n");
    let banner = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    let payload = b"{\"kind\":\"flowmux\",\"value\":1}\n".repeat(400);
    let expected = payload.clone();
    let start = Instant::now();
    let original = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(12))).unwrap();
        socket.set_write_timeout(Some(Duration::from_secs(12))).unwrap();
        let mut head = vec![0; header.len()];
        socket.read_exact(&mut head).unwrap();
        assert_eq!(head, header.as_bytes());
        socket.write_all(banner).unwrap();
        let mut actual = Vec::new();
        (&mut socket)
            .take(u64::try_from(expected.len() + 1).unwrap())
            .read_to_end(&mut actual)
            .expect("guest command must half-close its input after the payload");
        assert_eq!(actual, expected);
        socket.write_all(&actual).unwrap();
        socket.shutdown(Shutdown::Write).unwrap();
    });
    let result = Command::new("./busybox")
        .args(["sh", "guest-transfer.sh", &target.to_string(), &port.to_string(), "./busybox"])
        .output()
        .unwrap();
    original.join().unwrap();
    assert!(result.status.success(), "guest command exit: {}", result.status);
    assert_eq!(result.stdout.strip_prefix(banner).unwrap(), payload);
    assert!(result.stderr.is_empty());
    println!(
        "loopback command preflight: {} bytes each direction, exact equality, input half-close, elapsed_ms={}",
        payload.len(),
        start.elapsed().as_millis()
    );
}
