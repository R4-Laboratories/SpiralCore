//! Observe a real warm Published reply, withhold one matching ACK success in A,
//! then retry that original signed lease while both original guests stay live.
use super::*;

fn request(op: &str, input: Value) -> Value {
    json!({"object_ack_control":{"operation":op,"input":input}})
}

fn same_original(first: &Value, row: &Value, expected: &str) -> Result<()> {
    ensure!(
        row["outcome"] == expected
            && ["source_boot", "source_instance", "signed_lease_sha256"]
                .iter()
                .all(|k| row[*k].is_string() && row[*k] == first[*k]),
        "changed original boot, worker, lease or control disposition"
    );
    Ok(())
}

fn only_ack(delta: &[Value; 2]) -> Result<()> {
    for role in delta {
        for channel in ["sealed_in", "sealed_out"] {
            for field in FIELDS {
                ensure!(
                    role[channel][field].as_u64() == Some(0),
                    "settlement generated sealed peer traffic"
                );
            }
        }
    }
    for field in ["read", "write", "turns"] {
        ensure!(
            delta[0]["guest_5257"][field]
                .as_u64()
                .is_some_and(|n| n > 0),
            "missing original A guest reader exchange"
        );
    }
    Ok(())
}

pub(super) fn measure(
    pair: &str,
    successful: &Value,
    meters: &[Option<Meters>; 2],
    initial: [Value; 2],
    history: &mut fs::File,
) -> Result<Value> {
    let start = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let until = start + Duration::from_secs(30);
    let tx = hex::encode(rand::random::<[u8; 32]>());
    let payload =
        |tx: &str| json!({"pair":pair,"transfer":tx,"payload_hex":hex::encode(BODY),"codec":"raw"});
    let settle = |seq: u64| json!({"pair":pair,"transfer":tx,"seq":seq});
    let mut a = workload::Call::start(A, &request("withhold", payload(&tx)), until)?;
    let mut b = workload::Call::start(B, &json!({"object_receive":settle(0)}), until)?;
    let mut peer = b.wait()?;
    ensure!(
        peer["receiver_boot"] == successful["receiver_boot"]
            && object_repeat::check_turn(
                &peer,
                &tx,
                0,
                successful["receiver_instance"]
                    .as_str()
                    .context("missing B worker")?,
                BODY
            )?,
        "original B readback required"
    );
    peer.as_object_mut()
        .context("invalid B result")?
        .remove("readback_hex");
    peer["exact_readback_bytes"] = json!(BODY.len());
    let observed = a.wait()?;
    ensure!(
        observed["source_boot"] == successful["source_boot"]
            && observed["outcome"] == "application_ack_withheld"
            && observed["reply_kind"] == 103,
        "missing genuine original Published acknowledgment"
    );
    let first_io: [u64; 2] =
        serde_json::from_value(observed["initial_ack_reader_write_read"].clone())?;
    ensure!(
        first_io.iter().all(|&n| n > 0),
        "original ACK must cross its actual accepted reader socket"
    );
    for (field, len) in [("signed_lease_sha256", 32), ("source_instance", 16)] {
        let s = observed[field]
            .as_str()
            .context("missing original identity")?;
        ensure!(
            s.len() == 2 * len && hex::encode(hex::decode(s)?) == s,
            "invalid original identity"
        );
    }
    let lost = boundary(meters)?;
    let loss_counts = phase_counts(&initial, &lost)?;
    event(
        history,
        &json!({"stage":"ack_return_withheld","a":observed,"b":peer,"transport_delta":loss_counts}),
    )?;
    let mut controls = Vec::new();
    for (op, input, expected) in [
        ("refuse_wrong", settle(1), "wrong_sequence_refused"),
        (
            "refuse_new",
            payload(&hex::encode(rand::random::<[u8; 32]>())),
            "pending_blocks_new_send",
        ),
        ("settle", settle(0), "original_lease_settled"),
        ("refuse_replay", settle(0), "empty_slot_refused"),
    ] {
        let before = boundary(meters)?;
        let row = workload::Call::start(A, &request(op, input), until)?.wait()?;
        same_original(&observed, &row, expected)?;
        let delta = counter_deltas(&before, &boundary(meters)?)?;
        if op == "settle" {
            only_ack(&delta)?;
            ensure!(
                row["settled_reply_seq"] == 0
                    && ["ack_to_agent", "ack_from_agent"]
                        .iter()
                        .all(|k| row[*k].as_u64().is_some_and(|n| n > 0)),
                "missing original positive retry"
            );
        }
        event(
            history,
            &json!({"stage":op,"original":row,"transport_delta":delta}),
        )?;
        controls.push(row);
    }
    let recovery_counts = counter_deltas(&lost, &boundary(meters)?)?;
    only_ack(&recovery_counts)?;
    Ok(
        json!({"first_ack":observed,"b":peer,"controls":controls,"withheld_phase_meters_a_b":loss_counts,"recovery_meters_a_b":recovery_counts,"wall_ns":start.elapsed().as_nanos() as u64,"coordinator_cpu_ns":cpu.elapsed().as_nanos() as u64}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_requires_same_signed_lease_and_zero_sealed_turns() {
        let a = json!({"source_boot":"b","source_instance":"x","signed_lease_sha256":"a","outcome":"ok"});
        let mut b = a.clone();
        assert!(same_original(&a, &b, "ok").is_ok());
        b["signed_lease_sha256"] = json!("q");
        assert!(same_original(&a, &b, "ok").is_err());
        let zero = json!({"read":0,"write":0,"turns":0,"read_errors":0,"write_errors":0});
        let role =
            json!({"guest_5257":{"read":1,"write":2,"turns":1},"sealed_in":zero,"sealed_out":zero});
        let mut counts = [role.clone(), role];
        assert!(only_ack(&counts).is_ok());
        counts[1]["sealed_in"]["turns"] = json!(1);
        assert!(only_ack(&counts).is_err());
    }
}
