use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Endpoint},
    hex, object_id,
    protocol::{Frame, Mode, Offer},
    wan::{self, BenchAuthority, ConnectionPolicy, ServeConfig},
};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(PathBuf);
impl Store {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "spiral-persistent-http-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
type OriginTask = thread::JoinHandle<Vec<(usize, usize, usize)>>;
fn origin(
    store: &Store,
    total: usize,
    per_connection: usize,
    idle_ms: u64,
) -> (String, OriginTask) {
    let endpoint = Endpoint::open_with_policy(
        &store.0.join("cache"),
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let task = thread::spawn(move || {
        let mut seen = Vec::new();
        wan::serve_observed_configured(
            listener,
            endpoint,
            TOKEN,
            &BTreeSet::from([[1; 32]]),
            ServeConfig {
                duration: Duration::from_secs(5),
                max_requests: total,
                connection_policy: ConnectionPolicy::KeepAlive {
                    max_requests_per_connection: per_connection,
                    idle_timeout: Duration::from_millis(idle_ms),
                },
            },
            |r| {
                seen.push((
                    r.connection_ordinal,
                    r.request_on_connection,
                    r.request_ordinal,
                ));
                Ok(())
            },
        )
        .unwrap();
        seen
    });
    (address, task)
}
fn socket(address: &str) -> BufReader<TcpStream> {
    let s = TcpStream::connect(address).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    BufReader::new(s)
}
fn batch(index: u8) -> Vec<u8> {
    let bytes = [index, 0, 255, 0];
    let mut body = vec![0];
    body.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    body.extend_from_slice(&bytes);
    wan::encode_batch(&[
        Frame {
            kind: 1,
            transfer: [index; 32],
            seq: 0,
            body: Offer::cold(&bytes, Mode::Raw).encode().unwrap(),
        },
        Frame {
            kind: 2,
            transfer: [index; 32],
            seq: 1,
            body,
        },
        Frame {
            kind: 3,
            transfer: [index; 32],
            seq: 2,
            body: Vec::new(),
        },
    ])
    .unwrap()
}
fn write_request(reader: &mut BufReader<TcpStream>, index: u8, extra: &str) {
    let body = batch(index);
    write!(reader.get_mut(), "POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {TOKEN}\r\nX-Spiral-Scope: {}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n{extra}\r\n", hex(&[1;32]), body.len()).unwrap();
    reader.get_mut().write_all(&body).unwrap();
}
fn reply(reader: &mut BufReader<TcpStream>) -> (String, Vec<u8>) {
    let mut header = String::new();
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0, "missing reply");
        header.push_str(&line);
        if line == "\r\n" {
            break;
        }
        assert!(header.len() < 4096);
    }
    let len: usize = header
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(len <= wan::MAX_BATCH);
    let mut body = vec![0; len];
    reader.read_exact(&mut body).unwrap();
    (header, body)
}

#[test]
fn legacy_zero_duration_returns_ok_without_accepting_or_processing_requests() {
    for observed in [false, true] {
        let store = Store::new();
        let endpoint = Endpoint::open_with_policy(
            &store.0.join("cache"),
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backlog = listener.try_clone().unwrap();
        let mut client = socket(&listener.local_addr().unwrap().to_string());
        write_request(&mut client, 1, "");
        let mut observations = 0;
        let result = if observed {
            wan::serve_observed(
                listener,
                endpoint,
                TOKEN,
                &BTreeSet::from([[1; 32]]),
                Duration::ZERO,
                1,
                |_| {
                    observations += 1;
                    Ok(())
                },
            )
        } else {
            wan::serve(
                listener,
                endpoint,
                TOKEN,
                &BTreeSet::from([[1; 32]]),
                Duration::ZERO,
                1,
            )
        };
        assert_eq!(result, Ok(()));
        assert_eq!(observations, 0);
        backlog.set_nonblocking(true).unwrap();
        assert!(
            backlog.accept().is_ok(),
            "the queued connection must remain unaccepted"
        );
        let mut reopened = Endpoint::open_with_policy(
            &store.0.join("cache"),
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert!(reopened
            .read_object(&wan::context([1; 32]), object_id(&[1, 0, 255, 0]))
            .is_err());
    }
}

#[test]
fn eleven_transfers_reuse_one_connection_and_reopen_exactly() {
    let store = Store::new();
    let (address, task) = origin(&store, 11, 32, 500);
    let mut stream = socket(&address);
    for i in 1..=11 {
        write_request(&mut stream, i, "");
        let (header, body) = reply(&mut stream);
        assert!(header.starts_with("HTTP/1.1 200"));
        assert!(body.starts_with(b"SCWREP01"));
        assert!(header.contains(if i == 11 {
            "Connection: close"
        } else {
            "Connection: keep-alive"
        }));
    }
    assert_eq!(stream.read(&mut [0]).unwrap(), 0);
    assert_eq!(
        task.join().unwrap(),
        (1..=11).map(|i| (1, i, i)).collect::<Vec<_>>()
    );
    let mut ep = Endpoint::open_with_policy(
        &store.0.join("cache"),
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    for i in 1..=11 {
        let bytes = [i, 0, 255, 0];
        assert_eq!(
            ep.read_object(&wan::context([1; 32]), object_id(&bytes))
                .unwrap(),
            bytes
        );
    }
}

#[test]
fn connection_cap_and_client_close_are_honored() {
    for explicit_close in [false, true] {
        let store = Store::new();
        let (address, task) = origin(&store, 3, 2, 500);
        let mut first = socket(&address);
        write_request(
            &mut first,
            1,
            if explicit_close {
                "Connection: keep-alive, close\r\n"
            } else {
                ""
            },
        );
        let (header, _) = reply(&mut first);
        if explicit_close {
            assert!(header.contains("Connection: close"));
            assert_eq!(first.read(&mut [0]).unwrap(), 0);
            first = socket(&address);
        }
        write_request(&mut first, 2, "");
        let (header, _) = reply(&mut first);
        assert!(header.contains(if explicit_close {
            "Connection: keep-alive"
        } else {
            "Connection: close"
        }));
        if !explicit_close {
            assert_eq!(first.read(&mut [0]).unwrap(), 0);
            first = socket(&address);
        }
        write_request(&mut first, 3, "");
        assert!(reply(&mut first).0.contains("Connection: close"));
        let seen = task.join().unwrap();
        assert_eq!(
            seen,
            if explicit_close {
                vec![(1, 1, 1), (2, 1, 2), (2, 2, 3)]
            } else {
                vec![(1, 1, 1), (1, 2, 2), (2, 1, 3)]
            }
        );
    }
}

#[test]
fn reused_connection_rechecks_auth_and_scope_before_body() {
    for foreign_scope in [false, true] {
        let store = Store::new();
        let (address, task) = origin(&store, 2, 32, 500);
        let mut stream = socket(&address);
        write_request(&mut stream, 1, "");
        assert!(reply(&mut stream).0.starts_with("HTTP/1.1 200"));
        write!(stream.get_mut(),"POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {}\r\nX-Spiral-Scope: {}\r\nContent-Type: application/octet-stream\r\nContent-Length: 10000\r\n\r\n",if foreign_scope {TOKEN}else{"wrong"},hex(&[if foreign_scope {2}else{1};32])).unwrap();
        let (header, body) = reply(&mut stream);
        assert!(header.starts_with("HTTP/1.1 403"));
        assert!(body.is_empty());
        assert_eq!(stream.read(&mut [0]).unwrap(), 0);
        assert_eq!(task.join().unwrap(), vec![(1, 1, 1)]);
    }
}

#[test]
fn idle_connection_releases_serial_origin_without_consuming_a_request() {
    let store = Store::new();
    let (address, task) = origin(&store, 2, 32, 50);
    let mut first = socket(&address);
    write_request(&mut first, 1, "");
    assert!(reply(&mut first).0.contains("Connection: keep-alive"));
    let mut second = socket(&address);
    write_request(&mut second, 2, "");
    assert!(reply(&mut second).0.starts_with("HTTP/1.1 200"));
    assert_eq!(first.read(&mut [0]).unwrap(), 0);
    assert_eq!(task.join().unwrap(), vec![(1, 1, 1), (2, 1, 2)]);
}

#[test]
fn malformed_or_truncated_second_request_closes_connection() {
    for truncated in [false, true] {
        let store = Store::new();
        let (address, task) = origin(&store, 2, 32, 500);
        let mut stream = socket(&address);
        write_request(&mut stream, 1, "");
        let _ = reply(&mut stream);
        write!(stream.get_mut(),"POST /v1/batch HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer {TOKEN}\r\nX-Spiral-Scope: {}\r\nContent-Type: application/octet-stream\r\nContent-Length: 100\r\n{}\r\n",hex(&[1;32]),if truncated {""}else{"Transfer-Encoding: chunked\r\n"}).unwrap();
        stream.get_mut().shutdown(Shutdown::Write).unwrap();
        assert!(reply(&mut stream).0.starts_with("HTTP/1.1 400"));
        assert_eq!(stream.read(&mut [0]).unwrap(), 0);
        assert_eq!(task.join().unwrap(), vec![(1, 1, 1)]);
    }
}

#[test]
fn invalid_keep_alive_plan_is_rejected_before_cache_creation() {
    let store = Store::new();
    for (i, keep) in [
        serde_json::Value::Null,
        serde_json::json!(true),
        serde_json::json!({"max_requests_per_connection":0,"idle_timeout_ms":500}),
        serde_json::json!({"max_requests_per_connection":65,"idle_timeout_ms":500}),
        serde_json::json!({"max_requests_per_connection":32,"idle_timeout_ms":0}),
        serde_json::json!({"max_requests_per_connection":32,"idle_timeout_ms":1001}),
        serde_json::json!({"max_requests_per_connection":32,"idle_timeout_ms":500,"unknown":true}),
    ]
    .into_iter()
    .enumerate()
    {
        let plan = store.0.join(format!("plan-{i}.json"));
        let cache = store.0.join(format!("cache-{i}"));
        fs::write(&plan,serde_json::to_vec(&serde_json::json!({"schema":"spiralcore.wan.plan.v4","cold_graph_warming":false,"codec_cache_enabled":true,"allowed_scopes":[hex(&[1;32])],"http_keep_alive":keep})).unwrap()).unwrap();
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
        assert!(String::from_utf8_lossy(&out.stderr).contains("keep-alive"));
        assert!(!cache.exists());
    }
}

fn drain_diagnostic_request(request: &mut BufReader<TcpStream>) {
    let mut line = String::new();
    let mut length = None;
    loop {
        line.clear();
        assert!(request.read_line(&mut line).unwrap() > 0);
        if line == "\r\n" {
            break;
        }
        if let Some((_, value)) = line
            .split_once(':')
            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        {
            length = Some(value.trim().parse::<usize>().unwrap());
        }
    }
    let length = length.unwrap();
    assert!(length < 4096);
    request.read_exact(&mut vec![0; length]).unwrap();
}

#[test]
fn client_transport_failure_keeps_only_sanitized_status_and_resets_per_exchange() {
    for status in [400, 502] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut request = BufReader::new(stream);
            drain_diagnostic_request(&mut request);
            write!(request.get_mut(), "HTTP/1.1 {status} Rejected\r\nContent-Length: 0\r\nX-Untrusted: private-server-text\r\nConnection: close\r\n\r\n").unwrap();
            drop(request);
            let (stream, _) = listener.accept().unwrap();
            let mut request = BufReader::new(stream);
            drain_diagnostic_request(&mut request);
            let frames = wan::decode_batch(&batch(1)).unwrap();
            let reply = frames[0].reply(103, object_id(&[1, 0, 255, 0]).to_vec());
            let encoded = wan::encode_batch(&[reply]).unwrap();
            let mut body = b"SCWREP01".to_vec();
            body.extend_from_slice(&[0; 16]);
            body.extend_from_slice(&encoded[8..]);
            write!(
                request.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            request.get_mut().write_all(&body).unwrap();
        });
        let mut client = wan::HttpClient::new(&url, TOKEN, [1; 32]).unwrap();
        let frames = wan::decode_batch(&batch(1)).unwrap();
        assert!(matches!(
            client.exchange(&frames),
            Err(spiralcore_transfer::Error::Storage)
        ));
        let failure = client.last_transport_failure().unwrap();
        assert_eq!(failure.stage, "send");
        assert_eq!(failure.category, "http_status");
        assert_eq!(failure.http_status, Some(status));
        assert_eq!(failure.io_kind, None);
        assert_eq!(failure.timeout_phase, None);
        let safe = serde_json::to_string(&failure).unwrap();
        for secret in [TOKEN, "private-server-text", url.as_str()] {
            assert!(!safe.contains(secret));
        }
        assert!(client.exchange(&frames).is_ok());
        assert!(client.last_transport_failure().is_none());
        assert!(client.exchange(&[]).is_err());
        assert!(client.last_transport_failure().is_none());
        server.join().unwrap();
    }
}

#[test]
fn client_transport_failure_distinguishes_a_truncated_response_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut request = BufReader::new(stream);
        drain_diagnostic_request(&mut request);
        request
            .get_mut()
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nx")
            .unwrap();
        request.get_mut().shutdown(Shutdown::Write).unwrap();
    });
    let mut client = wan::HttpClient::new(&url, TOKEN, [1; 32]).unwrap();
    assert!(matches!(
        client.exchange(&wan::decode_batch(&batch(1)).unwrap()),
        Err(spiralcore_transfer::Error::Malformed)
    ));
    let failure = client.last_transport_failure().unwrap();
    assert_eq!(failure.stage, "response_body");
    assert!(matches!(failure.category, "io" | "protocol"));
    assert_eq!(failure.http_status, None);
    assert!(!serde_json::to_string(&failure).unwrap().contains(TOKEN));
    server.join().unwrap();
}

type DiagnosticTask = thread::JoinHandle<(Vec<usize>, Vec<wan::ConnectionObservation>)>;
fn diagnostic_origin(store: &Store) -> (String, DiagnosticTask) {
    let endpoint = Endpoint::open_with_policy(
        &store.0.join("cache"),
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let task = thread::spawn(move || {
        let mut admitted = Vec::new();
        let mut diagnostics = Vec::new();
        wan::serve_diagnosed_configured(
            listener,
            endpoint,
            TOKEN,
            &BTreeSet::from([[1; 32]]),
            ServeConfig {
                duration: Duration::from_secs(5),
                max_requests: 2,
                connection_policy: ConnectionPolicy::KeepAlive {
                    max_requests_per_connection: 32,
                    idle_timeout: Duration::from_millis(50),
                },
            },
            |r| {
                admitted.push(r.request_ordinal);
                Ok(())
            },
            |d| {
                diagnostics.push(d);
                Ok(())
            },
        )
        .unwrap();
        (admitted, diagnostics)
    });
    (address, task)
}

#[test]
fn connection_diagnostics_distinguish_empty_idle_from_rejected_auth_without_leaks() {
    let store = Store::new();
    let (address, task) = diagnostic_origin(&store);
    let mut first = socket(&address);
    write_request(&mut first, 1, "");
    assert!(reply(&mut first).0.starts_with("HTTP/1.1 200"));
    assert_eq!(first.read(&mut [0]).unwrap(), 0);
    let mut second = socket(&address);
    write!(second.get_mut(), "POST /v1/batch HTTP/1.1\r\nAuthorization: Bearer secret-sentinel\r\nX-Spiral-Scope: {}\r\nContent-Length: 100\r\n\r\n", hex(&[1; 32])).unwrap();
    assert!(reply(&mut second).0.starts_with("HTTP/1.1 403"));
    let (admitted, diagnostics) = task.join().unwrap();
    assert_eq!(admitted, vec![1]);
    assert_eq!(diagnostics.len(), 2);
    let idle = diagnostics[0];
    assert_eq!(idle.reason, "idle_timeout");
    assert_eq!(idle.header_bytes, 0);
    assert!(!idle.request_counted);
    assert_eq!(
        (
            idle.connection_ordinal,
            idle.request_on_connection,
            idle.request_ordinal
        ),
        (1, 2, 2)
    );
    let rejected = diagnostics[1];
    assert_eq!(rejected.reason, "request_rejected");
    assert_eq!(rejected.http_status, Some(403));
    assert!(rejected.request_counted);
    assert!(rejected.header_bytes > 0);
    assert_eq!(
        (
            rejected.connection_ordinal,
            rejected.request_on_connection,
            rejected.request_ordinal
        ),
        (2, 1, 2)
    );
    let safe = serde_json::to_string(&diagnostics).unwrap();
    for secret in [TOKEN, "secret-sentinel", hex(&[1; 32]).as_str()] {
        assert!(!safe.contains(secret));
    }
}

#[test]
fn connection_diagnostics_preserve_partial_header_timeout_and_eof_rejections() {
    for truncated in [false, true] {
        let store = Store::new();
        let (address, task) = diagnostic_origin(&store);
        let mut stream = socket(&address);
        write_request(&mut stream, 1, "");
        assert!(reply(&mut stream).0.starts_with("HTTP/1.1 200"));
        stream.get_mut().write_all(b"POST /v1").unwrap();
        if truncated {
            stream.get_mut().shutdown(Shutdown::Write).unwrap();
        }
        assert!(reply(&mut stream).0.starts_with("HTTP/1.1 400"));
        let (admitted, diagnostics) = task.join().unwrap();
        assert_eq!(admitted, vec![1]);
        assert_eq!(diagnostics.len(), 1);
        let d = diagnostics[0];
        assert_eq!(
            d.reason,
            if truncated {
                "header_truncated"
            } else {
                "header_timeout"
            }
        );
        assert_eq!(d.header_bytes, 8);
        assert_eq!(d.http_status, Some(400));
        assert!(d.request_counted);
        assert_eq!(
            (
                d.connection_ordinal,
                d.request_on_connection,
                d.request_ordinal
            ),
            (1, 2, 2)
        );
        assert!(d.elapsed_micros > 0);
    }
}
