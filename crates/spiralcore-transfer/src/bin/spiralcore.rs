//! Loopback-only experimental harness. The fixed policy below is not peer authentication.
use spiralcore_transfer::{
    client::{self, Stats},
    dag::Graph,
    endpoint::{Authority, Context, Endpoint, Limits},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    Error, Id, Result, MAX_FRAME, MAX_OBJECT,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const DEADLINE: Duration = Duration::from_secs(30);
const MAX_NODES: usize = 262144;
type CliResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
fn context() -> Context {
    Context {
        tenant: "spiralcore-local-demo".into(),
        domain: "public-synthetic-only".into(),
        sender: "loopback-sender".into(),
        recipient: "loopback-cache".into(),
        mailbox: "local-object-test".into(),
        capability: "fixed-demo-policy-not-a-secret".into(),
        authorization_scope: "local-experimental-objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
struct DemoAuthority;
impl Authority for DemoAuthority {
    fn authorize(&self, c: &Context) -> Result<()> {
        if *c == context() {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR {error}");
        std::process::exit(2);
    }
}
fn usage() {
    println!(
        "SpiralCore loopback experiment — public/synthetic data only; no peer authentication\n\
serve --cache PATH [--bind 127.0.0.1:PORT] [--sessions N]\n\
send --addr LOOPBACK:PORT --file PATH [--base PATH | --mode raw|zstd]\n\
send --addr LOOPBACK:PORT --file PATH --base PATH --mode sparse\n\
get --cache PATH --object HEX --out NEW_PATH\n\
demo --cache PATH\n\
Loopback IP literals only. Stop serve before get: the cache has one exclusive owner.\n\
Sparse base and target are limited to 16 MiB. CLI frames use sequential receipts;\n\
the WAN HTTP adapter batches requests. This CLI does not pipeline frames."
    );
}
fn options() -> CliResult<(String, BTreeMap<String, OsString>)> {
    let mut args = std::env::args_os().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".into());
    let command = command.into_string().map_err(|_| "command must be UTF-8")?;
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        let key = key.into_string().map_err(|_| "option must be UTF-8")?;
        if !key.starts_with("--") {
            return Err("expected --option value".into());
        }
        let value = args.next().ok_or("missing option value")?;
        if options.insert(key, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    Ok((command, options))
}
fn required(options: &mut BTreeMap<String, OsString>, key: &str) -> CliResult<OsString> {
    options
        .remove(key)
        .ok_or_else(|| format!("missing {key}").into())
}
fn no_extra(options: &BTreeMap<String, OsString>) -> CliResult<()> {
    if let Some(key) = options.keys().next() {
        Err(format!("unknown or unused option {key}").into())
    } else {
        Ok(())
    }
}
fn loopback(value: &std::ffi::OsStr) -> CliResult<SocketAddr> {
    let addr: SocketAddr = value.to_str().ok_or("address must be UTF-8")?.parse()?;
    if !addr.ip().is_loopback() {
        return Err(
            "only loopback IP addresses are allowed by this unauthenticated harness".into(),
        );
    }
    Ok(addr)
}
fn run() -> CliResult<()> {
    let (command, mut options) = options()?;
    match command.as_str() {
        "--help" | "help" => {
            no_extra(&options)?;
            usage();
        }
        "serve" => {
            let cache = required(&mut options, "--cache")?;
            let bind = options
                .remove("--bind")
                .unwrap_or_else(|| "127.0.0.1:0".into());
            let addr = loopback(&bind)?;
            let sessions = options
                .remove("--sessions")
                .map(|n| -> CliResult<usize> {
                    let n: usize = n.to_str().ok_or("sessions must be an integer")?.parse()?;
                    if n == 0 {
                        return Err("sessions must be positive".into());
                    }
                    Ok(n)
                })
                .transpose()?;
            no_extra(&options)?;
            let endpoint = Endpoint::open(Path::new(&cache), Limits::default(), DemoAuthority)?;
            let listener = TcpListener::bind(addr)?;
            eprintln!("Local public/synthetic experiment; fixed policy, no encryption or peer authentication.");
            println!("READY addr={}", listener.local_addr()?);
            std::io::stdout().flush()?;
            serve(listener, endpoint, sessions)?;
        }
        "send" => {
            let addr = loopback(&required(&mut options, "--addr")?)?;
            let file = required(&mut options, "--file")?;
            let base = options.remove("--base");
            let explicit_mode = options.remove("--mode");
            let mode_text = explicit_mode
                .as_deref()
                .map(|v| v.to_str().ok_or("mode must be UTF-8"))
                .transpose()?;
            let mode = match (mode_text, base.is_some()) {
                (None, true) => Mode::Dag,
                (None | Some("raw"), false) => Mode::Raw,
                (Some("zstd"), false) => Mode::Zstd,
                (Some("sparse"), true) => Mode::SparseV1,
                (Some("sparse"), false) => return Err("--mode sparse requires --base PATH".into()),
                (Some("raw" | "zstd"), true) => {
                    return Err("--base selects DAG unless --mode sparse is explicit".into())
                }
                _ => return Err("--mode must be raw, zstd or sparse".into()),
            };
            no_extra(&options)?;
            let bytes = read_bounded(Path::new(&file))?;
            let stats = if mode == Mode::SparseV1 {
                let base = read_bounded(Path::new(base.as_ref().ok_or(Error::Malformed)?))?;
                send_sparse(addr, &base, &bytes)?
            } else {
                let base = base
                    .map(|path| {
                        read_bounded(Path::new(&path))
                            .and_then(|bytes| Graph::fold(&bytes, MAX_NODES))
                    })
                    .transpose()?;
                send(addr, &bytes, mode, base.as_ref())?
            };
            print_stats("send", bytes.len(), &stats);
        }
        "get" => {
            let cache = required(&mut options, "--cache")?;
            let object = parse_id(&required(&mut options, "--object")?)?;
            let out = required(&mut options, "--out")?;
            no_extra(&options)?;
            let mut endpoint = Endpoint::open(Path::new(&cache), Limits::default(), DemoAuthority)?;
            let bytes = endpoint.read_object(&context(), object)?;
            let mut file = OpenOptions::new().write(true).create_new(true).open(&out)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            println!(
                "{{\"event\":\"get\",\"object\":\"{}\",\"object_bytes\":{}}}",
                hex(&object),
                bytes.len()
            );
        }
        "demo" => {
            let cache = required(&mut options, "--cache")?;
            no_extra(&options)?;
            demo(Path::new(&cache))?;
        }
        _ => return Err("unknown command; run spiralcore --help".into()),
    }
    Ok(())
}
fn parse_id(value: &std::ffi::OsStr) -> CliResult<Id> {
    let value = value
        .to_str()
        .ok_or("object id must be ASCII hexadecimal")?;
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("object id must be exactly 64 hexadecimal characters".into());
    }
    let mut id = [0; 32];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * i..2 * i + 2], 16)?;
    }
    Ok(id)
}
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    if size > MAX_OBJECT {
        return Err(Error::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size as usize)
        .map_err(|_| Error::Limit)?;
    let mut buffer = [0; 65536];
    loop {
        let remaining = MAX_OBJECT as usize + 1 - bytes.len();
        let count = file.read(&mut buffer[..remaining.min(65536)])?;
        if count == 0 {
            return Ok(bytes);
        }
        bytes.try_reserve_exact(count).map_err(|_| Error::Limit)?;
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() as u64 > MAX_OBJECT {
            return Err(Error::Limit);
        }
    }
}
fn configure(stream: &TcpStream) -> Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(DEADLINE))?;
    stream.set_write_timeout(Some(DEADLINE))?;
    Ok(())
}
fn time_left(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Error::Expired)
}
fn read_until(stream: &mut TcpStream, buffer: &mut [u8], deadline: Instant) -> Result<usize> {
    loop {
        stream.set_read_timeout(Some(time_left(deadline)?))?;
        match stream.read(buffer) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            result => return result.map_err(Error::from),
        }
    }
}
fn read_exact_until(
    stream: &mut TcpStream,
    mut buffer: &mut [u8],
    deadline: Instant,
) -> Result<()> {
    while !buffer.is_empty() {
        let count = read_until(stream, buffer, deadline)?;
        if count == 0 {
            return Err(Error::Malformed);
        }
        buffer = &mut buffer[count..];
    }
    Ok(())
}
fn write_all_until(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> Result<()> {
    while !bytes.is_empty() {
        stream.set_write_timeout(Some(time_left(deadline)?))?;
        match stream.write(bytes) {
            Ok(0) => return Err(Error::Storage),
            Ok(count) => bytes = &bytes[count..],
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
fn read_frame(stream: &mut TcpStream) -> Result<Option<(Frame, usize)>> {
    let deadline = Instant::now() + DEADLINE;
    let mut length = [0u8; 4];
    if read_until(stream, &mut length[..1], deadline)? == 0 {
        return Ok(None);
    }
    read_exact_until(stream, &mut length[1..], deadline)?;
    let length = u32::from_le_bytes(length) as usize;
    if !(56..=MAX_FRAME).contains(&length) {
        return Err(Error::Limit);
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| Error::Limit)?;
    bytes.resize(length, 0);
    read_exact_until(stream, &mut bytes, deadline)?;
    Ok(Some((Frame::decode(&bytes)?, length + 4)))
}
fn write_frame(stream: &mut TcpStream, frame: &Frame) -> Result<usize> {
    let bytes = frame.encode()?;
    if bytes.len() > MAX_FRAME {
        return Err(Error::Limit);
    }
    let deadline = Instant::now() + DEADLINE;
    write_all_until(stream, &(bytes.len() as u32).to_le_bytes(), deadline)?;
    write_all_until(stream, &bytes, deadline)?;
    stream.flush()?;
    Ok(bytes.len() + 4)
}
fn serve(
    listener: TcpListener,
    mut endpoint: Endpoint<DemoAuthority>,
    sessions: Option<usize>,
) -> Result<()> {
    let start = Instant::now();
    let mut accepted = 0usize;
    while sessions.is_none_or(|n| accepted < n) {
        let (mut stream, peer) = listener.accept()?;
        accepted += 1;
        if !peer.ip().is_loopback() {
            continue;
        }
        configure(&stream)?;
        let session = (|| -> Result<()> {
            while let Some((request, _)) = read_frame(&mut stream)? {
                let reply = match endpoint.handle(&context(), &request, start.elapsed().as_secs()) {
                    Ok(reply) => reply,
                    Err(error) => request.reply(127, (error as u16).to_le_bytes().to_vec()),
                };
                write_frame(&mut stream, &reply)?;
            }
            Ok(())
        })();
        if let Err(error) = session {
            eprintln!("SESSION_ERROR {error}");
        }
    }
    Ok(())
}
struct TransferStats {
    stats: Stats,
    socket_request_bytes: u64,
    socket_response_bytes: u64,
}
fn send(addr: SocketAddr, bytes: &[u8], mode: Mode, base: Option<&Graph>) -> Result<TransferStats> {
    send_with(addr, |transfer_id, exchange| {
        client::transfer(bytes, mode, base, transfer_id, exchange)
    })
}
fn send_sparse(addr: SocketAddr, base: &[u8], target: &[u8]) -> Result<TransferStats> {
    if base.len() > client::MAX_BATCHED_OBJECT || target.len() > client::MAX_BATCHED_OBJECT {
        return Err(Error::Limit);
    }
    send_with(addr, |transfer_id, exchange| {
        // This local stream adapter is deliberately sequential. Begin is probed
        // separately, so every successful batch uploads all planned frames and
        // its accounting matches the actual socket traffic. HTTP WAN batching
        // is provided by the separate HttpClient adapter.
        client::transfer_sparse_batched(base, target, transfer_id, true, |frames| {
            let mut replies = Vec::with_capacity(frames.len());
            for frame in frames {
                let reply = exchange(frame)?;
                let terminal = matches!(reply.kind, 103 | 127);
                replies.push(reply);
                if terminal {
                    break;
                }
            }
            Ok(replies)
        })
    })
}
fn send_with(
    addr: SocketAddr,
    perform: impl FnOnce(Id, &mut dyn FnMut(&Frame) -> Result<Frame>) -> Result<Stats>,
) -> Result<TransferStats> {
    if !addr.ip().is_loopback() {
        return Err(Error::Unauthorized);
    }
    let mut stream = TcpStream::connect_timeout(&addr, DEADLINE)?;
    configure(&stream)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Storage)?
        .as_nanos();
    let transfer_id = hash(
        b"spiralcore.local.demo.attempt.v1\0",
        &[
            &std::process::id().to_le_bytes(),
            &nonce.to_le_bytes(),
            &NEXT.fetch_add(1, Ordering::Relaxed).to_le_bytes(),
        ],
    );
    let mut socket_request_bytes = 0;
    let mut socket_response_bytes = 0;
    let stats = perform(transfer_id, &mut |request| {
        socket_request_bytes += write_frame(&mut stream, request)? as u64;
        let (reply, count) = read_frame(&mut stream)?.ok_or(Error::Malformed)?;
        socket_response_bytes += count as u64;
        Ok(reply)
    })?;
    if socket_request_bytes != stats.request_bytes + 4 * stats.requests as u64
        || socket_response_bytes != stats.response_bytes + 4 * stats.requests as u64
    {
        return Err(Error::Integrity);
    }
    Ok(TransferStats {
        stats,
        socket_request_bytes,
        socket_response_bytes,
    })
}
fn print_stats(label: &str, object_bytes: usize, report: &TransferStats) {
    let s = &report.stats;
    let mode = match s.mode {
        Mode::Raw => "raw",
        Mode::Zstd => "zstd",
        Mode::Dag => "dag",
        Mode::SparseV1 => "sparse-v1",
        Mode::ZstdPrefixV1 => "zstd-prefix-v1",
        Mode::RuntimeV1 => "runtime-v1",
    };
    println!("{{\"event\":\"transfer\",\"label\":\"{label}\",\"object\":\"{}\",\"object_bytes\":{object_bytes},\"mode\":\"{mode}\",\"request_bytes\":{},\"response_bytes\":{},\"data_bytes\":{},\"records_sent\":{},\"requests\":{},\"elapsed_micros\":{},\"socket_request_bytes\":{},\"socket_response_bytes\":{},\"socket_frame_bytes\":{}}}",
        hex(&s.object), s.request_bytes, s.response_bytes, s.data_bytes, s.records_sent, s.requests,
        s.elapsed_micros, report.socket_request_bytes, report.socket_response_bytes, report.socket_request_bytes + report.socket_response_bytes);
}
fn demo(cache: &Path) -> CliResult<()> {
    if cache.exists() && std::fs::read_dir(cache)?.next().is_some() {
        return Err("demo requires a new or empty cache directory so the cold measurement is cold; choose a new --cache path".into());
    }
    let endpoint = Endpoint::open(cache, Limits::default(), DemoAuthority)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let server = std::thread::spawn(move || serve(listener, endpoint, Some(3)));
    let mut state = 0x6a09e667f3bcc909u64;
    let bytes: Vec<u8> = (0..1 << 20)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    let base = Graph::fold(&bytes, MAX_NODES)?;
    let mut mutation = bytes.clone();
    for byte in &mut mutation[524288..524320] {
        *byte ^= 0xa5;
    }
    let cold = send(addr, &bytes, Mode::Raw, None)?;
    print_stats("cold", bytes.len(), &cold);
    let repeat = send(addr, &bytes, Mode::Dag, Some(&base))?;
    print_stats("repeat", bytes.len(), &repeat);
    let delta = send(addr, &mutation, Mode::Dag, Some(&base))?;
    print_stats("mutation", mutation.len(), &delta);
    server.join().map_err(|_| Error::Storage)??;
    let mut endpoint = Endpoint::open(cache, Limits::default(), DemoAuthority)?;
    for expected in [&bytes, &bytes, &mutation] {
        if endpoint.read_object(&context(), object_id(expected))? != *expected {
            return Err(Error::Integrity.into());
        }
    }
    println!("{{\"event\":\"demo_complete\",\"restart_verified\":true,\"transfers\":3,\"unique_objects\":2,\"transport\":\"tcp_loopback\"}}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_deadline_does_not_restart_after_a_partial_read() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(&[1]).unwrap();
            std::thread::sleep(Duration::from_millis(100));
            let _ = stream.write_all(&[2]);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        let result = read_exact_until(
            &mut stream,
            &mut [0; 2],
            Instant::now() + Duration::from_millis(25),
        );
        assert!(
            result.is_err(),
            "a later byte must not extend the frame deadline"
        );
        server.join().unwrap();
    }
}
