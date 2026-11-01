use std::{
    net::TcpListener,
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
