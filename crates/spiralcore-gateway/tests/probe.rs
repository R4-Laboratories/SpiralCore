use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[test]
fn paired_application_probes_verify_real_tcp_and_udp_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let fixture = directory.path().join("fixture");
    let bin = env!("CARGO_BIN_EXE_spiralcore-gateway-probe");
    assert!(Command::new(bin)
        .args(["fixture", fixture.to_str().unwrap(), "65536", "opaque", "9"])
        .status()
        .unwrap()
        .success());
    for protocol in ["tcp", "udp"] {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let addr = format!("127.0.0.1:{port}");
        let server_out = directory.path().join(format!("{protocol}-server.json"));
        let client_out = directory.path().join(format!("{protocol}-client.json"));
        let mut server = Command::new(bin)
            .args([
                &format!("{protocol}-server"),
                &addr,
                fixture.to_str().unwrap(),
                server_out.to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        let client = Command::new(bin)
            .args([
                &format!("{protocol}-client"),
                &addr,
                fixture.to_str().unwrap(),
                client_out.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        if !client.success() {
            let _ = server.kill();
            let _ = server.wait();
            panic!("{protocol} client failed");
        }
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = server.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() >= until {
                let _ = server.kill();
                let _ = server.wait();
                panic!("server did not finish");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let mut receipts = Vec::new();
        for path in [&client_out, &server_out] {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(value["exact"], true);
            assert_eq!(value["application_bytes"], 65536);
            assert!(value["wall_us"].as_u64().unwrap() > 0);
            receipts.push(value);
        }
        assert_eq!(receipts[0]["sha256"], receipts[1]["sha256"]);
    }
}

#[test]
fn sequence_defers_report_io_until_all_exchanges_finish() {
    let directory = tempfile::tempdir().unwrap();
    let fixture = directory.path().join("fixture");
    let bytes = vec![b'k'; 4096];
    std::fs::write(&fixture, &bytes).unwrap();
    let prefix = directory.path().join("server");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let addr = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_spiralcore-gateway-probe"))
        .args([
            "tcp-sequence-server",
            &addr,
            fixture.to_str().unwrap(),
            fixture.to_str().unwrap(),
            prefix.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let mut stream = TcpStream::connect(&addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut early_report = false;
    for index in 0..3 {
        stream
            .write_all(&(bytes.len() as u64).to_be_bytes())
            .unwrap();
        stream.write_all(&bytes).unwrap();
        let mut receipt = [0; 32];
        stream.read_exact(&mut receipt).unwrap();
        if index < 2 {
            // Hold the next exchange to expose any report work in between.
            std::thread::sleep(Duration::from_millis(50));
            early_report |= directory.path().join("server-pattern-before.json").exists();
        }
    }
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = server.try_wait().unwrap() {
            assert!(result.success());
            break;
        }
        if Instant::now() >= until {
            let _ = server.kill();
            let _ = server.wait();
            panic!("sequence report server timeout");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(!early_report, "report I/O ran between timed exchanges");
    for item in ["pattern-before", "opaque", "pattern-after"] {
        assert!(directory
            .path()
            .join(format!("server-{item}.json"))
            .exists());
    }
}

#[test]
fn three_exact_transfers_share_one_tcp_connection() {
    let directory = tempfile::tempdir().unwrap();
    let pattern = directory.path().join("pattern");
    let opaque = directory.path().join("opaque");
    std::fs::write(&pattern, vec![b'x'; 65536]).unwrap();
    std::fs::write(
        &opaque,
        spiralcore_gateway::measurement::fixture(65536, "opaque", 17).unwrap(),
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_spiralcore-gateway-probe");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let addr = format!("127.0.0.1:{port}");
    let server_prefix = directory.path().join("server");
    let client_prefix = directory.path().join("client");
    let mut server = Command::new(bin)
        .args([
            "tcp-sequence-server",
            &addr,
            pattern.to_str().unwrap(),
            opaque.to_str().unwrap(),
            server_prefix.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let client = Command::new(bin)
        .args([
            "tcp-sequence-client",
            &addr,
            pattern.to_str().unwrap(),
            opaque.to_str().unwrap(),
            client_prefix.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    if !client.success() {
        let _ = server.kill();
        let _ = server.wait();
        panic!("sequence client failed");
    }
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = server.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() >= end {
            let _ = server.kill();
            let _ = server.wait();
            panic!("sequence server timeout");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let mut connection = None;
    for (index, item) in ["pattern-before", "opaque", "pattern-after"]
        .iter()
        .enumerate()
    {
        let read = |prefix: &std::path::Path| -> serde_json::Value {
            serde_json::from_slice(
                &std::fs::read(format!("{}-{item}.json", prefix.display())).unwrap(),
            )
            .unwrap()
        };
        let a = read(&client_prefix);
        let b = read(&server_prefix);
        assert_eq!(a["exact"], true);
        assert_eq!(b["exact"], true);
        assert_eq!(a["application_bytes"], 65536);
        assert_eq!(a["sha256"], b["sha256"]);
        assert_eq!(a["detail"]["sequence_index"], index);
        assert_eq!(a["detail"]["same_connection"], true);
        assert_eq!(a["detail"]["connect_included"], index == 0);
        let endpoints = (
            a["detail"]["local_address"].clone(),
            a["detail"]["peer_address"].clone(),
        );
        if let Some(previous) = &connection {
            assert_eq!(&endpoints, previous);
        } else {
            connection = Some(endpoints);
        }
        assert_eq!(a["detail"]["local_address"], b["detail"]["peer_address"]);
    }
}
