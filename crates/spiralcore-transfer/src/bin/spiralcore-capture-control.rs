//! Linux-only experiment supervisor. This observes bounded quiet intervals; it cannot
//! prove that the network is drained or that traffic outside the interval was captured.
#[path = "../capture_fixture.rs"]
mod capture_fixture;
#[allow(dead_code)]
#[path = "../capture_lifecycle.rs"]
mod capture_lifecycle;
use capture_lifecycle::{parse_counter_block, parse_stats_sample, quiet, Lifecycle, Snapshot};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    net::{IpAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
const MAX_LOG: usize = 1024 * 1024;
const MAX_PCAP: u64 = 256 * 1024 * 1024;
const POLL: Duration = Duration::from_millis(20);
const SAMPLE: Duration = Duration::from_millis(500);
const SUDO: &str = "/usr/bin/sudo";
const TCPDUMP: &str = "/usr/bin/tcpdump";
#[derive(Default)]
struct Log {
    bytes: Vec<u8>,
    failed: bool,
}
struct Capture {
    child: Child,
    pid: Option<u32>,
    identity: Option<String>,
    log: Arc<Mutex<Log>>,
    reader: Option<thread::JoinHandle<()>>,
    pcap: PathBuf,
    start: Instant,
    offset: usize,
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn elapsed(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}
fn new_file(path: &Path) -> Result<File> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    o.mode(0o600);
    o.open(path).map_err(error)
}
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let f = File::open(path).map_err(error)?;
    if !f.metadata().map_err(error)?.is_file() {
        return Err("not a regular input file".into());
    }
    if f.metadata().map_err(error)?.len() > limit as u64 {
        return Err("input exceeds bound".into());
    }
    let mut bytes = Vec::new();
    f.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() > limit {
        return Err("input grew beyond bound".into());
    }
    Ok(bytes)
}
fn digest(path: &Path, limit: u64) -> Result<String> {
    let mut f = File::open(path).map_err(error)?;
    if !f.metadata().map_err(error)?.is_file() || f.metadata().map_err(error)?.len() > limit {
        return Err("hash input exceeds bound or is not regular".into());
    }
    let mut hasher = Sha256::new();
    let mut buf = [0; 65536];
    let mut len = 0u64;
    loop {
        let n = f.read(&mut buf).map_err(error)?;
        if n == 0 {
            break;
        }
        len += n as u64;
        if len > limit {
            return Err("hash input grew beyond bound".into());
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn process_identity(pid: u32) -> Result<String> {
    // Linux /proc starttime distinguishes a recycled numeric PID. The still-live
    // tracked sudo child is also required before each privileged signal.
    let path = PathBuf::from(format!("/proc/{pid}/stat"));
    let s = String::from_utf8(bounded(&path, 4096)?).map_err(error)?;
    let end = s.rfind(')').ok_or("invalid process stat")?;
    s[end + 1..]
        .split_whitespace()
        .nth(19)
        .map(str::to_owned)
        .ok_or_else(|| "missing process starttime".into())
}
fn collect_log(mut input: impl Read, shared: &Mutex<Log>) {
    let mut buf = [0; 4096];
    loop {
        match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let mut log = shared.lock().unwrap();
                let left = MAX_LOG.saturating_sub(log.bytes.len());
                log.bytes.extend_from_slice(&buf[..n.min(left)]);
                if n > left {
                    log.failed = true;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                shared.lock().unwrap().failed = true;
                break;
            }
        }
    }
}
impl Capture {
    fn signal(&mut self, signal: &str) -> Result<()> {
        if self.child.try_wait().map_err(error)?.is_some() {
            return Err(format!("capture child exited before {signal}"));
        }
        let pid = self.pid.ok_or("capture PID not established")?;
        if self.identity.as_ref() != Some(&process_identity(pid)?) {
            return Err("capture PID identity changed".into());
        }
        let status = capture_fixture::bounded_signal(pid, signal)?;
        if status["success"] != true {
            return Err(format!("capture signal {signal} failed"));
        }
        Ok(())
    }
    fn log_bytes(&self) -> Result<Vec<u8>> {
        let log = self.log.lock().map_err(error)?;
        if log.failed {
            return Err("capture stderr exceeded bound or reader failed".into());
        }
        Ok(log.bytes.clone())
    }
    fn file_bytes(&self) -> Result<u64> {
        let m = fs::metadata(&self.pcap).map_err(error)?;
        if !m.is_file() || m.len() > MAX_PCAP {
            return Err("capture file exceeded bound or is not regular".into());
        }
        Ok(m.len())
    }
    fn snapshot(&mut self) -> Result<Snapshot> {
        self.signal("-USR2")?; // Output flush only, never claimed as a kernel-ring drain.
        self.signal("-USR1")?;
        let until = Instant::now() + Duration::from_millis(900);
        loop {
            let bytes = self.log_bytes()?;
            let text = std::str::from_utf8(&bytes[self.offset..]).map_err(error)?;
            if text.len() > 4096 {
                return Err("statistics sample exceeds text bound".into());
            }
            if let Ok(counters) = parse_stats_sample(text) {
                let stderr_text = text.to_owned();
                self.offset = bytes.len();
                return Ok(Snapshot {
                    elapsed_millis: elapsed(self.start),
                    stderr_end: self.offset as u64,
                    file_bytes: self.file_bytes()?,
                    counters,
                    stderr_text,
                });
            }
            if Instant::now() >= until {
                return Err("missing, partial, duplicate or invalid statistics sample".into());
            }
            if self.child.try_wait().map_err(error)?.is_some() {
                return Err("capture exited while sampling".into());
            }
            thread::sleep(POLL);
        }
    }
    fn stop(&mut self, lifecycle: &mut Lifecycle) {
        lifecycle.interrupt_elapsed_millis = Some(elapsed(self.start));
        if let Err(e) = self.signal("-INT") {
            lifecycle.errors.push(e);
        }
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    lifecycle.capture_exit_code = status.code();
                    break;
                }
                Err(e) => {
                    lifecycle.errors.push(error(e));
                    break;
                }
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
                Ok(None) => {
                    lifecycle.forced_cleanup = true;
                    if let Err(e) = self.signal("-KILL") {
                        lifecycle.errors.push(e);
                    }
                    let _ = self.child.kill();
                    let kill_deadline = Instant::now() + Duration::from_millis(500);
                    loop {
                        match self.child.try_wait() {
                            Ok(Some(status)) => {
                                lifecycle.capture_exit_code = status.code();
                                break;
                            }
                            Ok(None) if Instant::now() < kill_deadline => thread::sleep(POLL),
                            _ => {
                                lifecycle.errors.push(
                                    "capture could not be reaped after forced cleanup".into(),
                                );
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
        if let Some(reader) = self.reader.take() {
            let deadline = Instant::now() + Duration::from_millis(250);
            while !reader.is_finished() && Instant::now() < deadline {
                thread::sleep(POLL);
            }
            if !reader.is_finished() {
                lifecycle
                    .errors
                    .push("capture stderr reader did not reach EOF".into());
            } else if reader.join().is_err() {
                lifecycle.errors.push("stderr reader panicked".into());
            }
        }
        match self.log_bytes() {
            Ok(bytes) => {
                lifecycle.stderr_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
                // The suffix starts after the last acknowledged USR1 sample. An
                // earlier complete snapshot can never substitute for final stats.
                let final_result = std::str::from_utf8(&bytes[self.offset..])
                    .map_err(error)
                    .and_then(parse_counter_block);
                match (final_result, self.file_bytes()) {
                    (Ok(counters), Ok(file_bytes)) => {
                        lifecycle.final_snapshot = Some(Snapshot {
                            elapsed_millis: elapsed(self.start),
                            stderr_end: bytes.len() as u64,
                            file_bytes,
                            counters,
                            stderr_text: std::str::from_utf8(&bytes[self.offset..])
                                .unwrap()
                                .to_owned(),
                        })
                    }
                    (a, b) => {
                        if let Err(e) = a {
                            lifecycle.errors.push(e);
                        }
                        if let Err(e) = b {
                            lifecycle.errors.push(e);
                        }
                    }
                }
            }
            Err(e) => lifecycle.errors.push(e),
        }
    }
}
struct Options {
    out: PathBuf,
    interface: String,
    local: String,
    remote: String,
    port: u16,
    command: Vec<OsString>,
    interrupt_for_qualification: bool,
}
fn options(mut args: impl Iterator<Item = OsString>) -> Result<Options> {
    let mut opts = BTreeMap::new();
    let mut command = Vec::new();
    while let Some(k) = args.next() {
        if k == "--" {
            command.extend(args);
            break;
        }
        let k = k.into_string().map_err(|_| "invalid option")?;
        if !matches!(
            k.as_str(),
            "--out-dir" | "--interface" | "--local-ip" | "--remote-ip" | "--port"
        ) {
            return Err("unknown option".into());
        }
        let v = args.next().ok_or("missing option value")?;
        if opts.insert(k, v).is_some() {
            return Err("duplicate option".into());
        }
    }
    let out = PathBuf::from(opts.remove("--out-dir").ok_or("missing --out-dir")?);
    let mut string = |key: &str| -> Result<String> {
        opts.remove(key)
            .ok_or_else(|| format!("missing {key}"))?
            .into_string()
            .map_err(|_| "invalid string".into())
    };
    let interface = string("--interface")?;
    let local = string("--local-ip")?;
    let remote = string("--remote-ip")?;
    if interface.is_empty()
        || interface.len() > 32
        || !interface
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
    {
        return Err("invalid interface".into());
    }
    let lip = local.parse::<IpAddr>().map_err(error)?;
    let rip = remote.parse::<IpAddr>().map_err(error)?;
    if lip == rip || lip.is_ipv4() != rip.is_ipv4() {
        return Err("capture endpoints must be distinct and use the same IP family".into());
    }
    let port = opts
        .remove("--port")
        .map(|s| {
            s.into_string()
                .map_err(|_| "invalid port")
                .and_then(|s| s.parse::<u16>().map_err(|_| "invalid port"))
        })
        .transpose()?
        .unwrap_or(443);
    if port == 0 || command.is_empty() {
        return Err("nonzero port and client command after -- are required".into());
    }
    Ok(Options {
        out,
        interface,
        local: lip.to_string(),
        remote: rip.to_string(),
        port,
        command,
        interrupt_for_qualification: false,
    })
}
fn capture_filter(remote: &str, port: u16) -> String {
    format!("host {remote} and tcp port {port}")
}
fn execute(options: Options) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Err("capture control requires Linux /proc and tcpdump signals".into());
    }
    let out = fs::canonicalize(options.out).map_err(error)?;
    let state = out.join(".capture-control");
    let mut db = fs::DirBuilder::new();
    #[cfg(unix)]
    db.mode(0o700);
    db.create(&state).map_err(error)?;
    // All authoritative evidence paths are exclusive; a retry needs a fresh directory.
    let mut evidence = new_file(&out.join("capture-lifecycle.json"))?;
    let mut stats = new_file(&out.join("tcpdump-stats.txt"))?;
    let pcap = out.join("headers.pcap");
    drop(new_file(&pcap)?);
    let pid_path = state.join("pid");
    drop(new_file(&pid_path)?);
    let mut lifecycle = Lifecycle::new(
        options.interface.clone(),
        options.local,
        options.remote.clone(),
        options.port,
    );
    // An interrupted setup still leaves parseable, explicitly incomplete state.
    serde_json::to_writer_pretty(&mut evidence, &lifecycle).map_err(error)?;
    evidence.sync_all().map_err(error)?;
    let start = Instant::now();
    let version = Command::new(TCPDUMP)
        .arg("--version")
        .output()
        .map_err(error)?;
    if !version.status.success() || version.stdout.len() + version.stderr.len() > 65536 {
        return Err("unable to obtain bounded tcpdump version".into());
    }
    lifecycle.tool_versions = String::from_utf8(version.stdout).map_err(error)?
        + &String::from_utf8(version.stderr).map_err(error)?;
    lifecycle.tcpdump_sha256 = digest(Path::new(TCPDUMP), 64 * 1024 * 1024)?;
    let user = Command::new("/usr/bin/id")
        .arg("-un")
        .output()
        .map_err(error)?;
    let user = String::from_utf8(user.stdout)
        .map_err(error)?
        .trim()
        .to_owned();
    if user.is_empty()
        || user.len() > 64
        || !user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err("invalid capture drop user".into());
    }
    let filter = capture_filter(&options.remote, options.port);
    let flags = vec![
        "--immediate-mode".into(),
        "-B".into(),
        "4096".into(),
        "-U".into(),
        "-s".into(),
        "96".into(),
        "--time-stamp-precision=nano".into(),
        "-i".into(),
        options.interface,
        "-Z".into(),
        user,
        "-w".into(),
        pcap.to_string_lossy().into_owned(),
        filter,
    ];
    lifecycle.flags = flags.clone();
    // dash's ulimit -f uses 512-byte blocks. RLIMIT_FSIZE caps PCAP writes even
    // while the controller blocks on the measured child. Stderr has a separate
    // bounded reader; no repeated polling runs during the measured command.
    let mut child = Command::new(SUDO)
        .args([
            "-n",
            "/bin/sh",
            "-c",
            "ulimit -f 524288 || exit 91; printf '%s\\n' \"$$\" > \"$1\"; shift; exec \"$@\"",
            "spiralcore-capture",
            pid_path.to_str().ok_or("non-UTF8 output path")?,
            TCPDUMP,
        ])
        .args(&flags)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(error)?;
    let stderr = child.stderr.take().ok_or("missing capture stderr pipe")?;
    let log = Arc::new(Mutex::new(Log::default()));
    let reader_log = log.clone();
    let reader = thread::spawn(move || collect_log(stderr, &reader_log));
    let mut capture = Capture {
        child,
        pid: None,
        identity: None,
        log,
        reader: Some(reader),
        pcap: pcap.clone(),
        start,
        offset: 0,
    };
    let experiment = (|| -> Result<()> {
        let deadline = start + Duration::from_secs(10);
        loop {
            if capture.child.try_wait().map_err(error)?.is_some() {
                return Err("capture exited before readiness".into());
            }
            if capture.pid.is_none() {
                let bytes = bounded(&pid_path, 32)?;
                if let Ok(pid) = std::str::from_utf8(&bytes)
                    .map_err(error)?
                    .trim()
                    .parse::<u32>()
                {
                    if pid > 1 {
                        capture.identity = Some(process_identity(pid)?);
                        capture.pid = Some(pid);
                        lifecycle.capture_pid = Some(pid);
                    }
                }
            }
            let log = capture.log_bytes()?;
            if capture.pid.is_some()
                && std::str::from_utf8(&log)
                    .map_err(error)?
                    .contains("listening on ")
                && capture.file_bytes()? >= 24
            {
                let mut bytes = [0; 24];
                File::open(&pcap)
                    .map_err(error)?
                    .read_exact(&mut bytes)
                    .map_err(error)?;
                if !matches!(
                    bytes.get(..4),
                    Some(
                        [0x4d, 0x3c, 0xb2, 0xa1]
                            | [0xa1, 0xb2, 0x3c, 0x4d]
                            | [0xd4, 0xc3, 0xb2, 0xa1]
                            | [0xa1, 0xb2, 0xc3, 0xd4]
                    )
                ) {
                    return Err("invalid initial PCAP header".into());
                }
                capture.offset = log.len();
                lifecycle.ready = true;
                lifecycle.ready_elapsed_millis = Some(elapsed(start));
                break;
            }
            if Instant::now() >= deadline {
                return Err("capture readiness timed out".into());
            }
            thread::sleep(POLL);
        }
        lifecycle.preflight.push(capture.snapshot()?);
        thread::sleep(SAMPLE);
        lifecycle.preflight.push(capture.snapshot()?);
        if !quiet(&lifecycle.preflight) {
            return Err("startup counters or file extent are not quiet and complete".into());
        }
        lifecycle.client_started_elapsed_millis = Some(elapsed(start));
        let status = Command::new(&options.command[0])
            .args(&options.command[1..])
            .stdin(Stdio::null())
            .status()
            .map_err(error)?;
        lifecycle.client_ended_elapsed_millis = Some(elapsed(start));
        lifecycle.client_exit_code = status.code();
        if !status.success() {
            lifecycle.errors.push("measured client failed".into());
        }
        if options.interrupt_for_qualification {
            capture.signal("-INT")?;
            thread::sleep(Duration::from_millis(100));
        }
        let deadline = Instant::now() + Duration::from_secs(9);
        loop {
            lifecycle.quiescence.push(capture.snapshot()?);
            let n = lifecycle.quiescence.len();
            if n >= 3 && quiet(&lifecycle.quiescence[n - 3..]) {
                break;
            }
            if Instant::now() + SAMPLE >= deadline || n >= 18 {
                return Err("post-client quiet interval timed out".into());
            }
            thread::sleep(SAMPLE);
        }
        Ok(())
    })();
    if let Err(e) = experiment {
        lifecycle.errors.push(e);
    }
    capture.stop(&mut lifecycle);
    match digest(&pcap, MAX_PCAP) {
        Ok(hash) => lifecycle.pcap_sha256 = Some(hash),
        Err(e) => lifecycle.errors.push(e),
    }
    if let Some(f) = &lifecycle.final_snapshot {
        stats.write_all(f.stderr_text.as_bytes()).map_err(error)?;
    }
    stats.sync_all().map_err(error)?;
    let final_counters = lifecycle.final_snapshot.as_ref().map(|f| f.counters);
    let failures = lifecycle.failure_reasons(final_counters, capture.file_bytes().unwrap_or(0));
    evidence.set_len(0).map_err(error)?;
    evidence.seek(SeekFrom::Start(0)).map_err(error)?;
    serde_json::to_writer_pretty(&mut evidence, &lifecycle).map_err(error)?;
    evidence.write_all(b"\n").map_err(error)?;
    evidence.sync_all().map_err(error)?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "capture lifecycle incomplete: {}",
            failures.join(", ")
        ))
    }
}
fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--help") {
        println!("spiralcore-capture-control --out-dir EXISTING_DIR --interface IFACE --local-ip IP --remote-ip IP [--port 443] -- CLIENT [ARGS...]\nLinux-only, selected-flow capture with bounded readiness/quiescence and independent final statistics. No traffic completeness claim outside the capture interval.");
        return;
    }
    let result = if args.first().is_some_and(|s| s == "--qualify") && args.len() == 2 {
        capture_fixture::qualify(Path::new(&args[1]))
    } else if args.first().is_some_and(|s| s == "--fixture-server") && args.len() == 2 {
        capture_fixture::fixture_server(&args[1])
    } else if args.first().is_some_and(|s| s == "--fixture-client") && args.len() == 4 {
        fixture_client(&args[1], Path::new(&args[2]), &args[3])
    } else {
        options(args.into_iter()).and_then(execute)
    };
    if let Err(e) = result {
        eprintln!("spiralcore-capture-control: {e}");
        std::process::exit(1);
    }
}
fn nanos() -> Result<String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_nanos()
        .to_string())
}
fn fixture_exchange(addr: &str) -> Result<()> {
    let addr = addr.parse::<std::net::SocketAddr>().map_err(error)?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(error)?;
    stream.set_nodelay(true).map_err(error)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(error)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(error)?;
    let bytes = [0x5a; 1024];
    for _ in 0..64 {
        stream.write_all(&bytes).map_err(error)?;
        let mut restored = [0; 1024];
        stream.read_exact(&mut restored).map_err(error)?;
        if bytes != restored {
            return Err("qualification fixture bytes differ".into());
        }
    }
    Ok(())
}
fn fixture_client(addr: &OsString, out: &Path, _kind: &OsString) -> Result<()> {
    let addr = addr.to_str().ok_or("invalid fixture address")?;
    if addr != capture_fixture::FIXTURE_ADDR {
        return Err("fixture only permits its isolated namespace address".into());
    }
    let start = nanos()?;
    fixture_exchange(addr)?;
    let end = nanos()?;
    let row = serde_json::json!({"task":{"id":"capture-qualification"},"phase":"transfer","status":"published","metrics":{"request_body_bytes":65536,"response_body_bytes":65536},"unix_start_nanos":start,"unix_end_nanos":end});
    let mut f = new_file(&out.join("rows.jsonl"))?;
    writeln!(f, "{row}").map_err(error)?;
    drop(new_file(&out.join("client-finished"))?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stderr_collection_is_bounded_even_for_a_noisy_child() {
        let shared = Mutex::new(Log::default());
        collect_log(std::io::repeat(b'x').take((MAX_LOG + 20) as u64), &shared);
        let log = shared.lock().unwrap();
        assert_eq!(log.bytes.len(), MAX_LOG);
        assert!(log.failed);
    }
    #[test]
    fn no_shell_text_can_enter_the_interface_and_port() {
        let args = |i: &str, p: &str| {
            [
                "--out-dir",
                "tmp",
                "--interface",
                i,
                "--local-ip",
                "10.0.0.1",
                "--remote-ip",
                "10.0.0.2",
                "--port",
                p,
                "--",
                "true",
            ]
            .map(OsString::from)
            .into_iter()
        };
        assert!(options(args("eth0; false", "443")).is_err());
        assert!(options(args("eth0", "0")).is_err());
        assert!(options(args("eth0", "443")).is_ok());
    }
    #[test]
    fn ethernet_capture_filter_has_no_direction_metadata_load() {
        assert_eq!(
            capture_filter("198.18.0.2", 443),
            "host 198.18.0.2 and tcp port 443"
        );
    }
}
