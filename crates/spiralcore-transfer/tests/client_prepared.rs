use spiralcore_transfer::{
    client::{transfer, transfer_batched, transfer_prepared_batched, transfer_sparse_batched},
    endpoint::{Authority, Context, Endpoint, Limits},
    object_id,
    prepared::PreparedObject,
    protocol::{Frame, Mode, Offer},
    Error, Result,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-client-prepared-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn endpoint(&self) -> Endpoint<Policy> {
        Endpoint::open(&self.0, Limits::default(), Policy).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn context() -> Context {
    Context {
        tenant: "batch-tests".into(),
        domain: "public".into(),
        sender: "s".into(),
        recipient: "r".into(),
        mailbox: "m".into(),
        capability: "c".into(),
        authorization_scope: "objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
struct Policy;
impl Authority for Policy {
    fn authorize(&self, c: &Context) -> Result<()> {
        if *c == context() {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn process(endpoint: &mut Endpoint<Policy>, frames: &[Frame]) -> Result<Vec<Frame>> {
    assert!(!frames.is_empty());
    assert!(frames.len() <= 512);
    assert!(
        12 + frames
            .iter()
            .map(|f| 4 + f.encode().unwrap().len())
            .sum::<usize>()
            <= 20 * 1024 * 1024
    );
    let mut replies = Vec::new();
    for frame in frames {
        let reply = endpoint
            .handle(&context(), frame, 0)
            .unwrap_or_else(|e| frame.reply(127, (e as u16).to_le_bytes().to_vec()));
        let stop = matches!(reply.kind, 103 | 127);
        replies.push(reply);
        if stop {
            break;
        }
    }
    Ok(replies)
}
fn bytes(n: usize) -> Vec<u8> {
    let mut state = 0x123456789abcdefu64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

#[test]
fn prepared_sender_matches_canonical_offer_and_actual_accounting() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let base_bytes = bytes(32768);
    transfer(&base_bytes, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let base = PreparedObject::new(&base_bytes, 262144).unwrap();
    let mut target = base_bytes.clone();
    target[1024..1056].fill(255);
    let delta = base.prepare_delta(&target).unwrap();
    let mut uploaded = 0;
    let mut received = 0;
    let mut requests = 0;
    let stats = transfer_prepared_batched(&delta, [2; 32], true, |frames| {
        if frames[0].kind == 1 {
            let offer = Offer::decode(&frames[0].body)?;
            assert_eq!(offer.mode, Mode::Dag);
            assert_eq!(offer.base, Some(base.graph().manifest().id()));
            assert_eq!(offer.manifest.as_ref(), Some(delta.manifest()));
        }
        requests += frames.len();
        uploaded += frames
            .iter()
            .map(|f| f.encode().unwrap().len() as u64)
            .sum::<u64>();
        let replies = process(&mut endpoint, frames)?;
        received += replies
            .iter()
            .map(|f| f.encode().unwrap().len() as u64)
            .sum::<u64>();
        Ok(replies)
    })
    .unwrap();
    assert_eq!(stats.mode, Mode::Dag);
    assert_eq!(stats.records_sent, delta.new_records().len());
    assert_eq!(stats.requests, requests);
    assert_eq!(stats.request_bytes, uploaded);
    assert_eq!(stats.response_bytes, received);
    assert_eq!(
        endpoint
            .read_object(&context(), object_id(&target))
            .unwrap(),
        target
    );
}

#[test]
fn prepared_repeat_uses_raw_identity_probe() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(8192);
    transfer(&payload, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let base = PreparedObject::new(&payload, 262144).unwrap();
    let repeated = base.prepare_delta(&payload).unwrap();
    let stats = transfer_prepared_batched(&repeated, [2; 32], true, |frames| {
        assert_eq!(frames.len(), 1);
        let offer = Offer::decode(&frames[0].body)?;
        assert_eq!(offer.mode, Mode::Raw);
        assert!(offer.manifest.is_none() && offer.base.is_none());
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert_eq!(stats.mode, Mode::Raw);
    assert_eq!(stats.requests, 1);
    assert_eq!(stats.records_sent, 0);
    assert_eq!(stats.data_bytes, 0);
}

#[test]
fn prepared_missing_descendants_are_repaired_from_target_closure() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(8192);
    transfer(&payload, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let base = PreparedObject::new(&payload, 262144).unwrap();
    let mut target = payload.clone();
    target[123] ^= 1;
    let delta = base.prepare_delta(&target).unwrap();
    let mut omitted = false;
    let mut needs = 0;
    let stats = transfer_prepared_batched(&delta, [2; 32], true, |frames| {
        let mut delivered = frames.to_vec();
        for frame in &mut delivered {
            if frame.kind == 2 && !omitted {
                frame.body = vec![2, 0, 0, 0, 0, 0, 0];
                omitted = true;
            }
        }
        let replies = process(&mut endpoint, &delivered)?;
        needs += replies.iter().filter(|reply| reply.kind == 104).count();
        Ok(replies)
    })
    .unwrap();
    assert!(omitted);
    assert_eq!(needs, 1);
    assert!(stats.records_sent > delta.new_records().len());
    assert_eq!(
        endpoint.read_object(&context(), stats.object).unwrap(),
        target
    );
}

#[test]
fn prepared_peer_cannot_request_an_unreachable_old_base_record() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(32);
    transfer(&payload, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let base = PreparedObject::new(&payload, 100).unwrap();
    let old_id = base.graph().manifest().roots[0].1;
    let mut target = payload.clone();
    target[0] ^= 1;
    let delta = base.prepare_delta(&target).unwrap();
    let mut calls = 0;
    let result = transfer_prepared_batched(&delta, [2; 32], false, |frames| {
        calls += 1;
        let mut replies = process(&mut endpoint, frames)?;
        let last = replies.last_mut().unwrap();
        assert_eq!(last.kind, 103);
        last.kind = 104;
        last.body = 1u16.to_le_bytes().to_vec();
        last.body.extend_from_slice(&old_id);
        Ok(replies)
    });
    assert_eq!(result.unwrap_err(), Error::Integrity);
    assert_eq!(calls, 1, "unreachable base record was sent as repair");
}

#[test]
fn prepared_cache_hit_counts_every_already_uploaded_frame() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(8192);
    let mut target = payload.clone();
    target[17] ^= 1;
    for (i, data) in [&payload, &target].into_iter().enumerate() {
        transfer(data, Mode::Raw, None, [i as u8 + 1; 32], |f| {
            endpoint.handle(&context(), f, 0)
        })
        .unwrap();
    }
    let base = PreparedObject::new(&payload, 1000).unwrap();
    let delta = base.prepare_delta(&target).unwrap();
    let mut uploaded = 0;
    let mut count = 0;
    let stats = transfer_prepared_batched(&delta, [3; 32], false, |frames| {
        count += frames.len();
        uploaded += frames
            .iter()
            .map(|f| f.encode().unwrap().len() as u64)
            .sum::<u64>();
        let replies = process(&mut endpoint, frames)?;
        assert_eq!(replies.len(), 1);
        Ok(replies)
    })
    .unwrap();
    assert_eq!(stats.requests, count);
    assert_eq!(stats.request_bytes, uploaded);
    assert!(count > 2);
    assert_eq!(stats.records_sent, delta.new_records().len());
}

#[test]
fn sparse_sender_handles_edits_insertions_and_deletions_exactly() {
    let base = bytes(32768);
    let mut edit = base.clone();
    edit[123..155].fill(255);
    let mut inserted = base.clone();
    inserted.splice(1000..1000, [9, 0, 8]);
    let mut deleted = base.clone();
    deleted.drain(500..507);
    for target in [edit, inserted, deleted] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        transfer(&base, Mode::Raw, None, [1; 32], |f| {
            endpoint.handle(&context(), f, 0)
        })
        .unwrap();
        let mut uploaded = 0;
        let mut received = 0;
        let mut requests = 0;
        let stats = transfer_sparse_batched(&base, &target, [2; 32], false, |frames| {
            let offer = Offer::decode(&frames[0].body)?;
            assert_eq!(offer.mode, Mode::SparseV1);
            assert_eq!(offer.base, Some(object_id(&base)));
            assert!(offer.manifest.is_none());
            requests += frames.len();
            uploaded += frames
                .iter()
                .map(|f| f.encode().unwrap().len() as u64)
                .sum::<u64>();
            let replies = process(&mut endpoint, frames)?;
            received += replies
                .iter()
                .map(|f| f.encode().unwrap().len() as u64)
                .sum::<u64>();
            Ok(replies)
        })
        .unwrap();
        assert_eq!(stats.mode, Mode::SparseV1);
        assert_eq!(stats.records_sent, 0);
        assert_eq!(stats.requests, requests);
        assert_eq!(stats.request_bytes, uploaded);
        assert_eq!(stats.response_bytes, received);
        assert!(stats.request_bytes < target.len() as u64 / 4);
        assert_eq!(
            endpoint.read_object(&context(), stats.object).unwrap(),
            target
        );
    }
}

#[test]
fn generic_senders_reject_sparse_without_the_owned_base_bytes_api() {
    let single = transfer(b"abc", Mode::SparseV1, None, [1; 32], |_| {
        panic!("unexpected exchange")
    });
    assert_eq!(single.unwrap_err(), Error::Unsupported);
    let batch = transfer_batched(b"abc", Mode::SparseV1, None, [1; 32], true, |_| {
        panic!("unexpected exchange")
    });
    assert_eq!(batch.unwrap_err(), Error::Unsupported);
}

#[test]
fn sparse_raw_fallback_happens_before_exchange_without_requiring_remote_base() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let base = vec![0; 4096];
    let target = vec![1; 4096];
    assert!(spiralcore_transfer::delta::encode(&base, &target)
        .unwrap()
        .is_none());
    let stats = transfer_sparse_batched(&base, &target, [1; 32], false, |frames| {
        let offer = Offer::decode(&frames[0].body)?;
        assert_eq!(offer.mode, Mode::Raw);
        assert!(offer.base.is_none());
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert_eq!(stats.mode, Mode::Raw);
    assert_eq!(
        endpoint.read_object(&context(), stats.object).unwrap(),
        target
    );
}

#[test]
fn missing_base_is_an_error_without_reoffering_the_transfer_id() {
    let base = bytes(4096);
    let mut target = base.clone();
    target[101] ^= 1;
    let prepared = PreparedObject::new(&base, 1000).unwrap();
    let delta = prepared.prepare_delta(&target).unwrap();
    for sparse in [false, true] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let mut calls = 0;
        let mut exchange = |frames: &[Frame]| {
            calls += 1;
            assert_eq!(frames.len(), 1);
            process(&mut endpoint, frames)
        };
        let result = if sparse {
            transfer_sparse_batched(&base, &target, [1; 32], true, &mut exchange)
        } else {
            transfer_prepared_batched(&delta, [1; 32], true, &mut exchange)
        };
        assert_eq!(result.unwrap_err(), Error::Missing);
        assert_eq!(calls, 1);
        assert!(endpoint
            .read_object(&context(), object_id(&target))
            .is_err());
    }
}

#[test]
fn sparse_ack_and_need_responses_obey_strict_mode_rules() {
    for bad_ack in [false, true] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let base = bytes(4096);
        transfer(&base, Mode::Raw, None, [1; 32], |f| {
            endpoint.handle(&context(), f, 0)
        })
        .unwrap();
        let mut target = base.clone();
        target[101] ^= 1;
        let result = transfer_sparse_batched(&base, &target, [2; 32], false, |frames| {
            let mut replies = process(&mut endpoint, frames)?;
            if bad_ack {
                let ack = replies.iter_mut().find(|r| r.kind == 102).unwrap();
                ack.body = 0u64.to_le_bytes().to_vec();
            } else {
                let last = replies.last_mut().unwrap();
                last.kind = 104;
                last.body = 1u16.to_le_bytes().to_vec();
                last.body.extend_from_slice(&object_id(&base));
            }
            Ok(replies)
        });
        assert_eq!(result.unwrap_err(), Error::Integrity);
    }
}

#[test]
fn sparse_probe_and_unprobed_cache_hit_have_exact_upload_accounting() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let base = bytes(8192);
    let mut target = base.clone();
    target[17] ^= 1;
    for (i, data) in [&base, &target].into_iter().enumerate() {
        transfer(data, Mode::Raw, None, [i as u8 + 1; 32], |f| {
            endpoint.handle(&context(), f, 0)
        })
        .unwrap();
    }
    for probe in [false, true] {
        let mut uploaded = 0;
        let mut requests = 0;
        let stats = transfer_sparse_batched(
            &base,
            &target,
            [if probe { 4 } else { 3 }; 32],
            probe,
            |frames| {
                requests += frames.len();
                uploaded += frames
                    .iter()
                    .map(|f| f.encode().unwrap().len() as u64)
                    .sum::<u64>();
                let replies = process(&mut endpoint, frames)?;
                assert_eq!(replies.len(), 1);
                Ok(replies)
            },
        )
        .unwrap();
        assert_eq!(stats.requests, requests);
        assert_eq!(stats.request_bytes, uploaded);
        if probe {
            assert_eq!(requests, 1);
            assert_eq!(stats.data_bytes, 0);
        } else {
            assert!(requests > 2);
            assert!(stats.data_bytes > 0);
        }
    }
    let repeated = transfer_sparse_batched(&base, &base, [5; 32], true, |frames| {
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert_eq!(repeated.mode, Mode::Raw);
    assert_eq!(repeated.requests, 1);
    assert_eq!(repeated.data_bytes, 0);
}

#[test]
fn sparse_bounds_are_checked_before_any_exchange() {
    let oversized = vec![0; spiralcore_transfer::client::MAX_BATCHED_OBJECT + 1];
    for (base, target) in [(&oversized[..], &[][..]), (&[][..], &oversized[..])] {
        assert_eq!(
            transfer_sparse_batched(base, target, [1; 32], true, |_| panic!(
                "oversized exchange"
            ))
            .unwrap_err(),
            Error::Limit
        );
    }
}
