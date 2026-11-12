//! Measurements for the bounded repeated-object run.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Instant};

use super::workload::Call;

pub fn count(raw: &str) -> Result<usize> {
    let n: usize = raw.parse()?;
    ensure!((2..=8).contains(&n), "series requires 2 to 8 objects");
    Ok(n)
}

/// The source has staged an immutable operation. Keep that data operation
/// pending at the relay until the original guest responds to this control.
/// These are interleaved operations, not simultaneous writes on one socket.
pub fn admitted_control(
    ordinal: usize,
    source_request: &Value,
    staged_ns: u64,
    start: Instant,
    until: Instant,
) -> Result<Value> {
    let request = json!({"control_probe":ordinal, "direction":source_request["direction"],
        "recipient":source_request["recipient"]});
    let control_start = start.elapsed().as_nanos() as u64;
    let mut call = Call::start(super::A, &request, until)?;
    let mut result = call.wait()?;
    let control_end = start.elapsed().as_nanos() as u64;
    ensure!(
        result["control_probe"] == ordinal
            && ["agent_request_bytes", "agent_reply_bytes"]
                .iter()
                .all(|name| result[*name].as_u64().is_some_and(|bytes| bytes > 0)),
        "source control omitted original agent confirmation"
    );
    result["start_ns"] = json!(control_start);
    result["end_ns"] = json!(control_end);
    result["data_staged_before_control_ns"] = json!(staged_ns);
    result["sdk_request_bytes"] = json!(call.input_bytes);
    result["sdk_reply_bytes"] = json!(call.output_bytes);
    Ok(result)
}

pub fn record(ids: &mut BTreeSet<String>, row: &Value) -> Result<()> {
    let id = row["guest_a"]["message_id"]
        .as_str()
        .context("original ID absent")?;
    let control = &row["interleaved_control"];
    let mark = |key: &str| control[key].as_u64().context("missing control time");
    ensure!(
        mark("data_staged_before_control_ns")? <= mark("start_ns")?
            && mark("start_ns")? <= mark("end_ns")?
            && mark("end_ns")?
                <= row["through_observed_v2_ns"]
                    .as_u64()
                    .context("missing V2 time")?,
        "control outside pending data transaction"
    );
    ensure!(
        ids.insert(id.to_owned()),
        "two distinct operations reused an ID"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_count_and_causal_order() {
        assert!(count("1").is_err());
        assert_eq!(count("8").unwrap(), 8);
        assert!(count("9").is_err());
        let mut ids = BTreeSet::new();
        let mut row = json!({"guest_a":{"message_id":"a"},
            "interleaved_control":{"data_staged_before_control_ns":1,"start_ns":1,"end_ns":3},
            "through_observed_v2_ns":4});
        record(&mut ids, &row).unwrap();
        assert!(record(&mut ids, &row).is_err());
        row["guest_a"]["message_id"] = json!("b");
        row["through_observed_v2_ns"] = json!(2);
        assert!(record(&mut ids, &row).is_err());
    }
}
