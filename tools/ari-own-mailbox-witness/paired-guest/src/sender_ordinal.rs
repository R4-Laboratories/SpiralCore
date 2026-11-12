//! Calls on A's original guest-agent socket. Only the agent can sign settlement.
use anyhow::{Context, Result, ensure};
use mvm_core::mailbox::{
    reader_wire::{
        ApplicationReaderRequest, ReaderResponse, read_response, write_application_request,
    },
    sender_ordinal::{ApplicationSenderRequestV1, SenderOrdinalV1, SenderSettlementV1},
};
use serde_json::{Value, json};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before, receive_before};
use std::time::Instant;

pub(super) fn run(input: &Value, until: Instant) -> Result<Value> {
    let ordinal = input["ordinal"].as_u64().context("invalid ordinal")?;
    let handle = SenderOrdinalV1 {
        direction: super::field(input, "direction")?,
        ordinal,
    };
    let mode = input["sender_ordinal"]
        .as_str()
        .context("missing operation")?;
    let fields = input.as_object().context("expected object")?.len();
    let request = match mode {
        "stage" if fields == 3 => ApplicationSenderRequestV1::Stage {
            handle,
            payload: super::fixture::ordinal_frame(ordinal).context("fixture bounds")?,
        },
        "status" if fields == 3 => ApplicationSenderRequestV1::Status { handle },
        "settle" if fields == 4 => ApplicationSenderRequestV1::Settle {
            handle,
            receipt_digest: super::field(input, "receipt_digest")?,
        },
        _ => anyhow::bail!("unsupported sender selection"),
    };
    let mut traffic = ReaderTraffic::default();
    let mut stream = application_reader_before(until, &mut traffic)?;
    write_application_request(
        &mut stream,
        &ApplicationReaderRequest::SenderOrdinalV1 {
            request: Box::new(request),
        },
    )?;
    let result = read_response(&mut stream)?;
    drop(stream);
    let mut row = match result {
        ReaderResponse::SenderPendingV1 { handle: h } if h == handle => {
            json!({"outcome":"pending"})
        }
        ReaderResponse::SenderAcceptedV1 { handle: h, receipt }
            if h == handle && receipt.receipt.message_id == handle.message_id()? =>
        {
            json!({"outcome":"accepted","receipt_digest":hex::encode(SenderSettlementV1::receipt_digest(&receipt)?)})
        }
        ReaderResponse::SenderSettledV1 {
            handle: h,
            retired_through,
        } if h == handle => json!({"outcome":"settled","retired_through":retired_through}),
        ReaderResponse::Refused { reason } => return Err(reason.into()),
        _ => anyhow::bail!("uncorrelated original sender response"),
    };
    row["message_id"] = json!(hex::encode(handle.message_id()?.as_bytes()));
    row["agent_request_bytes"] = json!(traffic.written);
    row["agent_reply_bytes"] = json!(traffic.read);
    Ok(row)
}

pub(super) fn empty(input: &Value, until: Instant) -> Result<Value> {
    let n = input["ordinal_empty_control"]
        .as_u64()
        .context("missing ordinal")?;
    ensure!(
        (1..=2049).contains(&n) && input.as_object().is_some_and(|v| v.len() == 1),
        "invalid empty control"
    );
    let mut traffic = ReaderTraffic::default();
    ensure!(
        receive_before(until, &mut traffic)?.is_none(),
        "B held a delivery before the new ordinal was forwarded"
    );
    ensure!(
        traffic.read > 0 && traffic.written > 0,
        "missing signed Receive exchange"
    );
    Ok(
        json!({"ordinal":n,"empty":true,"agent_request_bytes":traffic.written,"agent_reply_bytes":traffic.read}),
    )
}
