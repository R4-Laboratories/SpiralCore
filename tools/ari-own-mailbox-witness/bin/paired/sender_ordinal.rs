//! Run a bounded campaign on the same originally admitted A and B.
use super::{A, B, fixture, ordinal_fixture, verify_return, workload::Call};
use anyhow::{Context, Result, ensure};
use mvm_contract::protocol::mailbox::MailboxJournalDirectionV1;
use mvm_core::mailbox::sender_ordinal::{SenderOrdinalV1, SenderSettlementV1};
use mvm_hostd::mailbox_process::original_journal::{
    JournalHelper, relay::OriginalJournalSourceHost,
};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::{fs::OpenOptionsExt, process::ExitStatusExt},
    path::Path,
    time::{Duration, Instant},
};

fn worker(name: &str, input: &Value, until: Instant) -> Result<Value> {
    let mut c = Call::start(name, input, until)?;
    let mut result = c.wait()?;
    result["sdk_request_bytes"] = json!(c.input_bytes);
    result["sdk_response_bytes"] = json!(c.output_bytes);
    Ok(result)
}

fn one(
    n: u64,
    d: &MailboxJournalDirectionV1,
    commitment: [u8; 32],
    owner: &mut OriginalJournalSourceHost,
    restart_helpers: bool,
) -> Result<Value> {
    let begin = Instant::now();
    let until = begin + Duration::from_secs(50);
    let handle = SenderOrdinalV1 {
        direction: commitment,
        ordinal: n,
    };
    let id = hex::encode(handle.message_id()?.as_bytes());
    let operation =
        |name: &str| json!({"sender_ordinal":name,"direction":hex::encode(commitment),"ordinal":n});
    let stage = worker(A, &operation("stage"), until)?;
    ensure!(
        stage["outcome"] == "pending" && stage["message_id"] == id,
        "original stage not pending"
    );
    let staged_ns = begin.elapsed().as_nanos();
    let control_begin_ns = begin.elapsed().as_nanos();
    let control = worker(B, &json!({"ordinal_empty_control":n}), until)?;
    let control_end_ns = begin.elapsed().as_nanos();
    ensure!(
        control["ordinal"] == n && control["empty"] == true,
        "missing B Receive control"
    );
    let mut c = Call::start(A, &operation("status"), until)?;
    let mut relay = None;
    let mut poll_count = 0_u32;
    let mut req_bytes = 0_usize;
    let mut reply_bytes = 0_usize;
    let mut pending_cpu = 0_u64;
    let (mut pending_write, mut pending_read) = (0_u64, 0_u64);
    let source = loop {
        if relay.is_none() {
            let mut seen = None;
            let count = owner.forward_once_observed(until, |env, bytes| {
                seen = Some(verify_return(d, env, bytes))
            })?;
            if let Some(count) = count {
                let v2 = seen.context("missing original relay")??;
                ensure!(
                    v2.receipt.message_id == handle.message_id()?,
                    "B accepted another ordinal"
                );
                relay = Some((
                    count,
                    hex::encode(SenderSettlementV1::receipt_digest(&v2)?),
                    begin.elapsed().as_nanos(),
                ));
            }
        }
        if let Some(row) = c.completed()? {
            req_bytes += c.input_bytes;
            reply_bytes += c.output_bytes;
            poll_count += 1;
            ensure!(row["message_id"] == id, "wrong original status");
            match row["outcome"].as_str() {
                Some("accepted") => break row,
                Some("pending") => {
                    pending_write += row["agent_request_bytes"]
                        .as_u64()
                        .context("missing original reader write")?;
                    pending_read += row["agent_reply_bytes"]
                        .as_u64()
                        .context("missing original reader read")?;
                    pending_cpu += row["cpu_ns"].as_u64().context("missing guest CPU")?;
                    ensure!(Instant::now() < until, "pending ordinal deadline");
                    c = Call::start(A, &operation("status"), until)?;
                }
                _ => anyhow::bail!("unexpected source disposition"),
            }
        }
        ensure!(Instant::now() < until, "original status/relay deadline");
        std::thread::sleep(Duration::from_millis(2));
    };
    let (counts, digest, relay_at_ns) =
        relay.context("guest accepted without matching observed relay")?;
    ensure!(
        source["receipt_digest"] == digest,
        "A recovered a different signed B receipt"
    );
    let helper_recovery = if restart_helpers {
        let mut restarts = Vec::new();
        for which in [JournalHelper::Source, JournalHelper::Destination] {
            let start = Instant::now();
            let status = owner.restart_helper(which, until)?;
            ensure!(
                status.signal() == Some(libc::SIGKILL),
                "original helper was not interrupted by the controller"
            );
            restarts.push(json!({
                "helper": format!("{which:?}"),
                "original_exit_signal": status.signal(),
                "elapsed_ns": start.elapsed().as_nanos(),
            }));
        }
        let recovered = worker(A, &operation("status"), until)?;
        ensure!(
            recovered["outcome"] == "accepted"
                && recovered["message_id"] == id
                && recovered["receipt_digest"] == digest,
            "helper recovery changed the original accepted receipt"
        );
        Some(json!({"restarts":restarts,"source_status":recovered}))
    } else {
        None
    };
    let mut b = worker(
        B,
        &json!({"message_id":id,"ephemeral_readback":true,"ordinal_readback":n}),
        until,
    )?;
    ensure!(
        b["role"] == "received_and_acknowledged" && b["message_id"] == id,
        "missing B signed ACK"
    );
    let expected = ordinal_fixture::ordinal_frame(n).context("ordinal outside fixture range")?;
    let hex = b
        .as_object_mut()
        .context("missing destination result")?
        .remove("readback_hex")
        .context("missing exact body")?;
    let recovered = hex::decode(hex.as_str().context("invalid readback")?)?;
    ensure!(
        recovered == expected[expected.len() - fixture::BODY_LEN..],
        "B readback differs for ordinal {n}"
    );
    let readback_at_ns = begin.elapsed().as_nanos();
    let mut req = operation("settle");
    req["receipt_digest"] = json!(digest);
    let mut settle = worker(A, &req, until)?;
    let mut pending_settles = 0u64;
    let (mut settle_write, mut settle_read, mut settle_cpu) = (0u64, 0u64, 0u64);
    let (mut pending_sdk_request, mut pending_sdk_response) = (0u64, 0u64);
    while settle["message_id"] == id && settle["outcome"] == "pending" {
        pending_settles += 1;
        pending_sdk_request += settle["sdk_request_bytes"]
            .as_u64()
            .context("missing pending settlement SDK request size")?;
        pending_sdk_response += settle["sdk_response_bytes"]
            .as_u64()
            .context("missing pending settlement SDK response size")?;
        settle_write += settle["agent_request_bytes"]
            .as_u64()
            .context("missing settlement request cost")?;
        settle_read += settle["agent_reply_bytes"]
            .as_u64()
            .context("missing settlement response cost")?;
        settle_cpu += settle["cpu_ns"]
            .as_u64()
            .context("missing settlement CPU")?;
        ensure!(Instant::now() < until, "original guest settlement deadline");
        settle = worker(A, &operation("status"), until)?;
    }
    ensure!(
        settle["outcome"] == "settled"
            && settle["message_id"] == id
            && settle["retired_through"] == n,
        "contiguous signed settlement absent"
    );
    let mut row = json!({"ordinal":n,"source_stage":stage,"source_return":source,"source_settlement":settle,"destination":b,"empty_control":control,
        "staged_ns":staged_ns,"control_start_ns":control_begin_ns,"control_end_ns":control_end_ns,"verified_host_receipt_ns":relay_at_ns,"verified_guest_readback_ns":readback_at_ns,"verified_settlement_ns":begin.elapsed().as_nanos(),
        "source_status_calls":poll_count,"source_status_request_bytes":req_bytes,"source_status_response_bytes":reply_bytes,"pending_status_guest_cpu_ns":pending_cpu,"pending_status_agent_written":pending_write,"pending_status_agent_read":pending_read,"pending_settlement_statuses":pending_settles,"pending_settlement_agent_written":settle_write,"pending_settlement_agent_read":settle_read,"pending_settlement_cpu_ns":settle_cpu,
        "pending_settlement_sdk_request_bytes":pending_sdk_request,"pending_settlement_sdk_response_bytes":pending_sdk_response,
        "sealed_to_b":counts.sealed_to_destination,"signed_to_a":counts.reply_to_source,"exact_bytes":recovered.len()});
    if let Some(recovery) = helper_recovery {
        row["helper_recovery"] = recovery;
    }
    Ok(row)
}
pub fn campaign(
    count: usize,
    home: &Path,
    d: &MailboxJournalDirectionV1,
    commitment: [u8; 32],
    owner: &mut OriginalJournalSourceHost,
    restart_helpers: bool,
) -> Result<Value> {
    ensure!(
        (1..=2049).contains(&count)
            && d.sender_settlement.is_some()
            && (!restart_helpers || count == 1),
        "ordinal campaign was not selected"
    );
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(home.join("sender-ordinal-series.jsonl"))?;
    let start = Instant::now();
    let mut forward = 0_u64;
    let mut reverse = 0_u64;
    for ordinal in 1..=count {
        let row = one(ordinal as u64, d, commitment, owner, restart_helpers)
            .with_context(|| format!("ordinal {ordinal}/{count}"))?;
        forward += row["sealed_to_b"].as_u64().context("missing frame count")?;
        reverse += row["signed_to_a"]
            .as_u64()
            .context("missing receipt count")?;
        serde_json::to_writer(&mut out, &row)?;
        out.write_all(b"\n")?;
        out.sync_data()?;
    }
    Ok(
        json!({"ordinals":count,"exact_body_bytes":count*fixture::BODY_LEN,"per_transfer_records":"sender-ordinal-series.jsonl",
        "sealed_to_b":forward,"signed_to_a":reverse,"verified_settlements":count,"series_elapsed_ns":start.elapsed().as_nanos()}),
    )
}
