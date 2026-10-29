//! Private synthetic Linux Ethernet qualification; never a WAN measurement.
use super::*;
use serde_json::{json, Value};
use std::net::{Ipv4Addr, SocketAddr};
const IP: &str = "/usr/sbin/ip";
const ETHTOOL: &str = "/usr/sbin/ethtool";
const LOCAL: &str = "198.18.0.1";
const REMOTE: &str = "198.18.0.2";
pub(super) const FIXTURE_ADDR: &str = "198.18.0.2:443";
const TOOL_LIMIT: usize = 64 * 1024;

fn overlaps(address: &str, prefix: u64) -> Result<bool> {
    if prefix > 32 {
        return Err("invalid IPv4 prefix".into());
    }
    let address = u32::from(address.parse::<Ipv4Addr>().map_err(error)?);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let first = address & mask;
    let last = first | !mask;
    let fixture = u32::from(Ipv4Addr::new(198, 18, 0, 0));
    Ok(first <= fixture + 3 && last >= fixture)
}
fn check_collisions(addresses: &Value, routes: &Value) -> Result<()> {
    for interface in addresses.as_array().ok_or("invalid address inventory")? {
        for address in interface["addr_info"]
            .as_array()
            .ok_or("missing address information")?
        {
            if address["family"].as_str() == Some("inet")
                && overlaps(
                    address["local"].as_str().ok_or("missing IPv4 address")?,
                    address["prefixlen"].as_u64().ok_or("missing IPv4 prefix")?,
                )?
            {
                return Err("fixture address overlaps existing interface".into());
            }
        }
    }
    for route in routes.as_array().ok_or("invalid route inventory")? {
        let destination = route["dst"].as_str().ok_or("missing route destination")?;
        if destination == "default" || destination == "0.0.0.0/0" {
            continue;
        }
        let (address, prefix) = destination.split_once('/').unwrap_or((destination, "32"));
        if overlaps(address, prefix.parse::<u64>().map_err(error)?)? {
            return Err("fixture prefix overlaps existing route".into());
        }
    }
    Ok(())
}
fn offloads_disabled(text: &str) -> Result<()> {
    for key in [
        "tcp-segmentation-offload:",
        "generic-segmentation-offload:",
        "generic-receive-offload:",
        "large-receive-offload:",
    ] {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| line.split_whitespace().next() == Some(key))
            .collect::<Vec<_>>();
        if lines.len() != 1 || lines[0].split_whitespace().nth(1) != Some("off") {
            return Err(format!("offload is absent, duplicate or enabled: {key}"));
        }
    }
    Ok(())
}
fn collect_tool_log(mut input: impl Read, shared: &Mutex<Log>) {
    let mut buffer = [0; 4096];
    loop {
        match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let mut log = shared.lock().unwrap();
                let room = TOOL_LIMIT.saturating_sub(log.bytes.len());
                log.bytes.extend_from_slice(&buffer[..n.min(room)]);
                if n > room {
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
struct Pipes {
    out: Arc<Mutex<Log>>,
    err: Arc<Mutex<Log>>,
    readers: Vec<thread::JoinHandle<()>>,
}
impl Pipes {
    fn new(child: &mut Child) -> Result<Self> {
        let stdout = child.stdout.take().ok_or("missing stdout")?;
        let stderr = child.stderr.take().ok_or("missing stderr")?;
        let out = Arc::new(Mutex::new(Log::default()));
        let err = Arc::new(Mutex::new(Log::default()));
        let a = out.clone();
        let b = err.clone();
        Ok(Self {
            out,
            err,
            readers: vec![
                thread::spawn(move || collect_tool_log(stdout, &a)),
                thread::spawn(move || collect_tool_log(stderr, &b)),
            ],
        })
    }
    fn text(shared: &Mutex<Log>) -> Result<String> {
        let log = shared.lock().map_err(error)?;
        if log.failed {
            return Err("tool output exceeded bound or reader failed".into());
        }
        String::from_utf8(log.bytes.clone()).map_err(error)
    }
    fn finish(&mut self) -> Result<(String, String)> {
        let until = Instant::now() + Duration::from_millis(500);
        while self.readers.iter().any(|r| !r.is_finished()) && Instant::now() < until {
            thread::sleep(POLL);
        }
        if self.readers.iter().any(|r| !r.is_finished()) {
            return Err("tool output pipes did not close".into());
        }
        for reader in self.readers.drain(..) {
            reader.join().map_err(|_| "tool reader panic")?;
        }
        Ok((Self::text(&self.out)?, Self::text(&self.err)?))
    }
}
#[derive(Clone, Copy)]
enum Creation {
    Namespace,
    Link,
}
fn tool_command(program: &str, args: &[&str], privileged: bool) -> Command {
    let mut command = Command::new(if privileged { SUDO } else { program });
    if privileged {
        // GNU timeout runs as root and owns a separate process group. Do not use
        // --foreground: descendants must receive the TERM/KILL deadline too.
        command.args([
            "-n",
            "/usr/bin/timeout",
            "--signal=TERM",
            "--kill-after=1s",
            "5s",
            program,
        ]);
    }
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}
fn run_tool(
    program: &str,
    args: &[&str],
    privileged: bool,
    on_exit: impl FnOnce(bool),
) -> Result<Value> {
    let mut child = tool_command(program, args, privileged)
        .spawn()
        .map_err(error)?;
    let mut pipes = Pipes::new(&mut child)?;
    // The outer deadline exceeds the root supervisor's5s TERM+1s KILL window.
    let until = Instant::now() + Duration::from_secs(8);
    let (status, outer_timeout) = loop {
        if let Some(s) = child.try_wait().map_err(error)? {
            break (Some(s), false);
        }
        if Instant::now() >= until {
            let _ = child.kill();
            break (None, true);
        }
        thread::sleep(POLL);
    };
    // Creation ownership becomes authoritative immediately after raw success,
    // before pipe collection or any fallible JSON/log operation.
    on_exit(status.is_some_and(|s| s.success()));
    let mut reaped = status.is_some();
    if !reaped {
        let until = Instant::now() + Duration::from_secs(1);
        while Instant::now() < until {
            if child.try_wait().map_err(error)?.is_some() {
                reaped = true;
                break;
            }
            thread::sleep(POLL);
        }
    }
    let (stdout, stderr, output_error) = match pipes.finish() {
        Ok((a, b)) => (a, b, None),
        Err(e) => (String::new(), String::new(), Some(e)),
    };
    let success =
        !outer_timeout && reaped && output_error.is_none() && status.is_some_and(|s| s.success());
    Ok(
        json!({"program":program,"args":args,"privileged":privileged,"root_timeout_seconds":if privileged {Some(5)}else{None},"root_kill_after_seconds":if privileged {Some(1)}else{None},"exit_code":status.as_ref().and_then(|s|s.code()),"outer_timeout":outer_timeout,"command_tree_settled":!outer_timeout&&reaped,"reaped":reaped,"stdout":stdout,"stderr":stderr,"output_error":output_error,"success":success}),
    )
}
pub(super) fn bounded_signal(pid: u32, signal: &str) -> Result<Value> {
    run_tool("/bin/kill", &[signal, &pid.to_string()], true, |_| {})
}

struct Topology {
    namespace: String,
    host: String,
    peer: String,
    namespace_created: bool,
    link_created: bool,
    namespace_intended: bool,
    link_intended: bool,
    unsettled_command: bool,
    records: Vec<Value>,
    log: File,
}
impl Topology {
    fn new(dir: &Path) -> Result<Self> {
        // Generated names only, never supplied by CLI. Collision checks remain authoritative.
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_nanos();
        let nonce = format!("{stamp}:{}:{}", std::process::id(), dir.display());
        let suffix = format!("{:x}", Sha256::digest(nonce.as_bytes()))[..12].to_owned();
        Ok(Self {
            namespace: format!("scn{suffix}"),
            host: format!("sch{suffix}"),
            peer: format!("scp{suffix}"),
            namespace_created: false,
            link_created: false,
            namespace_intended: false,
            link_intended: false,
            unsettled_command: false,
            records: Vec::new(),
            log: new_file(&dir.join("topology-commands.jsonl"))?,
        })
    }
    fn tool(&mut self, program: &str, args: &[&str], privileged: bool) -> Result<String> {
        self.tool_with_creation(program, args, privileged, None)
    }
    fn tool_with_creation(
        &mut self,
        program: &str,
        args: &[&str],
        privileged: bool,
        creation: Option<Creation>,
    ) -> Result<String> {
        if self.records.len() >= 64 {
            return Err("topology command bound exceeded".into());
        }
        let result = run_tool(program, args, privileged, |success| {
            if success {
                match creation {
                    Some(Creation::Namespace) => self.namespace_created = true,
                    Some(Creation::Link) => self.link_created = true,
                    None => {}
                }
            }
        });
        let record = match result {
            Ok(record) => record,
            Err(e) => {
                self.unsettled_command = true;
                return Err(e);
            }
        };
        if record["command_tree_settled"] != true {
            self.unsettled_command = true;
        }
        self.records.push(record.clone());
        writeln!(self.log, "{record}").map_err(error)?;
        self.log.sync_all().map_err(error)?;
        if record["success"] != true {
            return Err(format!(
                "bounded topology command failed: {program} {args:?}"
            ));
        }
        Ok(record["stdout"]
            .as_str()
            .ok_or("missing command stdout")?
            .to_owned())
    }
    fn create(&mut self, args: &[&str], creation: Creation) -> Result<()> {
        let resource = match creation {
            Creation::Namespace => {
                self.namespace_intended = true;
                "namespace"
            }
            Creation::Link => {
                self.link_intended = true;
                "link"
            }
        };
        // The intent must be durable before launching a potentially mutating command.
        let intent = json!({"creation_intent":resource,"program":IP,"args":args});
        writeln!(self.log, "{intent}").map_err(error)?;
        self.log.sync_all().map_err(error)?;
        self.tool_with_creation(IP, args, true, Some(creation))
            .map(|_| ())
    }
    fn ip(&mut self, args: &[&str]) -> Result<String> {
        self.tool(IP, args, true)
    }
    fn setup(&mut self) -> Result<()> {
        self.tool("/usr/bin/timeout", &["--version"], false)?;
        self.tool(IP, &["-Version"], false)?;
        self.tool(ETHTOOL, &["--version"], false)?;
        let addresses: Value =
            serde_json::from_str(&self.ip(&["-j", "-4", "addr", "show"])?).map_err(error)?;
        let routes: Value =
            serde_json::from_str(&self.ip(&["-j", "-4", "route", "show", "table", "all"])?)
                .map_err(error)?;
        check_collisions(&addresses, &routes)?;
        let links: Value =
            serde_json::from_str(&self.ip(&["-j", "link", "show"])?).map_err(error)?;
        if links
            .as_array()
            .ok_or("invalid link inventory")?
            .iter()
            .any(|v| {
                v["ifname"]
                    .as_str()
                    .is_some_and(|n| n == self.host || n == self.peer)
            })
        {
            return Err("fixture link already exists".into());
        }
        if self
            .ip(&["netns", "list"])?
            .lines()
            .any(|s| s.split_whitespace().next() == Some(&self.namespace))
        {
            return Err("fixture namespace already exists".into());
        }
        let ns = self.namespace.clone();
        let host = self.host.clone();
        let peer = self.peer.clone();
        self.create(&["netns", "add", &ns], Creation::Namespace)?;
        self.create(
            &["link", "add", &host, "type", "veth", "peer", "name", &peer],
            Creation::Link,
        )?;
        self.ip(&["link", "set", &peer, "netns", &ns])?;
        self.ip(&["addr", "add", "198.18.0.1/30", "dev", &host])?;
        self.ip(&[
            "netns",
            "exec",
            &ns,
            IP,
            "addr",
            "add",
            "198.18.0.2/30",
            "dev",
            &peer,
        ])?;
        self.ip(&["link", "set", &host, "up"])?;
        self.ip(&["netns", "exec", &ns, IP, "link", "set", &peer, "up"])?;
        self.ip(&["netns", "exec", &ns, IP, "link", "set", "lo", "up"])?;
        self.tool(
            ETHTOOL,
            &[
                "-K", &host, "tso", "off", "gso", "off", "gro", "off", "lro", "off",
            ],
            true,
        )?;
        self.ip(&[
            "netns", "exec", &ns, ETHTOOL, "-K", &peer, "tso", "off", "gso", "off", "gro", "off",
            "lro", "off",
        ])?;
        offloads_disabled(&self.tool(ETHTOOL, &["-k", &host], true)?)?;
        offloads_disabled(&self.ip(&["netns", "exec", &ns, ETHTOOL, "-k", &peer])?)?;
        self.ip(&["-j", "addr", "show", "dev", &host])?;
        self.ip(&["netns", "exec", &ns, IP, "-j", "addr", "show"])?;
        self.ip(&[
            "netns", "exec", &ns, IP, "-j", "route", "show", "table", "all",
        ])?;
        Ok(())
    }
    fn cleanup(&mut self) -> Result<()> {
        if self.unsettled_command {
            return Err("privileged command tree unsettled; cleanup unverified; retain for disposable-runner teardown".into());
        }
        let ns = self.namespace.clone();
        let host = self.host.clone();
        let peer = self.peer.clone();
        let mut errors = Vec::new();
        // A failed or timed-out creation can leave a name present without proof
        // that this process owns it. Do not convert unique naming into authority.
        if self.namespace_intended && !self.namespace_created {
            match self.ip(&["netns", "list"]) {
                Ok(s) if !s.lines().any(|s| s.split_whitespace().next() == Some(&ns)) => {}
                _ => errors.push("uncertain namespace ownership; cleanup unverified".into()),
            }
        }
        if self.link_intended && !self.link_created {
            match self
                .ip(&["-j", "link", "show"])
                .and_then(|s| serde_json::from_str::<Value>(&s).map_err(error))
            {
                Ok(v)
                    if v.as_array().is_some_and(|a| {
                        !a.iter()
                            .any(|v| v["ifname"].as_str().is_some_and(|n| n == host || n == peer))
                    }) => {}
                _ => errors.push("uncertain link ownership; cleanup unverified".into()),
            }
        }
        if self.namespace_created {
            match self.ip(&["netns", "pids", &ns]) {
                Ok(pids) if pids.trim().is_empty() => {}
                Ok(_) => errors.push("owned fixture namespace still has processes".to_string()),
                Err(e) => errors.push(e),
            }
        }
        if self.link_created {
            match self.ip(&["link", "delete", &host]) {
                Ok(_) => self.link_created = false,
                Err(e) => errors.push(e),
            }
        }
        if self.namespace_created {
            match self.ip(&["netns", "delete", &ns]) {
                Ok(_) => self.namespace_created = false,
                Err(e) => errors.push(e),
            }
        }
        match self
            .ip(&["-j", "link", "show"])
            .and_then(|s| serde_json::from_str::<Value>(&s).map_err(error))
        {
            Ok(v)
                if v.as_array().is_some_and(|a| {
                    !a.iter()
                        .any(|v| v["ifname"].as_str().is_some_and(|n| n == host || n == peer))
                }) => {}
            _ => errors.push("fixture link cleanup unverified".into()),
        }
        match self.ip(&["netns", "list"]) {
            Ok(s) if !s.lines().any(|s| s.split_whitespace().next() == Some(&ns)) => {}
            _ => errors.push("fixture namespace cleanup unverified".into()),
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}
struct Server {
    child: Child,
    pipes: Pipes,
    pid: Option<u32>,
    identity: Option<String>,
    pid_path: PathBuf,
    signals: Vec<Value>,
}
impl Server {
    fn spawn(executable: &Path, dir: &Path, ns: &str, kind: &str) -> Result<Self> {
        let pid_path = dir.join("fixture-server.pid");
        drop(new_file(&pid_path)?);
        // Fixed shell text only; every path/name is a separate argv element.
        let mut child = Command::new(SUDO)
            .args([
                "-n",
                "/bin/sh",
                "-c",
                "printf '%s\\n' \"$$\" > \"$1\"; shift; exec \"$@\"",
                "spiralcore-fixture",
            ])
            .arg(&pid_path)
            .args([IP, "netns", "exec", ns])
            .arg(executable)
            .args(["--fixture-server", kind])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(error)?;
        let pipes = Pipes::new(&mut child)?;
        Ok(Self {
            child,
            pipes,
            pid: None,
            identity: None,
            pid_path,
            signals: Vec::new(),
        })
    }
    fn ready(&mut self) -> Result<()> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if self.child.try_wait().map_err(error)?.is_some() {
                return Err("fixture server exited before readiness".into());
            }
            if self.pid.is_none() {
                if let Ok(pid) = std::str::from_utf8(&bounded(&self.pid_path, 32)?)
                    .map_err(error)?
                    .trim()
                    .parse::<u32>()
                {
                    if pid > 1 {
                        self.identity = Some(process_identity(pid)?);
                        self.pid = Some(pid);
                    }
                }
            }
            if self.pid.is_some() && Pipes::text(&self.pipes.out)? == "ready\n" {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err("fixture server readiness timeout".into());
            }
            thread::sleep(POLL);
        }
    }
    fn signal(&mut self, signal: &str) -> Result<()> {
        if self.child.try_wait().map_err(error)?.is_some() {
            return Ok(());
        }
        let pid = self.pid.ok_or("fixture PID unavailable")?;
        if self.identity.as_ref() != Some(&process_identity(pid)?) {
            return Err("fixture PID identity changed".into());
        }
        let result = bounded_signal(pid, signal)?;
        let success = result["success"] == true;
        self.signals.push(result);
        if !success {
            return Err("fixture bounded signal failed".into());
        }
        Ok(())
    }
    fn finish(&mut self, graceful: bool) -> Result<Value> {
        let mut forced = false;
        let mut failures = Vec::new();
        if !graceful {
            forced = true;
            if let Err(e) = self.signal("-TERM") {
                failures.push(e);
            }
        }
        let until = Instant::now() + Duration::from_secs(if graceful { 5 } else { 1 });
        let mut status = None;
        while Instant::now() < until {
            status = self.child.try_wait().map_err(error)?;
            if status.is_some() {
                break;
            }
            thread::sleep(POLL);
        }
        if status.is_none() {
            forced = true;
            if let Err(e) = self.signal("-KILL") {
                failures.push(e);
            }
            let until = Instant::now() + Duration::from_secs(1);
            while Instant::now() < until {
                status = self.child.try_wait().map_err(error)?;
                if status.is_some() {
                    break;
                }
                thread::sleep(POLL);
            }
        }
        if status.is_none() {
            failures.push("fixture server could not be reaped".into());
        }
        if graceful && (forced || !status.as_ref().is_some_and(|s| s.success())) {
            failures.push("fixture server failed or exceeded exit deadline".into());
        }
        let (stdout, stderr) = match self.pipes.finish() {
            Ok(v) => v,
            Err(e) => {
                failures.push(e);
                (String::new(), String::new())
            }
        };
        Ok(
            json!({"pid":self.pid,"starttime":self.identity,"exit_code":status.and_then(|s|s.code()),"reaped":status.is_some(),"forced_cleanup":forced,"stdout":stdout,"stderr":stderr,"errors":failures,"signals":self.signals}),
        )
    }
}
pub(super) fn fixture_server(kind: &OsString) -> Result<()> {
    let kind = kind.to_str().ok_or("invalid fixture kind")?;
    if !matches!(kind, "normal" | "interrupted" | "ongoing") {
        return Err("invalid fixture kind".into());
    }
    let listener = TcpListener::bind(FIXTURE_ADDR).map_err(error)?;
    listener.set_nonblocking(true).map_err(error)?;
    println!("ready");
    std::io::stdout().flush().map_err(error)?;
    let deadline = Instant::now() + Duration::from_secs(25);
    for connection in 0..if kind == "ongoing" { 2 } else { 1 } {
        let mut stream = loop {
            match listener.accept() {
                Ok((s, peer)) => {
                    if peer.ip().to_string() != LOCAL {
                        return Err("unexpected fixture peer".into());
                    }
                    break s;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("fixture accept timeout".into());
                    }
                    thread::sleep(POLL);
                }
                Err(e) => return Err(error(e)),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(error)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(error)?;
        stream.set_nodelay(true).map_err(error)?;
        if connection == 0 {
            for _ in 0..64 {
                if Instant::now() >= deadline {
                    return Err("fixture total deadline".into());
                }
                let mut bytes = [0; 1024];
                stream.read_exact(&mut bytes).map_err(error)?;
                if bytes != [0x5a; 1024] {
                    return Err("fixture request mismatch".into());
                }
                stream.write_all(&bytes).map_err(error)?;
            }
            thread::sleep(Duration::from_millis(700));
        } else {
            let mut count = 0;
            loop {
                if Instant::now() >= deadline || count > 120 {
                    return Err("fixture pulse bound".into());
                }
                let mut b = [0];
                match stream.read_exact(&mut b) {
                    Ok(()) => {
                        if b != [0x6b] {
                            return Err("fixture pulse mismatch".into());
                        }
                        stream.write_all(&b).map_err(error)?;
                        count += 1;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(error(e)),
                }
            }
            if count != 120 {
                return Err("fixture pulse count differs".into());
            }
        }
    }
    Ok(())
}
fn pulse(marker: PathBuf) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(15);
    while !marker.is_file() {
        if Instant::now() >= until {
            return Err("fixture marker timeout".into());
        }
        thread::sleep(POLL);
    }
    let addr = FIXTURE_ADDR.parse::<SocketAddr>().map_err(error)?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(error)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(error)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(error)?;
    stream.set_nodelay(true).map_err(error)?;
    for _ in 0..120 {
        stream.write_all(&[0x6b]).map_err(error)?;
        let mut b = [0];
        stream.read_exact(&mut b).map_err(error)?;
        if b != [0x6b] {
            return Err("pulse response differs".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}
fn case(
    dir: &Path,
    kind: &str,
    executable: &Path,
    topology: &mut Topology,
    server: &mut Option<Server>,
) -> Result<Value> {
    topology.setup()?;
    *server = Some(Server::spawn(executable, dir, &topology.namespace, kind)?);
    server.as_mut().unwrap().ready()?;
    let traffic = if kind == "ongoing" {
        let marker = dir.join("client-finished");
        Some(thread::spawn(move || pulse(marker)))
    } else {
        None
    };
    let result = execute(Options {
        out: dir.into(),
        interface: topology.host.clone(),
        local: LOCAL.into(),
        remote: REMOTE.into(),
        port: 443,
        command: vec![
            executable.as_os_str().into(),
            "--fixture-client".into(),
            FIXTURE_ADDR.into(),
            dir.as_os_str().into(),
            kind.into(),
        ],
        interrupt_for_qualification: kind == "interrupted",
    });
    if let Some(traffic) = traffic {
        traffic.join().map_err(|_| "fixture pulse panic")??;
    }
    if result.is_err() && !dir.join("rows.jsonl").is_file() {
        return result
            .map(|_| Value::Null)
            .map_err(|e| format!("capture setup failed before fixture: {e}"));
    }
    let evidence = bounded(&dir.join("capture-lifecycle.json"), MAX_LOG)?;
    let lifecycle: Lifecycle = serde_json::from_slice(&evidence).map_err(error)?;
    let status = Command::new(executable.with_file_name("spiralcore-capture"))
        .arg("--pcap")
        .arg(dir.join("headers.pcap"))
        .arg("--rows")
        .arg(dir.join("rows.jsonl"))
        .args([
            "--local-ip",
            LOCAL,
            "--remote-ip",
            REMOTE,
            "--qualification-topology",
            "veth",
            "--capture-stats",
        ])
        .arg(dir.join("tcpdump-stats.txt"))
        .arg("--capture-lifecycle")
        .arg(dir.join("capture-lifecycle.json"))
        .arg("--out")
        .arg(dir.join("capture.json"))
        .status()
        .map_err(error)?;
    if !status.success() {
        return Err(format!("{kind} parser qualification failed"));
    }
    let report: Value =
        serde_json::from_slice(&bounded(&dir.join("capture.json"), MAX_LOG)?).map_err(error)?;
    let complete = report["capture_complete"]
        .as_bool()
        .ok_or("missing completeness")?;
    let expected = kind == "normal";
    if result.is_ok() != expected || complete != expected {
        return Err(format!("{kind} lifecycle/completeness differs: {result:?}"));
    }
    if kind == "ongoing"
        && !lifecycle
            .errors
            .iter()
            .any(|s| s == "post-client quiet interval timed out")
    {
        return Err("ongoing fixture did not exercise quiet timeout".into());
    }
    Ok(
        json!({"case":kind,"controller_success":result.is_ok(),"capture_complete":complete,"expected_success":expected,"lifecycle_sha256":format!("{:x}",Sha256::digest(&evidence)),"errors":lifecycle.errors}),
    )
}
pub(super) fn qualify(out: &Path) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Err("live capture qualification requires Linux".into());
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(out).map_err(error)?;
    let out = fs::canonicalize(out).map_err(error)?;
    let executable = std::env::current_exe().map_err(error)?;
    let mut outcomes = Vec::new();
    let mut failure = None;
    for kind in ["normal", "interrupted", "ongoing"] {
        let dir = out.join(kind);
        builder.create(&dir).map_err(error)?;
        let mut topology = Topology::new(&dir)?;
        let mut server = None;
        let result = case(&dir, kind, &executable, &mut topology, &mut server);
        let process = server
            .as_mut()
            .map(|s| s.finish(result.is_ok()))
            .transpose();
        let cleanup = topology.cleanup();
        let process_ok = process.as_ref().is_ok_and(|p| {
            p.as_ref()
                .is_none_or(|p| p["errors"].as_array().is_some_and(Vec::is_empty))
        });
        let passed = result.is_ok() && process_ok && cleanup.is_ok();
        let receipt = json!({"case":kind,"topology":{"namespace":topology.namespace,"host":topology.host,"peer":topology.peer,"local":LOCAL,"remote":REMOTE,"port":443,"commands":topology.records},"observation":result.as_ref().ok(),"error":result.as_ref().err(),"server":process.as_ref().ok(),"server_error":process.as_ref().err(),"cleanup_success":cleanup.is_ok(),"cleanup_error":cleanup.err(),"passed":passed});
        let mut f = new_file(&dir.join("fixture.json"))?;
        serde_json::to_writer_pretty(&mut f, &receipt).map_err(error)?;
        f.write_all(b"\n").map_err(error)?;
        f.sync_all().map_err(error)?;
        outcomes.push(receipt);
        if !passed {
            failure = Some(format!(
                "{kind} isolated Ethernet qualification failed; inspect fixture.json"
            ));
            break;
        }
    }
    let receipt = json!({"schema":"spiralcore.capture-qualification.v2","scope":"owned Linux veth/network-namespace lifecycle qualification only; synthetic plaintext TCP, not WAN throughput or TLS evidence","controller_sha256":digest(&executable,64*1024*1024)?,"cases":outcomes,"passed":failure.is_none()});
    let mut f = new_file(&out.join("qualification.json"))?;
    serde_json::to_writer_pretty(&mut f, &receipt).map_err(error)?;
    f.write_all(b"\n").map_err(error)?;
    f.sync_all().map_err(error)?;
    if let Some(e) = failure {
        Err(e)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn successful_creation_remains_owned_when_receipt_writing_fails() {
        let dir = std::env::temp_dir().join(format!(
            "spiralcore-creation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let mut topology = Topology::new(&dir).unwrap();
        topology.log = File::open(dir.join("topology-commands.jsonl")).unwrap();
        assert!(topology
            .tool_with_creation("/usr/bin/true", &[], false, Some(Creation::Namespace))
            .is_err());
        assert!(topology.namespace_created);
        drop(topology);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn privileged_deadline_wraps_the_actual_tool_without_foreground_escape() {
        let command = tool_command(IP, &["link", "show"], true);
        let args = command
            .get_args()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "-n",
                "/usr/bin/timeout",
                "--signal=TERM",
                "--kill-after=1s",
                "5s",
                IP,
                "link",
                "show"
            ]
        );
        assert!(!args.contains(&"--foreground"));
    }
    #[test]
    fn fixture_tool_output_is_bounded_before_collection_finishes() {
        let shared = Mutex::new(Log::default());
        collect_tool_log(
            std::io::repeat(b'x').take((TOOL_LIMIT + 4096) as u64),
            &shared,
        );
        assert_eq!(shared.lock().unwrap().bytes.len(), TOOL_LIMIT);
        assert!(Pipes::text(&shared).is_err());
    }
    #[test]
    fn fixture_collision_check_accepts_default_route_but_rejects_overlaps() {
        let addresses = json!([{"ifname":"eth0","addr_info":[{"family":"inet","local":"10.0.0.4","prefixlen":24}]}]);
        let routes = json!([{"dst":"default","gateway":"10.0.0.1"}]);
        assert!(check_collisions(&addresses, &routes).is_ok());
        for dst in [
            "198.18.0.0/30",
            "198.18.0.2/32",
            "198.18.0.0/15",
            "198.18.0.1",
        ] {
            assert!(check_collisions(&addresses, &json!([{"dst":dst}])).is_err());
        }
        assert!(check_collisions(
            &json!([{"addr_info":[{"family":"inet","local":"198.18.0.1","prefixlen":30}]}]),
            &routes
        )
        .is_err());
        assert!(check_collisions(&addresses, &json!([{"dst":"broken"}])).is_err());
    }
    #[test]
    fn all_required_offloads_must_be_explicitly_disabled() {
        let text = "Features for scqhost:\ntcp-segmentation-offload: off\ngeneric-segmentation-offload: off\ngeneric-receive-offload: off\nlarge-receive-offload: off [fixed]\n";
        assert!(offloads_disabled(text).is_ok());
        assert!(offloads_disabled(&text.replace(
            "generic-receive-offload: off",
            "generic-receive-offload: on"
        ))
        .is_err());
        assert!(
            offloads_disabled(&text.replace("large-receive-offload: off [fixed]\n", "")).is_err()
        );
        assert!(offloads_disabled(&(text.to_owned() + "generic-receive-offload: off\n")).is_err());
    }
}
