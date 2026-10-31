//! Persistent public-fixture transfers through real canonical mailbox streams.
use crate::{
    Result,
    config::{Arm, Bootstrap, Plan, Published, Role},
    fixtures::{self, Fixture},
};
use cpu_time::ProcessTime;
use mvm_core::mailbox::transport::CancellationToken;
use serde_json::{Value, json};
use spiralcore_mvm_adapter::{
    Direction, Error as AdapterError, MailboxAuthority,
    live::{HandshakeRole, LiveError, SocketIo, StreamCounters, StreamTransport},
};
use spiralcore_transfer::{
    self as transfer, Id,
    client::{self, Stats},
    endpoint::{ColdGraphPolicy, Context, Endpoint, Limits},
    protocol::{Frame, Mode},
    schema,
    session::{RuntimeSession, WarmSession},
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    zstd_whole,
};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    time::{Duration, Instant},
};

const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE: Id = [0; 32];

#[derive(Default)]
struct Meter {
    cpu_us: u128,
    wall_us: u128,
}
impl Meter {
    fn add(&mut self, cpu: u128, wall: u128) -> Result<()> {
        self.cpu_us = self.cpu_us.checked_add(cpu).ok_or("CPU counter overflow")?;
        self.wall_us = self
            .wall_us
            .checked_add(wall)
            .ok_or("wall counter overflow")?;
        Ok(())
    }
}
fn emit(out: &mut impl Write, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    Ok(())
}
fn recipe() -> Recipe {
    Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    }
}
fn cases(plan: &Plan) -> Result<Vec<Fixture>> {
    let mut cases = fixtures::objects(plan.size, plan.trial)?;
    if plan.arm == Arm::Warm {
        let artifact = schema::fit(&[b"{\"value\":0,\"message\":\"training\"}\n"])?;
        cases.insert(
            1,
            Fixture {
                kind: "schema",
                bytes: artifact,
            },
        );
        let checkpoint = State::seeded(48)?.encode()?;
        let prediction = PreparedPrediction::new(&checkpoint, &recipe())?;
        let generated = prediction.prediction().to_vec();
        let mut residual = generated.clone();
        let middle = residual.len() / 2;
        for byte in residual
            .get_mut(middle..middle + 32)
            .ok_or("runtime fixture too small")?
        {
            *byte ^= 0xa5;
        }
        cases.extend([
            Fixture {
                kind: "runtime-checkpoint",
                bytes: checkpoint,
            },
            Fixture {
                kind: "runtime-generated",
                bytes: generated,
            },
            Fixture {
                kind: "runtime-residual",
                bytes: residual,
            },
        ]);
    }
    Ok(cases)
}
fn transfer_id(plan: &Plan, position: usize) -> Result<Id> {
    let mut id = [0; 32];
    id[..8].copy_from_slice(
        &u64::try_from(position.checked_add(1).ok_or("position overflow")?)?.to_le_bytes(),
    );
    id[8..16].copy_from_slice(&plan.trial.to_le_bytes());
    id[16..24].copy_from_slice(&u64::try_from(plan.size)?.to_le_bytes());
    Ok(id)
}
fn difference(after: StreamCounters, before: StreamCounters) -> Result<StreamCounters> {
    let sub = |a: u64, b: u64| a.checked_sub(b).ok_or("stream counter rollback");
    Ok(StreamCounters {
        envelope_bytes_sent: sub(after.envelope_bytes_sent, before.envelope_bytes_sent)?,
        envelope_bytes_received: sub(
            after.envelope_bytes_received,
            before.envelope_bytes_received,
        )?,
        framing_bytes_sent: sub(after.framing_bytes_sent, before.framing_bytes_sent)?,
        framing_bytes_received: sub(after.framing_bytes_received, before.framing_bytes_received)?,
        frames_sent: sub(after.frames_sent, before.frames_sent)?,
        frames_received: sub(after.frames_received, before.frames_received)?,
    })
}

struct Client<S: SocketIo> {
    outgoing: Direction,
    incoming: Direction,
    stream: StreamTransport<S>,
    cancel: CancellationToken,
    exchange: Meter,
}
impl<S: SocketIo + 'static> Client<S> {
    fn call(&mut self, request: &Frame) -> Result<(Context, Frame)> {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let call = self.outgoing.send_call(
            request,
            &self.incoming,
            &self.stream,
            &self.cancel,
            Instant::now() + CALL_TIMEOUT,
        )?;
        let context = call.context().clone();
        let wire = self.stream.receive(&self.cancel, call.deadline())?;
        let reply = self.incoming.open_live(&wire)?;
        let reply = self.incoming.accept_reply(call, reply)?;
        self.exchange
            .add(cpu.elapsed().as_micros(), wall.elapsed().as_micros())?;
        Ok((context, reply))
    }
    fn exchange(&mut self, frames: &[Frame]) -> Result<Vec<Frame>> {
        if frames.is_empty() || frames.len() > 4096 {
            return Err("invalid frame batch".into());
        }
        let mut replies = Vec::with_capacity(frames.len());
        for frame in frames {
            let (_, reply) = self.call(frame)?;
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        Ok(replies)
    }
}
enum Sender {
    Raw,
    Whole(zstd_whole::PreparedEncoder),
    Warm(Box<WarmSession>),
}

pub fn sender<S: SocketIo + 'static>(
    bootstrap: &Bootstrap,
    socket: S,
    role: HandshakeRole,
    out: &mut impl Write,
) -> Result<Value> {
    if bootstrap.role != Role::Sender {
        return Err("sender bootstrap role mismatch".into());
    }
    let cpu = ProcessTime::now();
    let wall = Instant::now();
    let cases = cases(&bootstrap.plan)?;
    let outgoing = bootstrap
        .outgoing
        .as_ref()
        .ok_or("missing send material")?
        .direction()?;
    let incoming = bootstrap
        .incoming
        .as_ref()
        .ok_or("missing reply material")?
        .direction()?;
    let stream = StreamTransport::new(socket, outgoing.route_id(), CALL_TIMEOUT)?;
    let cancel = CancellationToken::default();
    stream.handshake(role, &cancel)?;
    let mut client = Client {
        outgoing,
        incoming,
        stream,
        cancel,
        exchange: Meter::default(),
    };
    let (context, probe) = client.call(&Frame {
        kind: 5,
        transfer: PROBE,
        seq: 0,
        body: Vec::new(),
    })?;
    if probe.kind != 127 || probe.body != (transfer::Error::Unknown as u16).to_le_bytes() {
        return Err("unexpected initial Status probe outcome".into());
    }
    let mut sender = match bootstrap.plan.arm {
        Arm::Raw => Sender::Raw,
        Arm::Whole => Sender::Whole(zstd_whole::PreparedEncoder::new()?),
        Arm::Warm => Sender::Warm(Box::new(WarmSession::new(
            &context,
            &cases
                .iter()
                .find(|case| case.kind == "schema")
                .ok_or("missing schema")?
                .bytes,
        )?)),
    };
    let mut runtime = if bootstrap.plan.arm == Arm::Warm {
        let checkpoint = &cases
            .iter()
            .find(|case| case.kind == "runtime-checkpoint")
            .ok_or("missing checkpoint")?
            .bytes;
        Some(RuntimeSession::new(
            &context,
            PreparedPrediction::new(checkpoint, &recipe())?,
        )?)
    } else {
        None
    };
    let mut total = Meter::default();
    total.add(cpu.elapsed().as_micros(), wall.elapsed().as_micros())?;
    emit(
        out,
        &json!({"event":"sender_setup","plan":bootstrap.plan,"cpu_us":total.cpu_us,
        "wall_us":total.wall_us,"counters":client.stream.counters()?,
        "includes":"fixture generation, codec preparation, handshake and authenticated Status probe",
        "seed_baseline":"all arms seed raw; Whole uses prepared whole-object zstd for subsequent objects"}),
    )?;
    for (position, case) in cases.iter().enumerate() {
        let expected = transfer::object_id(&case.bytes);
        let owned = case.bytes.clone();
        let id = transfer_id(&bootstrap.plan, position)?;
        let before = client.stream.counters()?;
        let exchange_cpu = client.exchange.cpu_us;
        let exchange_wall = client.exchange.wall_us;
        let mut failure = None;
        let mut exchange = |frames: &[Frame]| match client.exchange(frames) {
            Ok(replies) => Ok(replies),
            Err(error) => {
                failure = Some(error);
                Err(transfer::Error::Storage)
            }
        };
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let result = match (&mut sender, case.kind) {
            (_, "runtime-checkpoint") => runtime
                .as_mut()
                .ok_or("missing runtime session")?
                .seed_checkpoint(&context, id, &mut exchange),
            (_, "runtime-generated" | "runtime-residual") => runtime
                .as_mut()
                .ok_or("missing runtime session")?
                .send(&context, owned, id, false, &mut exchange)
                .map(|stats| stats.transfer),
            (Sender::Warm(sender), "seed") => sender.seed_base(&context, owned, id, &mut exchange),
            (Sender::Warm(sender), "schema") => sender.seed_schema(&context, id, &mut exchange),
            (Sender::Warm(sender), _) => sender
                .send(&context, owned, id, false, &mut exchange)
                .map(|stats| stats.transfer),
            (Sender::Raw, _) | (Sender::Whole(_), "seed") => {
                client::transfer_batched(&owned, Mode::Raw, None, id, false, &mut exchange)
            }
            (Sender::Whole(sender), _) => client::transfer_prepared_zstd_whole_batched(
                sender,
                &owned,
                id,
                false,
                &mut exchange,
            ),
        };
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        let stats: Stats = match result {
            Ok(stats) => stats,
            Err(error) => return Err(failure.unwrap_or_else(|| Box::new(error))),
        };
        if stats.object != expected {
            return Err("sender publication identity mismatch".into());
        }
        let io_cpu = client
            .exchange
            .cpu_us
            .checked_sub(exchange_cpu)
            .ok_or("CPU counter rollback")?;
        let io_wall = client
            .exchange
            .wall_us
            .checked_sub(exchange_wall)
            .ok_or("wall counter rollback")?;
        total.add(cpu_us, wall_us)?;
        emit(
            out,
            &json!({"event":"sender_operation","plan":bootstrap.plan,"position":position,
            "kind":case.kind,"object":transfer::hex(&expected),"bytes":case.bytes.len(),"mode":format!("{:?}",stats.mode),
            "cpu_us":cpu_us,"wall_us":wall_us,"exchange_cpu_us":io_cpu,"exchange_wall_us":io_wall,
            "counters":difference(client.stream.counters()?, before)?,
            "application_request_bytes":stats.request_bytes,"application_response_bytes":stats.response_bytes}),
        )?;
    }
    let counters = client.stream.counters()?;
    client.stream.close()?;
    let result = json!({"event":"sender_done","plan":bootstrap.plan,"operations":cases.len(),
        "cpu_us":total.cpu_us,"wall_us":total.wall_us,"counters":counters,
        "timing":"setup plus operations; JSON output and fixture ownership copies excluded from operation timing"});
    emit(out, &result)?;
    Ok(result)
}

struct Server<S: SocketIo> {
    incoming: Direction,
    outgoing: Direction,
    stream: StreamTransport<S>,
    cancel: CancellationToken,
    endpoint: Endpoint<MailboxAuthority>,
    start: Instant,
}
struct Handled {
    reply: Frame,
    namespace: Id,
    meter: Meter,
}
impl<S: SocketIo + 'static> Server<S> {
    fn handle(&mut self, expected_transfer: Id) -> Result<Handled> {
        let wire = self
            .stream
            .receive(&self.cancel, Instant::now() + CALL_TIMEOUT)?;
        // Socket idle waiting is outside receiver processing timings.
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let verified = self.incoming.open_live(&wire)?;
        if verified.frame().transfer != expected_transfer {
            return Err("unexpected campaign transfer".into());
        }
        let namespace = verified.context().namespace()?;
        let reply = match verified.dispatch_live(&mut self.endpoint, self.start.elapsed().as_secs())
        {
            Ok(reply) => reply,
            Err(LiveError::Adapter(AdapterError::Frame(error))) => verified
                .frame()
                .reply(127, (error as u16).to_le_bytes().to_vec()),
            Err(error) => return Err(error.into()),
        };
        self.outgoing
            .reply_to_verified(&verified, &reply, &self.stream, &self.cancel)?;
        Ok(Handled {
            reply,
            namespace,
            meter: Meter {
                cpu_us: cpu.elapsed().as_micros(),
                wall_us: wall.elapsed().as_micros(),
            },
        })
    }
}

pub fn receiver<S: SocketIo + 'static>(
    bootstrap: &Bootstrap,
    cache_root: &Path,
    socket: S,
    role: HandshakeRole,
    out: &mut impl Write,
) -> Result<Value> {
    if bootstrap.role != Role::Receiver {
        return Err("receiver bootstrap role mismatch".into());
    }
    let cpu = ProcessTime::now();
    let wall = Instant::now();
    let cases = cases(&bootstrap.plan)?;
    let incoming = bootstrap
        .incoming
        .as_ref()
        .ok_or("missing receive material")?
        .direction()?;
    let outgoing = bootstrap
        .outgoing
        .as_ref()
        .ok_or("missing reply material")?
        .direction()?;
    let stream = StreamTransport::new(socket, outgoing.route_id(), CALL_TIMEOUT)?;
    let cancel = CancellationToken::default();
    let limits = Limits {
        max_active: 4,
        resident_bytes: 512 << 20,
        cache_bytes: 128 << 20,
        namespace_bytes: 128 << 20,
        ..Limits::default()
    };
    let mut endpoint = Endpoint::open_with_policy(
        cache_root,
        limits,
        incoming.authority(),
        ColdGraphPolicy::Disabled,
    )?;
    endpoint.set_codec_cache_enabled(true)?;
    stream.handshake(role, &cancel)?;
    let mut server = Server {
        incoming,
        outgoing,
        stream,
        cancel,
        endpoint,
        start: Instant::now(),
    };
    let probe = server.handle(PROBE)?;
    if probe.reply.kind != 127
        || probe.reply.body != (transfer::Error::Unknown as u16).to_le_bytes()
    {
        return Err("unexpected initial Status request".into());
    }
    let namespace = probe.namespace;
    let mut total = Meter::default();
    total.add(cpu.elapsed().as_micros(), wall.elapsed().as_micros())?;
    emit(
        out,
        &json!({"event":"receiver_setup","plan":bootstrap.plan,"cpu_us":total.cpu_us,
        "wall_us":total.wall_us,"counters":server.stream.counters()?}),
    )?;
    let mut published = Vec::with_capacity(cases.len());
    for (position, case) in cases.iter().enumerate() {
        let expected = transfer::object_id(&case.bytes);
        let before = server.stream.counters()?;
        let mut processing = Meter::default();
        let mut messages = 0usize;
        loop {
            messages = messages.checked_add(1).ok_or("message count overflow")?;
            if messages > 4096 {
                return Err("operation frame limit".into());
            }
            let handled = server.handle(transfer_id(&bootstrap.plan, position)?)?;
            processing.add(handled.meter.cpu_us, handled.meter.wall_us)?;
            if handled.namespace != namespace {
                return Err("campaign namespace changed".into());
            }
            if handled.reply.kind == 127 {
                return Err("object endpoint refused operation".into());
            }
            if handled.reply.kind == 103 {
                if handled.reply.body != expected {
                    return Err("receiver publication identity mismatch".into());
                }
                published.push(Published {
                    namespace,
                    object: expected,
                    length: case.bytes.len(),
                    kind: case.kind.into(),
                });
                break;
            }
        }
        total.add(processing.cpu_us, processing.wall_us)?;
        emit(
            out,
            &json!({"event":"receiver_operation","plan":bootstrap.plan,"position":position,
            "kind":case.kind,"object":transfer::hex(&expected),"bytes":case.bytes.len(),
            "cpu_us":processing.cpu_us,"wall_us":processing.wall_us,
            "counters":difference(server.stream.counters()?, before)?,
            "timing":"open, endpoint processing and authenticated reply; idle receive wait excluded"}),
        )?;
    }
    // Never close after merely writing the last ACK: the sender must read and
    // validate it, then close. A partial trailing prefix is not a clean EOF.
    let before_eof = server.stream.counters()?;
    match server
        .stream
        .receive(&server.cancel, Instant::now() + CALL_TIMEOUT)
    {
        Err(LiveError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {}
        _ => return Err("expected sender EOF after final publication reply".into()),
    }
    let counters = server.stream.counters()?;
    let tail = difference(counters, before_eof)?;
    if tail.envelope_bytes_received != 0 || tail.framing_bytes_received != 0 {
        return Err("trailing partial frame after campaign".into());
    }
    drop(server);
    drop(cases);
    verify(&bootstrap.plan, cache_root, &published, out)?;
    let result = json!({"event":"receiver_done","plan":bootstrap.plan,"operations":published.len(),
        "cpu_us":total.cpu_us,"wall_us":total.wall_us,"counters":counters,"published":published,
        "exact_reopened":true,"timing":"setup plus receiver processing; independent readback recorded separately"});
    emit(out, &result)?;
    Ok(result)
}

fn read_publication(root: &Path, published: &Published) -> Result<Vec<u8>> {
    if published.length > 16 << 20 {
        return Err("publication exceeds fixture bound".into());
    }
    let namespace = root.join(transfer::hex(&published.namespace));
    for directory in [root, namespace.as_path()] {
        let meta = fs::symlink_metadata(directory)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("unsafe cache directory".into());
        }
    }
    let path = namespace.join(format!("{}.object", transfer::hex(&published.object)));
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("unsafe publication path".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    let mut header = [0; 49];
    file.read_exact(&mut header)?;
    let length = u64::from_le_bytes(header[8..16].try_into()?);
    if &header[..8] != b"SCPUB001"
        || length != u64::try_from(published.length)?
        || header[16..48] != published.object
    {
        return Err("publication header mismatch".into());
    }
    let overhead = match header[48] {
        0 => 49,
        1 => {
            let mut manifest = [0; 32];
            file.read_exact(&mut manifest)?;
            81
        }
        _ => return Err("invalid publication manifest tag".into()),
    };
    if file.metadata()?.len() != length.checked_add(overhead).ok_or("length overflow")? {
        return Err("publication file length mismatch".into());
    }
    let mut bytes = vec![0; published.length];
    file.read_exact(&mut bytes)?;
    let mut extra = [0; 1];
    if file.read(&mut extra)? != 0 || transfer::object_id(&bytes) != published.object {
        return Err("publication integrity mismatch".into());
    }
    Ok(bytes)
}

/// Offline historical-namespace verification. No authority, keys, socket,
/// Endpoint ownership writer or staging cleanup is opened by this function.
pub fn verify(
    plan: &Plan,
    cache_root: &Path,
    published: &[Published],
    out: &mut impl Write,
) -> Result<()> {
    let cases = cases(plan)?;
    if cases.len() != published.len() {
        return Err("publication count mismatch".into());
    }
    let namespace = published.first().ok_or("empty publication list")?.namespace;
    for (position, (expected, publication)) in cases.iter().zip(published).enumerate() {
        if publication.namespace != namespace
            || publication.kind != expected.kind
            || publication.length != expected.bytes.len()
            || publication.object != transfer::object_id(&expected.bytes)
        {
            return Err("publication does not match registered fixture".into());
        }
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let bytes = read_publication(cache_root, publication)?;
        if bytes != expected.bytes {
            return Err("exact byte comparison failed".into());
        }
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        emit(
            out,
            &json!({"event":"verified","plan":plan,"position":position,"kind":expected.kind,
            "object":transfer::hex(&publication.object),"namespace":transfer::hex(&namespace),
            "bytes":bytes.len(),"exact":true,"cpu_us":cpu_us,"wall_us":wall_us,"read_only":true}),
        )?;
    }
    Ok(())
}
