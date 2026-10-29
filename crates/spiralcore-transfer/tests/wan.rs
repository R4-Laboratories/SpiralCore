use spiralcore_transfer::{
    endpoint::{Endpoint, Limits},
    hex, object_id,
    protocol::{Frame, Mode, Offer},
    wan::{self, BenchAuthority, HttpClient},
    Error,
};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "spiral-wan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn server(t: &Temp, requests: usize) -> (String, thread::JoinHandle<()>) {
    let endpoint = Endpoint::open(&t.0.join("cache"), Limits::default(), BenchAuthority).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        wan::serve(
            listener,
            endpoint,
            TOKEN,
            &BTreeSet::from([[1; 32]]),
            Duration::from_secs(10),
            requests,
        )
        .unwrap()
    });
    (url, task)
}
fn frames(bytes: &[u8]) -> Vec<Frame> {
    let mut body = vec![0];
    body.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    body.extend_from_slice(bytes);
    vec![
        Frame {
            kind: 1,
            transfer: [2; 32],
            seq: 0,
            body: Offer::cold(bytes, Mode::Raw).encode().unwrap(),
        },
        Frame {
            kind: 2,
            transfer: [2; 32],
            seq: 1,
            body,
        },
        Frame {
            kind: 3,
            transfer: [2; 32],
            seq: 2,
            body: vec![],
        },
    ]
}
#[test]
fn batch_bounds_and_canonical_framing() {
    let fs = frames(b"exact\0bytes\0");
    let body = wan::encode_batch(&fs).unwrap();
    let recovered = wan::decode_batch(&body).unwrap();
    assert_eq!(recovered.len(), 3);
    for (a, b) in fs.iter().zip(recovered) {
        assert_eq!(a.encode().unwrap(), b.encode().unwrap());
    }
    assert!(wan::decode_batch(&body[..body.len() - 1]).is_err());
    let mut extra = body.clone();
    extra.push(0);
    assert!(wan::decode_batch(&extra).is_err());
    assert!(wan::encode_batch(&[]).is_err());
    assert!(wan::encode_batch(&vec![fs[0].clone(); 513]).is_err());
}
#[test]
fn authenticated_batch_commits_exact_bytes_with_measured_receiver_work() {
    let t = Temp::new();
    let (url, task) = server(&t, 1);
    let bytes = b"binary\0with trailing zeros\0\0";
    let request = frames(bytes);
    let expected_request = wan::encode_batch(&request).unwrap().len();
    let mut client = HttpClient::new(&url, TOKEN, [1; 32]).unwrap();
    let replies = client.exchange(&request).unwrap();
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[2].kind, 103);
    assert_eq!(replies[2].body, object_id(bytes));
    let metrics = client.metrics();
    assert_eq!(metrics.http_requests, 1);
    assert_eq!(metrics.request_body_bytes, expected_request as u64);
    assert!(
        metrics.response_body_bytes
            > replies
                .iter()
                .map(|f| f.encode().unwrap().len() as u64)
                .sum::<u64>()
    );
    assert!(metrics.receiver_wall_micros > 0);
    task.join().unwrap();
    let mut endpoint =
        Endpoint::open(&t.0.join("cache"), Limits::default(), BenchAuthority).unwrap();
    assert_eq!(
        endpoint
            .read_object(&wan::context([1; 32]), object_id(bytes))
            .unwrap(),
        bytes
    );
}
#[test]
fn authentication_and_scope_fail_before_reading_body() {
    for (auth, scope) in [
        ("wrong".to_string(), hex(&[1; 32])),
        (TOKEN.to_string(), hex(&[9; 32])),
    ] {
        let t = Temp::new();
        let (url, task) = server(&t, 1);
        let mut socket = TcpStream::connect(url.trim_start_matches("http://")).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        write!(socket,"POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {auth}\r\nX-Spiral-Scope: {scope}\r\nContent-Type: application/octet-stream\r\nContent-Length: 10000\r\n\r\n").unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403"), "{response}");
        task.join().unwrap();
    }
}
#[test]
fn hostile_http_headers_and_truncation_are_rejected() {
    for extra in [
        "Content-Length: 99999999\r\n",
        "Content-Length: 10\r\nTransfer-Encoding: chunked\r\n",
        "Content-Length: 10\r\nContent-Length: 10\r\n",
        "Content-Length: 10\r\nContent-Encoding: gzip\r\n",
    ] {
        let t = Temp::new();
        let (url, task) = server(&t, 1);
        let mut socket = TcpStream::connect(url.trim_start_matches("http://")).unwrap();
        write!(socket,"POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {TOKEN}\r\nX-Spiral-Scope: {}\r\nContent-Type: application/octet-stream\r\n{extra}\r\n",hex(&[1;32])).unwrap();
        socket.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with("HTTP/1.1 400") || response.starts_with("HTTP/1.1 413"),
            "{response}"
        );
        task.join().unwrap();
    }
}
#[test]
fn client_rejects_unsafe_urls_and_short_capabilities() {
    for url in [
        "http://example.com",
        "http://0.0.0.0:1234",
        "https://user:pass@example.com",
        "https://example.com/?x=1",
        "https://example.com/#fragment",
    ] {
        assert!(HttpClient::new(url, TOKEN, [1; 32]).is_err(), "{url}");
    }
    assert!(matches!(
        HttpClient::new("https://example.com", "short", [1; 32]),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn an_idle_completed_connection_cannot_block_another_client() {
    let t = Temp::new();
    let (url, task) = server(&t, 2);
    let mut first = TcpStream::connect(url.trim_start_matches("http://")).unwrap();
    let body = wan::encode_batch(&frames(b"first")).unwrap();
    write!(first, "POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {TOKEN}\r\nX-Spiral-Scope: {}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",hex(&[1;32]),body.len()).unwrap();
    first.write_all(&body).unwrap();
    first
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut reply = [0; 1024];
    assert!(first.read(&mut reply).unwrap() > 0);
    // Keep the first descriptor alive while another connection is admitted.
    let mut second = TcpStream::connect(url.trim_start_matches("http://")).unwrap();
    second
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    write!(second,"POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer wrong\r\nContent-Length: 10000\r\n\r\n").unwrap();
    let count = second
        .read(&mut reply)
        .expect("idle prior client must not monopolize origin");
    assert!(String::from_utf8_lossy(&reply[..count]).starts_with("HTTP/1.1 403"));
    drop(first);
    task.join().unwrap();
}

#[test]
fn client_rejects_a_truncated_nonterminal_reply_prefix() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = frames(b"abc");
    let ready = requests[0].reply(101, vec![0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
    let mut body = b"SCWREP01".to_vec();
    body.extend_from_slice(&[0; 16]);
    body.extend_from_slice(&1u32.to_le_bytes());
    let frame = ready.encode().unwrap();
    body.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    body.extend_from_slice(&frame);
    let task = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = vec![0; 4096];
        let _ = socket.read(&mut request).unwrap();
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        socket.write_all(&body).unwrap();
    });
    let result = HttpClient::new(&url, TOKEN, [1; 32])
        .unwrap()
        .exchange(&requests);
    task.join().unwrap();
    assert!(
        result.is_err(),
        "a Ready-only response silently dropped uploaded data"
    );
}

#[test]
fn campaign_origin_refuses_to_reuse_a_populated_cache() {
    let t = Temp::new();
    let cache = t.0.join("cache");
    let mut endpoint = Endpoint::open(&cache, wan::limits(), BenchAuthority).unwrap();
    for f in frames(b"preserve") {
        endpoint.handle(&wan::context([1; 32]), &f, 0).unwrap();
    }
    drop(endpoint);
    let plan = t.0.join("plan.json");
    fs::write(
        &plan,
        serde_json::to_vec(&serde_json::json!({"allowed_scopes":[hex(&[1;32])]})).unwrap(),
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
        .args(["serve", "--cache"])
        .arg(&cache)
        .arg("--plan")
        .arg(&plan)
        .args(["--seconds", "1"])
        .env("SPIRALCORE_WAN_TOKEN", TOKEN)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("fresh cache"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut endpoint = Endpoint::open(&cache, wan::limits(), BenchAuthority).unwrap();
    assert_eq!(
        endpoint
            .read_object(&wan::context([1; 32]), object_id(b"preserve"))
            .unwrap(),
        b"preserve"
    );
}
