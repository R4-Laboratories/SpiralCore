//! Compile-time witness at the return boundary of the real guest-reader ACK.
use super::*;
use anyhow::Context;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Default)]
pub(super) struct WithholdOnce {
    target: Option<(Id, Id)>,
    seen: Option<Id>,
    accepted_reader_bytes: [u64; 2],
    instance: [u8; 16],
}

#[derive(Debug)]
struct Withheld;
impl std::fmt::Display for Withheld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("qualification withheld the matched original ACK from application settlement")
    }
}
impl std::error::Error for Withheld {}

fn fingerprint(lease: &mvm_contract::protocol::mailbox::SignedMailboxDeliveryLease) -> Result<Id> {
    Ok(Sha256::digest(serde_json::to_vec(lease)?).into())
}

impl WithholdOnce {
    pub(super) fn after_original_ack(
        &mut self,
        pair: Id,
        tx: Id,
        seq: u64,
        delivery: &ObjectDelivery,
        reader_bytes: [u64; 2],
    ) -> Result<()> {
        if self.target == Some((pair, tx))
            && self.seen.is_none()
            && seq == 0
            && delivery.reply_frame()?.kind == 103
        {
            ensure!(
                reader_bytes.iter().all(|n| *n > 0),
                "missing original socket ACK bytes"
            );
            self.accepted_reader_bytes = reader_bytes;
            self.seen = Some(fingerprint(delivery.lease())?);
            return Err(Withheld.into());
        }
        Ok(())
    }
}

pub(super) fn control(
    source: &mut GuestObjectSource,
    task: &Value,
    deadline: Instant,
) -> Result<Value> {
    let input = &task["input"];
    ensure!(
        task.as_object().is_some_and(|t| t.len() == 2),
        "invalid ACK control"
    );
    let pair = crate::field(input, "pair")?;
    let tx = crate::field(input, "transfer")?;
    let boot = mvm_core::mailbox::reader::MailboxReaderBootBinding::load_guest_original()?;
    ensure!(
        boot.durable_object_pair_sha256_v1 == Some(pair),
        "different original guest"
    );
    let boot_id = hex::encode(boot.boot.as_bytes());
    let mut result = match task["operation"].as_str() {
        Some("withhold") => {
            ensure!(
                source.loss.target.is_none() && source.unsettled.is_none(),
                "qualification is single use"
            );
            std::fs::File::open("/dev/urandom")?.read_exact(&mut source.loss.instance)?;
            source.loss.target = Some((pair, tx));
            let e = run_source(source, input, deadline)
                .err()
                .context("expected original operation to stop")?;
            ensure!(
                e.is::<Withheld>(),
                "genuine original ACK was not observed: {e:#}"
            );
            let pending = source
                .unsettled
                .as_ref()
                .context("missing original pending Reply")?;
            ensure!(
                (pending.pair, pending.transfer, pending.seq) == (pair, tx, 0)
                    && source.loss.seen == Some(fingerprint(pending.delivery.lease())?),
                "wrong retained lease"
            );
            json!({"outcome":"application_ack_withheld","reply_kind":pending.delivery.reply_frame()?.kind,"initial_ack_reader_write_read":source.loss.accepted_reader_bytes})
        }
        Some("refuse_new") => {
            ensure!(source.unsettled.is_some(), "missing original pending Reply");
            let e = run_source(source, input, deadline)
                .err()
                .context("expected original operation to stop")?;
            ensure!(
                e.to_string() == "original object Reply still needs settlement",
                "unexpected replacement disposition: {e:#}"
            );
            json!({"outcome":"pending_blocks_new_send"})
        }
        Some("refuse_wrong") | Some("refuse_replay") => {
            let wrong = task["operation"] == "refuse_wrong";
            ensure!(
                source.loss.seen.is_some() && wrong == source.unsettled.is_some(),
                "wrong original pending state"
            );
            let e = retry_settlement(source, input, deadline)
                .err()
                .context("expected original operation to stop")?;
            let expected = if wrong {
                "settlement does not identify the pending object Reply"
            } else {
                "no object Reply settlement pending"
            };
            ensure!(
                e.to_string() == expected,
                "unexpected settlement disposition: {e:#}"
            );
            json!({"outcome":if wrong {"wrong_sequence_refused"} else {"empty_slot_refused"}})
        }
        Some("settle") => {
            let original = source
                .unsettled
                .as_ref()
                .context("missing unsettled original")?;
            ensure!(
                source.loss.seen == Some(fingerprint(original.delivery.lease())?),
                "original lease changed"
            );
            let mut r = retry_settlement(source, input, deadline)?;
            ensure!(source.unsettled.is_none(), "original slot still occupied");
            r["outcome"] = json!("original_lease_settled");
            r
        }
        _ => anyhow::bail!("unknown ACK control"),
    };
    result["signed_lease_sha256"] = json!(hex::encode(
        source.loss.seen.context("missing original ACK")?
    ));
    result["source_instance"] = json!(hex::encode(source.loss.instance));
    result["source_boot"] = json!(boot_id);
    Ok(result)
}
