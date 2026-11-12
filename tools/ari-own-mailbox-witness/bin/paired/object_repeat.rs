//! Exercise the two guest workloads while their original owner services the selected pair.
use super::{A, B, BODY, workload};
use anyhow::{Context, Result, ensure};
use mvm_core::mailbox::reader::MailboxReaderBootBinding;
use serde_json::{Value, json};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

// Frozen Object-v1 identities of the two checked-in qualification fixtures.
const FIXTURE_ID: &str = if cfg!(feature = "large-object-fixture") {
    "0f6b13787cb3e42e48ab190848ddb08a9427a0eeebda8aa5b41f2095c9bedc51"
} else {
    "8791bb4715aaf056d927961968ee5b0e14a5cb0df911ab7ee334bbca209f0598"
};

const COUNTERS: [&str; 6] = [
    "operations_to_agent",
    "operations_from_agent",
    "ack_to_agent",
    "ack_from_agent",
    "authority_to_agent",
    "authority_from_agent",
];

fn number(row: &Value, field: &str) -> Result<u64> {
    row[field]
        .as_u64()
        .with_context(|| format!("missing {field} counter"))
}

fn id(value: &Value, bytes: usize) -> Result<&str> {
    let text = value.as_str().context("missing identifier")?;
    ensure!(
        text.len() == bytes * 2
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid identifier"
    );
    Ok(text)
}

#[derive(Default)]
struct Receiver {
    instance: Option<String>,
    counters: Option<[u64; 6]>,
}

fn reader_counts(traffic: &Value, previous: Option<[u64; 6]>) -> Result<[u64; 6]> {
    ensure!(
        traffic["order"] == json!(COUNTERS),
        "unexpected reader counter order"
    );
    let current: [u64; 6] = serde_json::from_value(traffic["this_invocation"].clone())?;
    let total: [u64; 6] = serde_json::from_value(traffic["worker_cumulative"].clone())?;
    for i in 0..6 {
        ensure!(
            current[i] > 0 && current[i] <= total[i],
            "missing reader traffic for {}",
            COUNTERS[i]
        );
        if let Some(prior) = previous {
            ensure!(
                total[i].checked_sub(prior[i]) == Some(current[i]),
                "discontinuous reader counter for {}",
                COUNTERS[i]
            );
        }
    }
    Ok(total)
}

fn check_turn(row: &Value, tx: &str, seq: u64, worker: &str, body: &[u8]) -> Result<bool> {
    ensure!(
        row["transfer"] == tx && row["seq"] == seq && id(&row["receiver_instance"], 16)? == worker,
        "B returned another transfer, sequence or worker"
    );
    ensure!(
        row["original_call_acknowledged"] == true && row["already_completed"] != true,
        "fresh original Call acknowledgment required"
    );
    number(row, "cpu_ns")?;
    number(row, "wall_ns")?;
    match row.get("readback_hex") {
        Some(Value::Null) => Ok(false),
        Some(Value::String(bytes)) => {
            ensure!(
                bytes.len() == 2 * body.len() && hex::decode(bytes)? == body,
                "B's exact object readback differs"
            );
            Ok(true)
        }
        _ => anyhow::bail!("missing B readback disposition"),
    }
}

fn check_sender(row: &Value, turns: usize, len: usize, warm: bool) -> Result<()> {
    ensure!(
        number(row, "calls")? == turns as u64 && number(row, "object_bytes")? == len as u64,
        "A and B disagree on object length or Call count"
    );
    let data = number(row, "data_bytes")?;
    ensure!(
        if warm {
            turns == 1 && data == 0
        } else {
            turns >= 3 && data == len as u64
        },
        "expected cold literal transfer followed by a one-Call warm lookup"
    );
    ensure!(
        row["object_id"] == FIXTURE_ID,
        "A reported a different fixture object identity"
    );
    for k in [
        "object_request_bytes",
        "object_reply_bytes",
        "operations_to_agent",
        "operations_from_agent",
        "ack_to_agent",
        "ack_from_agent",
    ] {
        ensure!(
            number(row, k)? > 0,
            "missing measured source traffic for {k}"
        );
    }
    number(row, "cpu_ns")?;
    number(row, "wall_ns")?;
    Ok(())
}

fn qualification_home() -> Result<PathBuf> {
    let home = PathBuf::from(
        std::env::var_os("MVM_HOME")
            .context("set MVM_HOME to the active private qualification home")?,
    );
    let metadata = std::fs::symlink_metadata(&home)?;
    ensure!(
        home.is_absolute()
            && home.canonicalize()? == home
            && metadata.is_dir()
            && metadata.mode() & 0o077 == 0,
        "expected a private canonical MVM_HOME"
    );
    mvm_core::private_fs::require_current_owner(&metadata)?;
    Ok(home)
}

fn original_boots(home: &Path, pair: [u8; 32]) -> Result<[MailboxReaderBootBinding; 2]> {
    let load = |name| -> Result<_> {
        let boot = MailboxReaderBootBinding::load(&mvm_core::config::vm_state_dir_at(home, name))?;
        ensure!(
            boot.durable_object_pair_sha256_v1 == Some(pair),
            "original {name} boot has a different pair"
        );
        Ok(boot)
    };
    let boots = [load(A)?, load(B)?];
    ensure!(
        boots[0].boot != boots[1].boot && boots[0].mailbox != boots[1].mailbox,
        "two distinct admitted guests required"
    );
    Ok(boots)
}

fn round(pair: &str, tx: &str, b_boot: &str, receiver: &mut Receiver, warm: bool) -> Result<Value> {
    let start = Instant::now();
    let host_cpu = cpu_time::ProcessTime::now();
    let until = start + Duration::from_secs(45);
    let mut a = workload::Call::start(
        A,
        &json!({"object_send":{"pair":pair, "transfer":tx, "payload_hex":hex::encode(BODY)}}),
        until,
    )?;
    let mut source = None;
    let mut dest = Vec::new();
    let mut receiver_cpu = 0u64;
    let mut sdk_in = a.input_bytes;
    let mut sdk_out = 0;
    // Raw Object-v1 needs Begin, at most two Data Calls for this fixture, then Finish.
    let mut exact = false;
    for seq in 0..4 {
        let mut b = workload::Call::start(
            B,
            &json!({"object_receive":{"pair":pair,"transfer":tx,"seq":seq}}),
            until,
        )?;
        sdk_in += b.input_bytes;
        let mut row = loop {
            if source.is_none() {
                source = a.completed()?;
            }
            if let Some(result) = b.completed()? {
                break result;
            }
            ensure!(
                Instant::now() < until,
                "guest object operation deadline expired"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        sdk_out += b.output_bytes;
        ensure!(
            row["receiver_boot"] == b_boot,
            "B did not use the selected boot"
        );
        let who = id(&row["receiver_instance"], 16)?;
        let worker = receiver.instance.get_or_insert_with(|| who.to_owned());
        exact = check_turn(&row, tx, seq, worker, BODY)?;
        receiver.counters = Some(reader_counts(&row["reader_bytes"], receiver.counters)?);
        receiver_cpu = receiver_cpu
            .checked_add(number(&row, "cpu_ns")?)
            .context("receiver CPU overflow")?;
        // Verification stdout is counted above. Retain its exact length, not another payload copy.
        row.as_object_mut()
            .context("invalid B result")?
            .remove("readback_hex");
        row["exact_readback_bytes"] = json!(if exact { BODY.len() } else { 0 });
        dest.push(row);
        if exact {
            break;
        }
    }
    ensure!(exact, "B did not verify the terminal object");
    let source = match source {
        Some(done) => done,
        None => a.wait()?,
    };
    let elapsed = start.elapsed();
    sdk_out += a.output_bytes;
    let application_bytes = number(&source, "object_request_bytes")?
        .checked_add(number(&source, "object_reply_bytes")?)
        .context("application byte total overflow")?;
    let workload_cpu = number(&source, "cpu_ns")?
        .checked_add(receiver_cpu)
        .context("workload CPU overflow")?;
    let measured = json!({"phase":if warm {"warm"} else {"cold"}, "transfer":tx, "guest_a":source, "guest_b":dest,
        "application_bytes_both_directions":application_bytes, "workload_cpu_ns":workload_cpu,
        "coordinator_process_cpu_ns":host_cpu.elapsed().as_nanos() as u64,
        "receiver_cpu_ns":receiver_cpu, "through_both_guest_results_ns":elapsed.as_nanos() as u64,
        "workload_stdin_bytes":sdk_in,"workload_stdout_bytes_including_readback":sdk_out});
    // Keep completed observations on stderr even if the expected reuse gate fails.
    eprintln!("{}", measured);
    check_sender(&source, dest.len(), BODY.len(), warm)?;
    Ok(measured)
}

pub fn run(pair: &str) -> Result<()> {
    let bytes: [u8; 32] = hex::decode(pair)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("pair must contain 32 bytes"))?;
    ensure!(
        bytes != [0; 32] && hex::encode(bytes) == pair,
        "canonical pair ID required"
    );
    let home = qualification_home()?;
    let boots = original_boots(&home, bytes)?;
    let b_boot = hex::encode(boots[1].boot.as_bytes());
    let cold_id = hex::encode(rand::random::<[u8; 32]>());
    let warm_id = hex::encode(rand::random::<[u8; 32]>());
    ensure!(cold_id != warm_id, "distinct transfer identities required");
    eprintln!(
        "{}",
        json!({"pair":pair,"cold_transfer":cold_id,"warm_transfer":warm_id})
    );
    let mut worker = Receiver::default();
    let cold = round(pair, &cold_id, &b_boot, &mut worker, false)?;
    ensure!(
        original_boots(&home, bytes)? == boots,
        "guest boots changed after cold transfer"
    );
    let warm = round(pair, &warm_id, &b_boot, &mut worker, true)?;
    ensure!(
        original_boots(&home, bytes)? == boots
            && cold["guest_a"]["object_id"] == warm["guest_a"]["object_id"],
        "installed pair or object identity changed"
    );
    let cold_bytes = number(&cold, "application_bytes_both_directions")?;
    let warm_bytes = number(&warm, "application_bytes_both_directions")?;
    ensure!(
        warm_bytes < cold_bytes,
        "warm transfer did not reduce application traffic"
    );
    println!(
        "{}",
        json!({"application_bytes_saved":cold_bytes-warm_bytes,"schema":"spiralcore.installed-object-repeat.v1","pair":pair,
        "source_boot":hex::encode(boots[0].boot.as_bytes()),"receiver_boot":b_boot,"receiver_instance":worker.instance,
        "object_bytes":BODY.len(),"cold":cold,"warm":warm,"host_relay_bytes":null,"network_wire_bytes":null})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_readback_requires_exact_bytes_original_turn_and_same_worker() {
        let tx = "04".repeat(32);
        let worker = "12".repeat(16);
        let mut row = json!({"transfer":tx,"seq":2,"receiver_instance":worker,
            "original_call_acknowledged":true,"readback_hex":"6100","cpu_ns":0,"wall_ns":4,
            "reader_bytes":{"order":COUNTERS,"this_invocation":[1,2,3,4,5,6]}});
        assert!(check_turn(&row, &tx, 2, &worker, b"a\0").unwrap());
        assert!(check_turn(&row, &tx, 3, &worker, b"a\0").is_err());
        assert!(check_turn(&row, &tx, 2, &"34".repeat(16), b"a\0").is_err());
        assert!(check_turn(&row, &tx, 2, &worker, b"a").is_err());
        row["readback_hex"] = Value::Null;
        assert!(!check_turn(&row, &tx, 2, &worker, b"a\0").unwrap());
        row["already_completed"] = json!(true);
        assert!(check_turn(&row, &tx, 2, &worker, b"a\0").is_err());
    }

    #[test]
    fn receiver_counts_must_cover_every_fresh_reader_operation() {
        let mut row = json!({"order":COUNTERS,"this_invocation":[1,2,3,4,5,6],
            "worker_cumulative":[11,12,13,14,15,16]});
        assert_eq!(
            reader_counts(&row, Some([10; 6])).unwrap(),
            [11, 12, 13, 14, 15, 16]
        );
        assert!(reader_counts(&row, Some([11; 6])).is_err());
        row["this_invocation"][2] = json!(0);
        assert!(reader_counts(&row, None).is_err());
        row["this_invocation"][2] = json!(18);
        assert!(reader_counts(&row, None).is_err());
    }

    #[test]
    fn zero_literal_bytes_alone_cannot_prove_warm_completion() {
        let mut row = json!({"calls":1,"data_bytes":0,"object_bytes":4096,
            "object_id":FIXTURE_ID,"cpu_ns":1,"wall_ns":2,
            "object_request_bytes":102,"object_reply_bytes":88,
            "operations_to_agent":1,"operations_from_agent":1,"ack_to_agent":1,"ack_from_agent":1});
        assert!(check_sender(&row, 1, 4096, true).is_ok());
        assert!(check_sender(&row, 2, 4096, true).is_err());
        row["data_bytes"] = json!(4096);
        assert!(check_sender(&row, 1, 4096, true).is_err());
        assert!(check_sender(&row, 1, 4096, false).is_err());
    }
}
