//! An admitted direction-availability query against A's actual guest agent.
use super::field;
use anyhow::{Context, ensure};
use mvm_contract::protocol::mailbox::PrincipalId;
use mvm_core::mailbox::reader_wire::{
    ApplicationReaderRequest, ReaderResponse, read_response, write_application_request,
};
use serde_json::{Value, json};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before};
use std::time::Instant;

pub fn probe(input: &Value, until: Instant) -> anyhow::Result<Value> {
    let ordinal = input["control_probe"]
        .as_u64()
        .context("missing control ordinal")?;
    ensure!(
        (1..=8).contains(&ordinal) && input.as_object().is_some_and(|o| o.len() == 3),
        "invalid control request"
    );
    let direction = field(input, "direction")?;
    let recipient = PrincipalId::from_bytes(field(input, "recipient")?);
    let mut bytes = ReaderTraffic::default();
    let mut stream = application_reader_before(until, &mut bytes)?;
    write_application_request(
        &mut stream,
        &ApplicationReaderRequest::CheckOutboundJournalV1 {
            commitment: direction,
            recipient,
            payload_bytes: super::FRAME as u32,
        },
    )?;
    let reply = read_response(&mut stream)?;
    ensure!(
        matches!(reply, ReaderResponse::OutboundJournalAvailableV1 { commitment } if commitment == direction),
        "original A agent refused this direction"
    );
    drop(stream);
    Ok(
        json!({"control_probe":ordinal,"agent_request_bytes":bytes.written,"agent_reply_bytes":bytes.read}),
    )
}
