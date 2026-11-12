//! One original data operation, including verified receipt and guest readback.
use super::{A, B, BODY, receipt_retry, series, verify_return, workload};
use anyhow::{Context, Result, ensure};
use mvm_contract::protocol::mailbox::{MailboxEnvelope, MailboxJournalDirectionV1};
use mvm_hostd::mailbox_process::original_journal::relay::OriginalJournalSourceHost;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub fn one(
    raw: &[u8],
    direction: &MailboxJournalDirectionV1,
    commitment: [u8; 32],
    helper: &mut OriginalJournalSourceHost,
    ordinal: Option<usize>,
) -> Result<Value> {
    let transaction = Instant::now();
    let until = transaction + Duration::from_secs(50);
    let source_request = json!({"frame_hex":hex::encode(&raw),"recipient":hex::encode(direction.address.recipient.as_bytes()),"direction":hex::encode(commitment)});
    let mut source_call = workload::Call::start(A, &source_request, until)?;
    let mut staged = None;
    let mut relay = None;
    let mut first_receipt: Option<receipt_retry::FirstReceipt<MailboxEnvelope>> = None;
    let control = if let Some(n) = ordinal {
        let stage = source_call.wait()?;
        let staged_ns = transaction.elapsed().as_nanos() as u64;
        staged = Some(stage);
        Some(series::admitted_control(
            n,
            &source_request,
            staged_ns,
            transaction,
            until,
        )?)
    } else {
        None
    };
    loop {
        if staged.is_none() {
            staged = source_call.completed()?
        }
        if relay.is_none() {
            let mut observed = None;
            let counts = helper.forward_once_observed(until, |env, bytes| {
                observed = Some((
                    env.clone(),
                    bytes.to_vec(),
                    verify_return(direction, env, bytes),
                ))
            })?;
            if let Some(counts) = counts {
                let (envelope, raw_receipt, checked) =
                    observed.context("missing original relay observation")?;
                let receipt = checked?;
                let at_ns = transaction.elapsed().as_nanos() as u64;
                if cfg!(feature = "receipt-loss-qualification") {
                    if let Some(first) = &first_receipt {
                        first
                            .verify_retry(
                                &envelope,
                                &raw_receipt,
                                counts.sealed_to_destination,
                                counts.reply_to_source,
                            )
                            .map_err(anyhow::Error::msg)?;
                    } else {
                        eprintln!(
                            "first verified B receipt: {} bytes, forward {} bytes, returned {}",
                            raw_receipt.len(),
                            counts.sealed_to_destination,
                            counts.reply_to_source
                        );
                        first_receipt = Some(
                            receipt_retry::FirstReceipt::lost(
                                envelope,
                                raw_receipt,
                                counts.sealed_to_destination,
                                counts.reply_to_source,
                                at_ns,
                            )
                            .map_err(anyhow::Error::msg)?,
                        );
                        continue;
                    }
                }
                relay = Some((counts, receipt, at_ns));
            }
        }
        if staged.is_some() && relay.is_some() {
            break;
        }
        ensure!(
            Instant::now() < until,
            "timed out awaiting original source Call or authenticated B verdict"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let source = staged.unwrap();
    let (counts, v2, verified_ns) = relay.unwrap();
    let id = hex::encode(v2.receipt.message_id.as_bytes());
    ensure!(
        source["role"] == "staged"
            && source["message_id"] == id
            && source["frame_bytes"] == super::fixture::FRAME_LEN,
        "B receipt differs from A original staged ID"
    );
    let mut b_request = json!({"message_id":id});
    if ordinal.is_some() {
        b_request["ephemeral_readback"] = json!(true);
    }
    let mut bcall = workload::Call::start(B, &b_request, until)?;
    let mut destination = loop {
        if let Some(v) = bcall.completed()? {
            break v;
        }
        ensure!(
            Instant::now() < until,
            "destination application Call incomplete"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    ensure!(
        destination["role"] == "received_and_acknowledged"
            && destination["message_id"] == id
            && destination["frame_bytes"] == super::fixture::FRAME_LEN
            && [
                "receive_to_agent",
                "receive_from_agent",
                "ack_to_agent",
                "ack_from_agent"
            ]
            .into_iter()
            .all(|k| destination[k].as_u64().is_some_and(|n| n > 0)),
        "missing destination application acceptance"
    );
    let recovery = if let Some(first) = first_receipt {
        ensure!(
            transaction.elapsed() < Duration::from_secs(30),
            "empty probe outside journal retention"
        );
        ensure!(
            source["outbox_retired_without_archive"] == true
                && source["outbox_retirement_wait_ns"].is_u64()
                && destination["after_ack"]["empty"] == true
                && ["to_agent", "from_agent"]
                    .iter()
                    .all(|k| destination["after_ack"][*k].as_u64().is_some_and(|v| v > 0)),
            "guest did not confirm original outbox retirement and empty B queue"
        );
        json!({"first_b_receipt_ns":first.at_ns,"verified_retry_ns":verified_ns,
            "first_sealed_a_to_b":first.forward_bytes,"first_receipt_from_b":first.bytes.len(),
            "first_receipt_to_a":0,"retry_sealed_a_to_b":counts.sealed_to_destination,
            "retry_receipt_from_b":first.bytes.len(),"retry_receipt_to_a":counts.reply_to_source,
            "exact_envelope_equal":true,"exact_signed_receipt_bytes_equal":true})
    } else {
        Value::Null
    };
    let reply_hex = destination
        .as_object_mut()
        .context("expected guest record")?
        .remove("readback_hex")
        .context("missing independent readback")?;
    let bytes = hex::decode(reply_hex.as_str().context("bad guest readback")?)?;
    ensure!(
        bytes.as_slice() == BODY,
        "byte comparison failed after B workload ACK"
    );
    let mut row = json!({
        "guest_a":source,"guest_b":destination,"sealed_a_to_b_bytes":counts.sealed_to_destination,"signed_b_to_a_bytes":counts.reply_to_source,
        "sdk_call_application_io":{"source_request":source_call.input_bytes,"source_reply":source_call.output_bytes,"destination_request":bcall.input_bytes,"destination_reply_with_readback":bcall.output_bytes},
        "through_observed_v2_ns":verified_ns,"through_verified_destination_call_ns":transaction.elapsed().as_nanos() as u64,
        "receipt_recovery":recovery, "verified_body_bytes":bytes.len(),"receipt_outcome":"AcceptedLocalDurable","guest_a_observed_receipt":null,"wan_bytes":null});
    if let Some(control) = control {
        row["interleaved_control"] = control;
    }
    Ok(row)
}
