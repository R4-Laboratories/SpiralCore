//! Local canonical-envelope conformance measurement; no guest or WAN claim.
use cpu_time::ProcessTime;
use serde_json::{Value, json};
use spiralcore_mvm_adapter::{Counters, conformance::Harness};
use spiralcore_transfer::{
    client::{self, Stats},
    hex, object_id,
    protocol::{Frame, Mode},
    schema,
    session::WarmSession,
    zstd_whole,
};
use std::{fs, io::Write, path::Path, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

trait DurableWrite: Write {
    fn persist(&mut self) -> std::io::Result<()>;
}
impl DurableWrite for fs::File {
    fn persist(&mut self) -> std::io::Result<()> {
        self.sync_all()
    }
}

fn emit(out: &mut impl Write, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    Ok(())
}

fn random(len: usize, mut seed: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}

fn targets(base: &[u8], trial: u64) -> Vec<(&'static str, Vec<u8>)> {
    let mut previous = base.to_vec();
    let mut values = Vec::new();
    for (position, kind) in [
        "localized",
        "unrelated",
        "localized-after",
        "truncate",
        "append",
    ]
    .into_iter()
    .enumerate()
    {
        let mut target = previous.clone();
        match position {
            0 | 2 => {
                let middle = target.len() / 2;
                for byte in &mut target[middle..middle + 32] {
                    *byte ^= 0xa5;
                }
            }
            1 => target = random(base.len(), 0x4b1d_8739_0921_4401 ^ trial),
            3 => target.truncate(target.len() - 31),
            4 => target.extend_from_slice(b"append\0bytes\0end\0"),
            _ => unreachable!(),
        }
        previous = target.clone();
        values.push((kind, target));
    }
    values
}

enum Sender {
    Warm(Box<WarmSession>),
    Whole(zstd_whole::PreparedEncoder),
}

fn body_size(frames: &[Frame]) -> u64 {
    12 + frames
        .iter()
        .map(|frame| 60 + frame.body.len() as u64)
        .sum::<u64>()
}

fn traffic(before: Counters, after: Counters) -> Result<Value> {
    let sent = after
        .envelope_bytes_sent
        .checked_sub(before.envelope_bytes_sent)
        .ok_or("counter rollback")?;
    let received = after
        .envelope_bytes_received
        .checked_sub(before.envelope_bytes_received)
        .ok_or("counter rollback")?;
    let frames = after
        .frame_bytes_sent
        .checked_sub(before.frame_bytes_sent)
        .ok_or("counter rollback")?;
    let opened = after
        .frame_bytes_opened
        .checked_sub(before.frame_bytes_opened)
        .ok_or("counter rollback")?;
    let count = after
        .envelopes_sent
        .checked_sub(before.envelopes_sent)
        .ok_or("counter rollback")?;
    let opened_count = after
        .envelopes_received
        .checked_sub(before.envelopes_received)
        .ok_or("counter rollback")?;
    if sent != received || frames != opened || count != opened_count {
        return Err("incomplete canonical exchange".into());
    }
    Ok(
        json!({"frame_bytes":frames,"envelope_bytes":sent,"received_envelope_bytes":received,"envelopes":count}),
    )
}

struct Totals {
    cpu: u128,
    wall: u128,
    envelope: u64,
    frames: u64,
}
impl Totals {
    fn add(&mut self, cpu: u128, wall: u128, traffic: &Value) {
        self.cpu += cpu;
        self.wall += wall;
        self.envelope += traffic["envelope_bytes"].as_u64().unwrap();
        self.frames += traffic["frame_bytes"].as_u64().unwrap();
    }
}

fn run_session(
    root: &Path,
    arm: &str,
    size: usize,
    trial: u64,
    out: &mut impl DurableWrite,
) -> Result<()> {
    if !matches!(arm, "warm" | "whole") || !(64..=(16 << 20)).contains(&size) {
        return Err("invalid registered input".into());
    }
    // Creating a new directory proves ownership; never reuse or remove user data.
    fs::create_dir(root)?;
    let generation = ProcessTime::now();
    let generation_wall = Instant::now();
    let base = random(size, 0x5842_aae1_42fc_1983 ^ trial);
    let targets = targets(&base, trial);
    let generation_cpu_us = generation.elapsed().as_micros();
    let generation_wall_us = generation_wall.elapsed().as_micros();
    let cpu = ProcessTime::now();
    let wall = Instant::now();
    let mut harness = Harness::new(&root.join("cache"))?;
    let context = harness.context().clone();
    let artifact = if arm == "warm" {
        Some(schema::fit(&[b"{\"value\":0,\"message\":\"training\"}\n"])?)
    } else {
        None
    };
    let mut sender = match &artifact {
        Some(artifact) => Sender::Warm(Box::new(WarmSession::new(&context, artifact)?)),
        None => Sender::Whole(zstd_whole::PreparedEncoder::new()?),
    };
    let setup_cpu = cpu.elapsed().as_micros();
    let setup_wall = wall.elapsed().as_micros();
    let setup_traffic = traffic(Counters::default(), harness.counters())?;
    let mut totals = Totals {
        cpu: 0,
        wall: 0,
        envelope: 0,
        frames: 0,
    };
    totals.add(setup_cpu, setup_wall, &setup_traffic);
    let mut setup = setup_traffic;
    setup["event"] = json!("setup");
    setup["arm"] = json!(arm);
    setup["size"] = json!(size);
    setup["trial"] = json!(trial);
    setup["cpu_us"] = json!(setup_cpu);
    setup["wall_us"] = json!(setup_wall);
    setup["generation_cpu_us"] = json!(generation_cpu_us);
    setup["generation_wall_us"] = json!(generation_wall_us);
    emit(out, &setup)?;

    let mut objects = vec![("seed", base)];
    if let Some(bytes) = artifact {
        objects.push(("schema", bytes));
    }
    objects.extend(targets);
    for (position, (kind, bytes)) in objects.iter().enumerate() {
        let expected = object_id(bytes);
        // Input ownership and fixture/expected-digest work are outside transfer timing.
        let owned = bytes.clone();
        let before = harness.counters();
        let mut modeled_http_body_bytes = 0;
        let mut exchange = |frames: &[Frame]| {
            modeled_http_body_bytes += body_size(frames);
            let replies = harness.exchange(frames)?;
            modeled_http_body_bytes += body_size(&replies) + 16;
            Ok(replies)
        };
        let mut transfer = [0u8; 32];
        transfer[..8].copy_from_slice(&(position as u64 + 1).to_le_bytes());
        transfer[8..16].copy_from_slice(&trial.to_le_bytes());
        transfer[16..24].copy_from_slice(&(size as u64).to_le_bytes());
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let stats: Stats = match (&mut sender, *kind) {
            (Sender::Warm(sender), "seed") => {
                sender.seed_base(&context, owned, transfer, &mut exchange)?
            }
            (Sender::Warm(sender), "schema") => {
                sender.seed_schema(&context, transfer, &mut exchange)?
            }
            (Sender::Warm(sender), _) => {
                sender
                    .send(&context, owned, transfer, false, &mut exchange)?
                    .transfer
            }
            (Sender::Whole(_), "seed") => {
                client::transfer_batched(&owned, Mode::Raw, None, transfer, false, &mut exchange)?
            }
            (Sender::Whole(sender), _) => client::transfer_prepared_zstd_whole_batched(
                sender,
                &owned,
                transfer,
                false,
                &mut exchange,
            )?,
        };
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        if stats.object != expected {
            return Err("publication identity mismatch".into());
        }
        let mut row = traffic(before, harness.counters())?;
        if row["frame_bytes"].as_u64() != Some(stats.request_bytes + stats.response_bytes) {
            return Err("frame accounting mismatch".into());
        }
        if arm == "warm"
            && size >= (1 << 20)
            && matches!(*kind, "localized" | "localized-after")
            && modeled_http_body_bytes != 600
        {
            return Err("warm HTTP model changed".into());
        }
        totals.add(cpu_us, wall_us, &row);
        row["event"] = json!("operation");
        row["arm"] = json!(arm);
        row["size"] = json!(size);
        row["trial"] = json!(trial);
        row["position"] = json!(position);
        row["kind"] = json!(kind);
        row["object"] = json!(hex(&expected));
        row["bytes"] = json!(bytes.len());
        row["mode"] = json!(format!("{:?}", stats.mode));
        row["cpu_us"] = json!(cpu_us);
        row["wall_us"] = json!(wall_us);
        row["modeled_http_body_bytes"] = json!(modeled_http_body_bytes);
        emit(out, &row)?;
    }
    harness.reopen()?;
    for (position, (kind, bytes)) in objects.iter().enumerate() {
        let id = object_id(bytes);
        let recovered = harness.read_object(id)?;
        if &recovered != bytes {
            return Err("stopped-store exact readback failed".into());
        }
        emit(
            out,
            &json!({"event":"verified","arm":arm,"size":size,"trial":trial,"position":position,"kind":kind,"object":hex(&id),"bytes":bytes.len(),"exact":true}),
        )?;
    }
    // Keep the verified store if its recorded evidence cannot be made durable.
    out.persist()?;
    drop(harness);
    fs::remove_dir_all(root)?;
    emit(
        out,
        &json!({"event":"session_complete","arm":arm,"size":size,"trial":trial,"operations":objects.len(),"cpu_us":totals.cpu,"wall_us":totals.wall,"frame_bytes":totals.frames,"envelope_bytes":totals.envelope,"exact_reopened":objects.len(),"owned_store_removed":!root.exists()}),
    )?;
    out.persist()?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: mailbox_transfer NEW_OWNED_ROOT NEW_JSONL_FILE".into());
    }
    let root = Path::new(&args[1]);
    // Both must be fresh: a failed campaign is retained, never overwritten.
    if root.exists() || Path::new(&args[2]).exists() {
        return Err("output paths must be new".into());
    }
    fs::create_dir(root)?;
    let mut output = fs::File::create_new(&args[2])?;
    let executable = fs::read(std::env::current_exe()?)?;
    emit(
        &mut output,
        &json!({"event":"registration","schema":"spiralcore.mvm.host-conformance.v1","mvm_revision":"3104a12c5b071a6851323fa8115d2afaf3f0bb8b","source_commit":std::env::var("SPIRALCORE_SOURCE").unwrap_or_else(|_|"unregistered".into()),"executable_object_id":hex(&object_id(&executable)),"hardware_sha256":cfg!(feature="hardware-sha256"),"trials_per_size":3,"sizes":[1<<20,16<<20],"operations":78,"transport":"canonical MVM InMemoryPeerTransport","network_or_guest":false,"cpu_scope":"combined sequential host process","byte_scope":"canonical encoded envelopes sent once in each direction; excludes network framing"}),
    )?;
    for size in [1 << 20, 16 << 20] {
        for trial in 0..3 {
            let arms = if trial % 2 == 0 {
                ["warm", "whole"]
            } else {
                ["whole", "warm"]
            };
            for arm in arms {
                run_session(
                    &root.join(format!("{size}-{trial}-{arm}")),
                    arm,
                    size,
                    trial,
                    &mut output,
                )?;
            }
        }
    }
    fs::remove_dir(root)?;
    emit(
        &mut output,
        &json!({"event":"complete","sessions":12,"exact_operations":78,"stores_removed":12}),
    )?;
    output.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    impl super::DurableWrite for Vec<u8> {
        fn persist(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FailedSync(Vec<u8>);
    impl std::io::Write for FailedSync {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            std::io::Write::write(&mut self.0, bytes)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl super::DurableWrite for FailedSync {
        fn persist(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("injected evidence sync failure"))
        }
    }

    #[test]
    fn failed_evidence_sync_retains_verified_store() {
        let root = std::env::temp_dir().join(format!("spiral-mvm-sync-{}", std::process::id()));
        assert!(!root.exists());
        let mut output = FailedSync(Vec::new());
        let result = super::run_session(&root, "whole", 128, 0, &mut output);
        assert!(result.is_err(), "evidence must be durable before cleanup");
        assert!(
            root.join("cache").exists(),
            "failed sync must retain the owned store"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mixed_session_records_exact_reopened_objects_and_canonical_bytes() {
        let root = std::env::temp_dir().join(format!("spiral-mvm-example-{}", std::process::id()));
        assert!(!root.exists());
        let mut rows = Vec::new();
        super::run_session(&root, "warm", 4096, 0, &mut rows).unwrap();
        let rows: Vec<serde_json::Value> = String::from_utf8(rows)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.iter().filter(|r| r["event"] == "operation").count(), 7);
        assert_eq!(
            rows.iter()
                .filter(|r| r["event"] == "verified" && r["exact"] == true)
                .count(),
            7
        );
        for row in rows.iter().filter(|r| r["event"] == "operation") {
            assert!(row["envelope_bytes"].as_u64().unwrap() > row["frame_bytes"].as_u64().unwrap());
            assert_eq!(row["envelope_bytes"], row["received_envelope_bytes"]);
        }
        assert!(
            !root.exists(),
            "only verified owned stores should be removed"
        );
    }
}
