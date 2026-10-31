use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
};

#[test]
fn connection_log_is_exclusive_and_contains_only_structured_transport_metadata() {
    let root =
        std::env::temp_dir().join(format!("spiral-wan-diagnostic-cli-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let plan = root.join("plan.json");
    let log = root.join("connections.jsonl");
    let cache = root.join("cache");
    fs::write(
        &plan,
        serde_json::json!({
            "schema":"spiralcore.wan.plan.v4", "cold_graph_warming":false,
            "codec_cache_enabled":true, "allowed_scopes":["01".repeat(32)]
        })
        .to_string(),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"));
    command
        .args(["serve", "--seconds", "1", "--cache"])
        .arg(&cache)
        .arg("--plan")
        .arg(&plan)
        .arg("--connection-log")
        .arg(&log)
        .env("SPIRALCORE_WAN_TOKEN", "ab".repeat(32));
    fs::write(&log, b"preserve me").unwrap();
    let occupied = command.output().unwrap();
    assert!(!occupied.status.success());
    assert_eq!(fs::read(&log).unwrap(), b"preserve me");
    assert!(!cache.exists());
    fs::remove_file(&log).unwrap();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    reader.read_line(&mut ready).unwrap();
    assert!(
        ready.starts_with("READY addr="),
        "missing diagnostic origin readiness: {ready:?}"
    );
    let address = ready.trim().strip_prefix("READY addr=").unwrap();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400"));
    assert!(child.wait().unwrap().success());
    let text = fs::read_to_string(&log).unwrap();
    let rows: Vec<serde_json::Value> = text
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["http_status"], 400);
    assert_eq!(rows[0]["connection_ordinal"], 1);
    assert!(!text.contains(&"ab".repeat(32)));
    assert!(!text.contains("GET /"));
    fs::remove_dir_all(&root).unwrap();
}
