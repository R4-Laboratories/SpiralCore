use serde_json::Value;
use spiralcore_transfer::{hex, object_id};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "spiral-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Server {
    child: Child,
    addr: String,
}
impl Server {
    fn start(cache: &Path, sessions: usize) -> Self {
        let mut child = command()
            .args(["serve", "--cache"])
            .arg(cache)
            .args(["--bind", "127.0.0.1:0", "--sessions", &sessions.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        if !line.starts_with("READY addr=") {
            let output = child.wait_with_output().unwrap();
            panic!(
                "server not ready: {line} {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Self {
            child,
            addr: line.trim().strip_prefix("READY addr=").unwrap().into(),
        }
    }
    fn wait(&mut self) {
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_spiralcore"))
}
fn send(addr: &str, file: &Path, base: Option<&Path>, mode: Option<&str>) -> Value {
    let mut cmd = command();
    cmd.args(["send", "--addr", addr, "--file"]).arg(file);
    if let Some(base) = base {
        cmd.arg("--base").arg(base);
    }
    if let Some(mode) = mode {
        cmd.args(["--mode", mode]);
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn real_loopback_cold_warm_mutation_and_exclusive_restart_read() {
    let t = Temp::new();
    let cache = t.0.join("cache");
    let base = t.0.join("base");
    let changed = t.0.join("changed");
    let bytes: Vec<u8> = (0..65536usize)
        .map(|n| ((n * 73 + n / 256) & 255) as u8)
        .collect();
    let mut mutation = bytes.clone();
    mutation[32768..32800].fill(211);
    fs::write(&base, &bytes).unwrap();
    fs::write(&changed, &mutation).unwrap();
    let mut server = Server::start(&cache, 3);
    let cold = send(&server.addr, &base, None, Some("raw"));
    let output = t.0.join("before-stop");
    assert!(!command()
        .args(["get", "--cache"])
        .arg(&cache)
        .args(["--object", &hex(&object_id(&bytes)), "--out"])
        .arg(output)
        .output()
        .unwrap()
        .status
        .success());
    let warm = send(&server.addr, &base, Some(&base), None);
    let delta = send(&server.addr, &changed, Some(&base), None);
    server.wait();
    assert_eq!(cold["object"], hex(&object_id(&bytes)));
    assert_eq!(warm["records_sent"], 0);
    assert!(
        warm["socket_frame_bytes"].as_u64().unwrap() < cold["socket_frame_bytes"].as_u64().unwrap()
    );
    assert!(delta["records_sent"].as_u64().unwrap() > 0);
    let out = t.0.join("recovered");
    let result = command()
        .args(["get", "--cache"])
        .arg(&cache)
        .args(["--object", &hex(&object_id(&mutation)), "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(out).unwrap(), mutation);
}

#[test]
fn zstd_send_and_get_preserve_trailing_zeroes() {
    let t = Temp::new();
    let cache = t.0.join("cache");
    let file = t.0.join("payload");
    let bytes = vec![0u8; 65537];
    fs::write(&file, &bytes).unwrap();
    let mut server = Server::start(&cache, 1);
    let stats = send(&server.addr, &file, None, Some("zstd"));
    server.wait();
    assert_eq!(stats["object"], hex(&object_id(&bytes)));
    let out = t.0.join("out");
    assert!(command()
        .args(["get", "--cache"])
        .arg(&cache)
        .args(["--object", &hex(&object_id(&bytes)), "--out"])
        .arg(&out)
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(fs::read(out).unwrap(), bytes);
}

#[test]
fn unsafe_addresses_bad_options_and_oversized_files_are_rejected() {
    let t = Temp::new();
    let bind = command()
        .args(["serve", "--cache"])
        .arg(t.0.join("cache"))
        .args(["--bind", "0.0.0.0:12345"])
        .output()
        .unwrap();
    assert!(!bind.status.success());
    assert!(String::from_utf8_lossy(&bind.stderr).contains("only loopback"));
    let dial = command()
        .args(["send", "--addr", "192.0.2.1:1", "--file"])
        .arg(t.0.join("unused"))
        .output()
        .unwrap();
    assert!(!dial.status.success());
    assert!(String::from_utf8_lossy(&dial.stderr).contains("only loopback"));
    let mode = command()
        .args([
            "send",
            "--addr",
            "127.0.0.1:1",
            "--file",
            "unused",
            "--mode",
            "dag",
        ])
        .output()
        .unwrap();
    assert!(!mode.status.success());
    assert!(String::from_utf8_lossy(&mode.stderr).contains("--mode must be"));
    let path = t.0.join("oversized");
    fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let result = command()
        .args(["send", "--addr", "127.0.0.1:1", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Limit"));
}

#[test]
fn partial_and_oversized_socket_frames_close_without_publication() {
    let t = Temp::new();
    let cache = t.0.join("cache");
    let mut server = Server::start(&cache, 3);
    for bytes in [vec![1, 2], (65537u32).to_le_bytes().to_vec(), {
        let mut b = (56u32).to_le_bytes().to_vec();
        b.extend([1, 2]);
        b
    }] {
        let mut stream = TcpStream::connect(&server.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(&bytes).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut reply = Vec::new();
        let result = stream.read_to_end(&mut reply);
        assert!(
            result.is_ok()
                || matches!(
                    result.unwrap_err().kind(),
                    std::io::ErrorKind::ConnectionReset
                )
        );
        assert!(reply.is_empty());
    }
    server.wait();
    assert!(fs::read_dir(cache).unwrap().all(|entry| matches!(
        entry.unwrap().file_name().to_str(),
        Some(".owner" | ".spiralcore-profile-v1")
    )));
}

#[test]
fn demo_uses_tcp_and_verifies_all_three_objects_after_restart() {
    let t = Temp::new();
    let output = command()
        .args(["demo", "--cache"])
        .arg(t.0.join("cache"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["label"], "cold");
    assert_eq!(rows[1]["label"], "repeat");
    assert_eq!(rows[2]["label"], "mutation");
    assert_eq!(rows[3]["restart_verified"], true);
    assert_eq!(rows[0]["object_bytes"], 1 << 20);
    assert!(
        rows[1]["socket_frame_bytes"].as_u64().unwrap()
            < rows[0]["socket_frame_bytes"].as_u64().unwrap() / 10
    );
    let repeat = command()
        .args(["demo", "--cache"])
        .arg(t.0.join("cache"))
        .output()
        .unwrap();
    assert!(!repeat.status.success());
    assert!(String::from_utf8_lossy(&repeat.stderr).contains("cold measurement is cold"));
}

#[test]
fn protocol_error_reply_preserves_request_identity() {
    use spiralcore_transfer::protocol::Frame;
    let t = Temp::new();
    let mut server = Server::start(&t.0.join("cache"), 1);
    let mut stream = TcpStream::connect(&server.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let request = Frame {
        kind: 5,
        transfer: [17; 32],
        seq: 99,
        body: vec![],
    };
    let bytes = request.encode().unwrap();
    stream
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .unwrap();
    stream.write_all(&bytes).unwrap();
    let mut length = [0; 4];
    stream.read_exact(&mut length).unwrap();
    let mut bytes = vec![0; u32::from_le_bytes(length) as usize];
    stream.read_exact(&mut bytes).unwrap();
    let reply = Frame::decode(&bytes).unwrap();
    assert_eq!(reply.kind, 127);
    assert_eq!(reply.transfer, request.transfer);
    assert_eq!(reply.seq, request.seq);
    assert_eq!(
        reply.body,
        (spiralcore_transfer::Error::Unknown as u16).to_le_bytes()
    );
    drop(stream);
    server.wait();
}
