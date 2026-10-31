//! Coarse external receiver attribution; production Endpoint is unchanged.
use cpu_time::ProcessTime;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    delta,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    hash, hex, object_id,
    protocol::{Frame, Mode, Offer},
    wan, Error, Id, MAX_CHUNK,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClockMode {
    OuterOnly,
    Stages,
}

impl ClockMode {
    fn label(self) -> &'static str {
        match self {
            Self::OuterOnly => "outer-only",
            Self::Stages => "stages",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Cost {
    cpu_ns: u64,
    wall_ns: u64,
}
fn elapsed(cpu: ProcessTime, wall: Instant) -> Result<Cost> {
    let cpu_ns = u64::try_from(cpu.try_elapsed()?.as_nanos())?;
    Ok(Cost {
        cpu_ns,
        wall_ns: u64::try_from(wall.elapsed().as_nanos())?,
    })
}
fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}
fn emit(out: &mut impl Write, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn fixture(len: usize, mut state: u64) -> Vec<u8> {
    state = state.max(1);
    let mut bytes = Vec::with_capacity(len);
    while bytes.len() < len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let chunk = state.to_le_bytes();
        bytes.extend_from_slice(&chunk[..(len - bytes.len()).min(8)]);
    }
    bytes
}
fn fixture_seed(len: usize, escape: bool) -> u64 {
    let domain: &[u8] = if escape {
        b"spiralcore.persistent-session.escape.v1\0"
    } else {
        b"spiralcore.warm-object.fixture.v1\0"
    };
    let digest = hash(domain, &[&0u64.to_le_bytes(), &[u8::from(len == 16 << 20)]]);
    u64::from_le_bytes(digest[..8].try_into().expect("fixed digest prefix")).max(1)
}
fn next_target(previous: &[u8], update: usize) -> Result<Vec<u8>> {
    if !(1..=10).contains(&update) || previous.len() < 2048 {
        return Err("invalid diagnostic fixture bounds".into());
    }
    if update == 5 {
        return Ok(fixture(previous.len(), fixture_seed(previous.len(), true)));
    }
    let edit = if update < 5 { update - 1 } else { update - 6 };
    let start = previous.len() / 2 + edit * 64;
    let mut target = previous.to_vec();
    for byte in &mut target[start..start + 32] {
        *byte ^= 0xa5;
    }
    Ok(target)
}
fn prepare_frames(base: Option<&[u8]>, target: &[u8], transfer: Id) -> Result<Vec<Frame>> {
    let recipe = base
        .map(|b| delta::encode(b, target))
        .transpose()?
        .flatten();
    let (mode, bodies, base_id) = if let Some(recipe) = recipe {
        (Mode::SparseV1, recipe.frames, base.map(object_id))
    } else {
        let bodies = target
            .chunks(MAX_CHUNK)
            .map(|chunk| {
                let mut b = Vec::with_capacity(chunk.len() + 5);
                b.push(0);
                b.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
                b.extend_from_slice(chunk);
                b
            })
            .collect();
        (Mode::Raw, bodies, None)
    };
    let offer = Offer {
        mode,
        len: target.len() as u64,
        object: object_id(target),
        manifest: None,
        base: base_id,
    };
    let mut frames = vec![Frame {
        kind: 1,
        transfer,
        seq: 0,
        body: offer.encode()?,
    }];
    for body in bodies {
        frames.push(Frame {
            kind: 2,
            transfer,
            seq: frames.len() as u64,
            body,
        });
    }
    frames.push(Frame {
        kind: 3,
        transfer,
        seq: frames.len() as u64,
        body: vec![],
    });
    Ok(frames)
}
fn context() -> Context {
    Context {
        tenant: "receiver-stages-public-fixture".into(),
        domain: "synthetic".into(),
        sender: "diagnostic-sender".into(),
        recipient: "diagnostic-receiver".into(),
        mailbox: "receiver-stages".into(),
        capability: "public-fixture-only".into(),
        authorization_scope: "receiver-stages".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
struct Policy(Context);
impl Authority for Policy {
    fn authorize(&self, c: &Context) -> spiralcore_transfer::Result<()> {
        if c == &self.0 {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn open(root: &Path) -> spiralcore_transfer::Result<Endpoint<Policy>> {
    Endpoint::open_with_policy(
        root,
        Limits {
            graph_cache_bytes: 0,
            graph_cache_entries: 0,
            ..Limits::default()
        },
        Policy(context()),
        ColdGraphPolicy::Disabled,
    )
}
fn expected_replies(frames: &[Frame]) -> Result<Vec<Frame>> {
    let offer = Offer::decode(&frames[0].body)?;
    let mut ready = vec![offer.mode as u8, u8::from(offer.base.is_some())];
    ready.extend_from_slice(&1u64.to_le_bytes());
    let mut replies = vec![frames[0].reply(101, ready)];
    let mut accepted = 0u64;
    for frame in &frames[1..frames.len() - 1] {
        accepted = accepted
            .checked_add(u32::from_le_bytes(frame.body[1..5].try_into()?) as u64)
            .ok_or("accepted overflow")?;
        replies.push(frame.reply(102, accepted.to_le_bytes().to_vec()));
    }
    replies.push(
        frames
            .last()
            .ok_or("missing finish")?
            .reply(103, offer.object.to_vec()),
    );
    Ok(replies)
}

// Three coarse handler groups. Reply comparison and record output happen later.
struct Dispatch {
    replies: Vec<Frame>,
    outer: Cost,
    stage_clocks: Option<Value>,
    error: Option<String>,
}
fn dispatch(
    endpoint: &mut Endpoint<Policy>,
    c: &Context,
    frames: &[Frame],
    mode: ClockMode,
    now: u64,
) -> Result<Dispatch> {
    let mut replies = Vec::with_capacity(frames.len());
    let mut stages = Vec::with_capacity(3);
    let groups = [
        &frames[..1],
        &frames[1..frames.len() - 1],
        &frames[frames.len() - 1..],
    ];
    let mut error = None;
    let outer_wall = Instant::now();
    let outer_cpu = ProcessTime::try_now()?;
    for group in groups {
        let timer = if mode == ClockMode::Stages {
            Some((Instant::now(), ProcessTime::try_now()?))
        } else {
            None
        };
        for frame in group {
            match endpoint.handle(c, frame, now) {
                Ok(reply) => replies.push(reply),
                Err(e) => {
                    error = Some(format!("{e:?}"));
                    break;
                }
            }
        }
        if let Some((wall, cpu)) = timer {
            stages.push(elapsed(cpu, wall)?);
        }
        if error.is_some() {
            break;
        }
    }
    let outer = elapsed(outer_cpu, outer_wall)?;
    let stage_clocks = if mode == ClockMode::Stages {
        Some(
            json!({"begin":stages.first(),"data":stages.get(1),"finish":stages.get(2),"groups":stages.len()}),
        )
    } else {
        None
    };
    Ok(Dispatch {
        replies,
        outer,
        stage_clocks,
        error,
    })
}

fn run_sequence(root: &Path, len: usize, mode: ClockMode, out: &mut impl Write) -> Result<usize> {
    match fs::symlink_metadata(root) {
        Ok(_) => return Err("diagnostic store must not exist".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    if !(2048..=16 << 20).contains(&len) {
        return Err("diagnostic object limit".into());
    }
    let c = context();
    let mut endpoint = open(root)?;
    let mut previous = fixture(len, fixture_seed(len, false));
    let mut seen = std::collections::BTreeSet::new();
    emit(
        out,
        &json!({"event":"sequence","bytes":len,"clock_mode":mode.label(),"root":root,"updates":10,"escape_update":5,"base_seed":fixture_seed(len,false),"escape_seed":fixture_seed(len,true)}),
    )?;
    for position in 0..=10 {
        let target = if position == 0 {
            previous.clone()
        } else {
            next_target(&previous, position)?
        };
        let id = object_id(&target);
        if !seen.insert(id) {
            return Err("duplicate fixture object".into());
        }
        let transfer = hash(
            b"spiralcore.receiver-stages.transfer.v1\0",
            &[
                &(len as u64).to_le_bytes(),
                &(position as u64).to_le_bytes(),
            ],
        );
        let frames = prepare_frames(
            (position > 0).then_some(previous.as_slice()),
            &target,
            transfer,
        )?;
        let offer = Offer::decode(&frames[0].body)?;
        let expected_mode = if position == 0 || position == 5 {
            Mode::Raw
        } else {
            Mode::SparseV1
        };
        if offer.mode != expected_mode {
            return Err("unexpected diagnostic mode".into());
        }
        let expected = expected_replies(&frames)?;
        let expected_body_bytes =
            wan::encode_batch(&frames)?.len() + wan::encode_batch(&expected)?.len() + 16;
        let Dispatch {
            replies,
            outer,
            stage_clocks,
            mut error,
        } = dispatch(&mut endpoint, &c, &frames, mode, position as u64)?;
        if error.is_none() && replies != expected {
            error = Some("reply identity/sequence/body differs".into());
        }
        emit(
            out,
            &json!({"event":"operation","bytes":len,"clock_mode":mode.label(),"position":position,"mode":format!("{:?}",offer.mode),"object":hex(&id),"sha256":sha(&target),"frames":frames.len(),"data_frames":frames.len()-2,"modeled_http_body_bytes":expected_body_bytes,"outer":outer,"stage_clocks":stage_clocks,"error":error}),
        )?;
        if error.is_some() {
            return Err("receiver operation failed; store retained".into());
        }
        previous = target;
    }
    drop(endpoint);
    let mut reopened = open(root)?;
    let mut expected = fixture(len, fixture_seed(len, false));
    for position in 0..=10 {
        if position > 0 {
            expected = next_target(&expected, position)?;
        }
        let id = object_id(&expected);
        let read = reopened.read_object(&c, id);
        let exact = read.as_ref().is_ok_and(|actual| actual == &expected);
        emit(
            out,
            &json!({"event":"verification","bytes":len,"clock_mode":mode.label(),"position":position,"object":hex(&id),"sha256":sha(&expected),"exact":exact,"error":read.as_ref().err().map(|e| format!("{e:?}"))}),
        )?;
        if !exact {
            return Err("reopened bytes differ; store retained".into());
        }
    }
    drop(reopened);
    fs::remove_dir_all(root)?;
    emit(
        out,
        &json!({"event":"cleanup","bytes":len,"clock_mode":mode.label(),"owned_store_removed":true,"verified_objects":11}),
    )?;
    Ok(11)
}
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    if values.len().is_multiple_of(2) {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
    } else {
        values[values.len() / 2]
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || args[3].len() != 40 || !args[3].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("usage: receiver_stages NEW_STORE_PARENT NEW_JSONL SOURCE_COMMIT_40HEX".into());
    }
    let root = Path::new(&args[1]);
    if root.exists() {
        return Err("store parent already exists".into());
    }
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(root)?;
    emit(
        &mut out,
        &json!({"event":"registration","schema":"spiralcore.receiver-stages.v1","source_commit":args[3],"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"sources":{"diagnostic":sha(include_bytes!("receiver_stages.rs")),"endpoint":sha(include_bytes!("../src/endpoint.rs")),"delta":sha(include_bytes!("../src/delta.rs")),"lib":sha(include_bytes!("../src/lib.rs")),"protocol":sha(include_bytes!("../src/protocol.rs")),"lockfile":sha(include_bytes!("../Cargo.lock"))},"hardware_sha256":cfg!(feature="hardware-sha256"),"sequence_objects":11,"sequences":4,"objects":44,"discarded_warmups":0,"scope":"in-process unchanged Endpoint; frames/expected replies/fixture generation outside outer timer; Begin/grouped Data/Finish are nested in outer CPU/wall, not added; no per-Data-frame clocks; reopened verification and logging excluded; no WAN/RSS/energy claim"}),
    )?;
    for (len, modes) in [
        (1 << 20, [ClockMode::OuterOnly, ClockMode::Stages]),
        (16 << 20, [ClockMode::Stages, ClockMode::OuterOnly]),
    ] {
        for mode in modes {
            let child = root.join(format!("{len}-{}", mode.label()));
            if let Err(e) = run_sequence(&child, len, mode, &mut out) {
                emit(
                    &mut out,
                    &json!({"event":"failed","bytes":len,"clock_mode":mode.label(),"error":e.to_string(),"store":child}),
                )?;
                return Err(e);
            }
        }
    }
    let raw = fs::read_to_string(&args[2])?;
    let rows: Vec<Value> = raw
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    let mut gates = Vec::new();
    for len in [1 << 20, 16 << 20] {
        let mut ratios = Vec::new();
        for position in 0..=10 {
            let find = |mode| {
                rows.iter()
                    .find(|r| {
                        r["event"] == "operation"
                            && r["bytes"] == len
                            && r["position"] == position
                            && r["clock_mode"] == mode
                    })
                    .ok_or("missing paired observation")
            };
            let a = find("outer-only")?;
            let b = find("stages")?;
            for key in [
                "object",
                "sha256",
                "mode",
                "frames",
                "modeled_http_body_bytes",
            ] {
                if a[key] != b[key] {
                    return Err("clock modes differ in operation".into());
                }
            }
            let denominator = a["outer"]["cpu_ns"].as_u64().ok_or("invalid outer CPU")?;
            if denominator == 0 {
                return Err("unresolved zero CPU denominator".into());
            }
            ratios.push(
                b["outer"]["cpu_ns"].as_u64().ok_or("invalid staged CPU")? as f64
                    / denominator as f64,
            );
        }
        let ratio = median(ratios.clone());
        gates.push(json!({"bytes":len,"paired_positions":11,"ratios":ratios,"median_staged_over_outer_cpu_ratio":ratio,"threshold":1.10,"attribution_qualified":ratio<=1.10}));
    }
    fs::remove_dir(root)?;
    emit(
        &mut out,
        &json!({"event":"complete","exact_objects":44,"sequences":4,"gates":gates,"note":"one sequence per size/mode; paired positions are different updates, not independent replicate trials; failed overhead gate prohibits stage attribution, never filters observations"}),
    )?;
    out.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spiralcore_transfer::{delta, object_id};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn fresh() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "spiralcore-receiver-stages-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn rolling_fixture_has_nine_local_edits_one_escape_and_no_duplicate_object() {
        let mut prior = fixture(65536, 0x4189_83f1_097a_22e1);
        let mut seen = std::collections::BTreeSet::from([object_id(&prior)]);
        for update in 1..=10 {
            let next = next_target(&prior, update).unwrap();
            assert_eq!(next.len(), prior.len());
            let changed = prior.iter().zip(&next).filter(|(a, b)| a != b).count();
            assert!(seen.insert(object_id(&next)));
            if update == 5 {
                assert!(changed > prior.len() / 2);
                assert!(delta::encode(&prior, &next).unwrap().is_none());
            } else {
                assert_eq!(changed, 32);
                assert!(delta::encode(&prior, &next).unwrap().is_some());
            }
            prior = next;
        }
    }

    #[test]
    fn prepared_frames_are_complete_and_exact_before_receiver_timing() {
        let base = fixture(65536, 0x4189_83f1_097a_22e1);
        for update in [1, 5] {
            let target = next_target(&base, update).unwrap();
            let frames = prepare_frames(Some(&base), &target, [7; 32]).unwrap();
            let offer = Offer::decode(&frames[0].body).unwrap();
            assert_eq!(offer.object, object_id(&target));
            assert_eq!(
                offer.mode,
                if update == 5 {
                    Mode::Raw
                } else {
                    Mode::SparseV1
                }
            );
            assert_eq!(frames[0].kind, 1);
            assert_eq!(frames.last().unwrap().kind, 3);
            assert!(frames
                .iter()
                .enumerate()
                .all(|(i, f)| f.seq == i as u64 && f.transfer == [7; 32]));
            let mut output = Vec::new();
            let mut operations = 0;
            for frame in &frames[1..frames.len() - 1] {
                assert_eq!(frame.kind, 2);
                if offer.mode == Mode::SparseV1 {
                    delta::apply(
                        &base,
                        &mut output,
                        target.len(),
                        &mut operations,
                        &frame.body,
                    )
                    .unwrap();
                } else {
                    assert_eq!(frame.body[0], 0);
                    output.extend_from_slice(&frame.body[5..]);
                }
            }
            assert_eq!(output, target);
        }
    }

    #[test]
    fn each_clock_mode_runs_one_seed_ten_updates_and_reopens_all_exactly() {
        for mode in [ClockMode::OuterOnly, ClockMode::Stages] {
            let root = fresh();
            let mut log = Vec::new();
            assert_eq!(run_sequence(&root, 4096, mode, &mut log).unwrap(), 11);
            let rows: Vec<serde_json::Value> = std::str::from_utf8(&log)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            let operations: Vec<_> = rows.iter().filter(|r| r["event"] == "operation").collect();
            assert_eq!(operations.len(), 11);
            for r in operations {
                assert!(r["error"].is_null());
                assert_eq!(r["stage_clocks"].is_null(), mode == ClockMode::OuterOnly);
                if mode == ClockMode::Stages {
                    let stages = &r["stage_clocks"];
                    let total: u64 = ["begin", "data", "finish"]
                        .iter()
                        .map(|key| stages[*key]["wall_ns"].as_u64().unwrap())
                        .sum();
                    assert!(total <= r["outer"]["wall_ns"].as_u64().unwrap());
                    assert_eq!(stages["groups"], 3);
                }
            }
            assert_eq!(
                rows.iter()
                    .filter(|r| r["event"] == "verification" && r["exact"] == true)
                    .count(),
                11
            );
            assert!(!root.exists());
        }
    }

    #[test]
    fn existing_root_is_rejected_without_deleting_user_files() {
        let root = fresh();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("keep"), b"preserve").unwrap();
        let mut log = Vec::new();
        assert!(run_sequence(&root, 4096, ClockMode::OuterOnly, &mut log).is_err());
        assert_eq!(fs::read(root.join("keep")).unwrap(), b"preserve");
        fs::remove_file(root.join("keep")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn malformed_data_stops_before_finish_and_never_publishes() {
        let root = fresh();
        let mut endpoint = open(&root).unwrap();
        let target = fixture(4096, 71);
        let mut frames = prepare_frames(None, &target, [11; 32]).unwrap();
        frames[1].body.truncate(5);
        let observed = dispatch(&mut endpoint, &context(), &frames, ClockMode::Stages, 0).unwrap();
        assert!(observed.error.is_some());
        assert_eq!(observed.replies.len(), 1);
        assert_eq!(observed.stage_clocks.as_ref().unwrap()["groups"], 2);
        assert!(observed.stage_clocks.as_ref().unwrap()["finish"].is_null());
        assert_eq!(
            endpoint.read_object(&context(), object_id(&target)),
            Err(Error::Unknown)
        );
        drop(endpoint);
        fs::remove_dir_all(root).unwrap();
    }
}
