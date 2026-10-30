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
    io::{BufRead, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
#[test]
fn campaign_origin_rejects_invalid_codec_cache_configuration_before_creating_cache() {
    let t = Temp::new();
    for (index, value) in [
        serde_json::Value::Null,
        serde_json::json!("true"),
        serde_json::json!(1),
        serde_json::json!({}),
    ]
    .into_iter()
    .enumerate()
    {
        let plan = serde_json::json!({"schema":"spiralcore.wan.plan.v4",
            "cold_graph_warming":false,"allowed_scopes":[hex(&[1;32])],
            "codec_cache_enabled":value});
        let path = t.0.join(format!("codec-plan-{index}.json"));
        let cache = t.0.join(format!("codec-cache-{index}"));
        fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
            .args(["serve", "--cache"])
            .arg(&cache)
            .arg("--plan")
            .arg(&path)
            .args(["--seconds", "1"])
            .env("SPIRALCORE_WAN_TOKEN", TOKEN)
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "invalid codec cache setting was accepted"
        );
        assert!(String::from_utf8_lossy(&out.stderr).contains("codec cache"));
        assert!(!cache.exists());
    }
}
#[test]
fn campaign_origin_rejects_ambiguous_warming_policy_before_creating_cache() {
    let t = Temp::new();
    for (i, plan) in [
        serde_json::json!({"schema":"spiralcore.wan.plan.v4","allowed_scopes":[hex(&[1;32])]}),
        serde_json::json!({"schema":"spiralcore.wan.plan.v4","cold_graph_warming":true,"allowed_scopes":[hex(&[1;32])]}),
        serde_json::json!({"schema":"spiralcore.wan.plan.v3","cold_graph_warming":false,"allowed_scopes":[hex(&[1;32])]}),
    ].into_iter().enumerate() {
        let path = t.0.join(format!("plan-{i}.json"));
        let cache = t.0.join(format!("cache-{i}"));
        fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
            .args(["serve", "--cache"]).arg(&cache).arg("--plan").arg(&path)
            .args(["--seconds", "1"]).env("SPIRALCORE_WAN_TOKEN", TOKEN).output().unwrap();
        assert!(!out.status.success(), "unsupported policy was accepted");
        assert!(String::from_utf8_lossy(&out.stderr).contains("warming policy"));
        assert!(!cache.exists());
    }
}
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

#[test]
fn codec_cache_cli_is_opt_in_and_records_persistent_hits_without_credentials() {
    use spiralcore_transfer::{client, schema, schema_zstd};
    for enabled in [None, Some(false), Some(true)] {
        let t = Temp::new();
        let mut plan = serde_json::json!({"schema":"spiralcore.wan.plan.v4",
            "cold_graph_warming":false,"allowed_scopes":[hex(&[1;32])]});
        if let Some(value) = enabled {
            plan["codec_cache_enabled"] = serde_json::json!(value);
        }
        let path = t.0.join("plan.json");
        let cache = t.0.join("cache");
        fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
            .args(["serve", "--cache"])
            .arg(&cache)
            .arg("--plan")
            .arg(&path)
            .args(["--seconds", "2"])
            .env("SPIRALCORE_WAN_TOKEN", TOKEN)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut ready = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let addr = ready.trim().strip_prefix("READY addr=").unwrap();
        let mut http = HttpClient::new(&format!("http://{addr}"), TOKEN, [1; 32]).unwrap();
        let training = b"{\"long_identifier\":0,\"status_message\":\"training-value\",\"enabled_flag\":true}\n";
        let artifact = schema::fit(&[training]).unwrap();
        client::transfer_batched(&artifact, Mode::Raw, None, [11; 32], false, |f| {
            http.exchange(f)
        })
        .unwrap();
        let mut prepared = schema_zstd::PreparedEncoder::new(&artifact).unwrap();
        let mut targets = Vec::new();
        for i in 0..2 {
            let bytes = format!("{{\"long_identifier\":{i},\"status_message\":\"held-out-{i}\",\"enabled_flag\":false}}\n").repeat(128).into_bytes();
            let stats = client::transfer_prepared_schema_zstd_batched(
                &mut prepared,
                &bytes,
                [12 + i; 32],
                false,
                |f| http.exchange(f),
            )
            .unwrap();
            assert_eq!(stats.mode, Mode::SchemaZstdV1);
            targets.push(bytes);
        }
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{:?}", output);
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains(TOKEN));
        let rows: Vec<serde_json::Value> = stderr
            .lines()
            .filter(|s| s.starts_with('{'))
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        if enabled == Some(true) {
            assert_eq!(rows.len(), 3);
            for (index, row) in rows.iter().enumerate() {
                assert_eq!(row["event"], "wan_request");
                assert_eq!(row["request_ordinal"], index + 1);
                assert_eq!(row["scope"], hex(&[1; 32]));
                assert_eq!(row["http_status"], 200);
            }
            assert_eq!(rows[0]["codec_cache"]["preparations"], 0);
            assert_eq!(rows[1]["codec_cache"]["preparations"], 1);
            assert_eq!(rows[2]["codec_cache"]["preparations"], 1);
            assert_eq!(rows[2]["codec_cache"]["hits"], 1);
        } else {
            assert!(rows.is_empty());
        }
        let mut reopened = Endpoint::open(&cache, Limits::default(), BenchAuthority).unwrap();
        for bytes in std::iter::once(&artifact).chain(targets.iter()) {
            assert_eq!(
                reopened
                    .read_object(&wan::context([1; 32]), object_id(bytes))
                    .unwrap(),
                *bytes
            );
        }
    }
}

#[test]
fn observed_origin_rejects_failed_logging_and_does_not_observe_unauthorized_requests() {
    for authorized in [false, true] {
        let t = Temp::new();
        let endpoint =
            Endpoint::open(&t.0.join("cache"), Limits::default(), BenchAuthority).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = thread::spawn(move || {
            let mut observed = 0;
            let result = wan::serve_observed(
                listener,
                endpoint,
                TOKEN,
                &BTreeSet::from([[1; 32]]),
                Duration::from_secs(10),
                1,
                |observation| {
                    observed += 1;
                    assert_eq!(observation.scope, [1; 32]);
                    assert_eq!(observation.request_ordinal, 1);
                    assert_eq!(observation.http_status, 200);
                    Err(Error::Storage)
                },
            );
            (result, observed)
        });
        let mut http =
            HttpClient::new(&url, TOKEN, if authorized { [1; 32] } else { [9; 32] }).unwrap();
        assert!(http.exchange(&frames(b"observed payload")).is_err());
        let (result, observed) = task.join().unwrap();
        assert_eq!(observed, usize::from(authorized));
        if authorized {
            assert!(matches!(result, Err(Error::Storage)));
        } else {
            assert!(result.is_ok());
        }
    }
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
