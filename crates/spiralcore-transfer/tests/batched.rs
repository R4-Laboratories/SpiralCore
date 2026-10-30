use spiralcore_transfer::{
    client::{transfer, transfer_batched},
    dag::Graph,
    endpoint::{Authority, Context, Endpoint, Limits},
    object_id,
    protocol::{Frame, Mode},
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
            "spiral-batched-{}-{}",
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
fn cold_raw_and_zstd_each_use_one_exact_batch() {
    for mode in [Mode::Raw, Mode::Zstd] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let payload = vec![29u8; 131073];
        let mut calls = 0;
        let mut uploaded = 0;
        let mut received = 0;
        let mut count = 0;
        let stats = transfer_batched(&payload, mode, None, [1; 32], false, |frames| {
            calls += 1;
            assert_eq!(frames.first().unwrap().kind, 1);
            assert_eq!(frames.last().unwrap().kind, 3);
            assert!(frames.len() > 2);
            count += frames.len();
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
        assert_eq!(calls, 1);
        assert_eq!(stats.requests, count);
        assert_eq!(stats.request_bytes, uploaded);
        assert_eq!(stats.response_bytes, received);
        assert_eq!(
            endpoint
                .read_object(&context(), object_id(&payload))
                .unwrap(),
            payload
        );
    }
}

#[test]
fn warm_probe_skips_data_but_unprobed_cache_hit_counts_the_actual_upload() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(100000);
    transfer(&payload, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let mut calls = 0;
    let hit = transfer_batched(&payload, Mode::Raw, None, [2; 32], true, |frames| {
        calls += 1;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].kind, 1);
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(hit.requests, 1);
    assert_eq!(hit.data_bytes, 0);
    let mut uploaded = 0;
    let mut count = 0;
    let unprobed = transfer_batched(&payload, Mode::Raw, None, [3; 32], false, |frames| {
        count = frames.len();
        assert!(count > 2);
        uploaded = frames
            .iter()
            .map(|f| f.encode().unwrap().len() as u64)
            .sum::<u64>();
        let replies = process(&mut endpoint, frames)?;
        assert_eq!(replies.len(), 1);
        Ok(replies)
    })
    .unwrap();
    assert_eq!(unprobed.requests, count);
    assert_eq!(unprobed.request_bytes, uploaded);
    assert_eq!(unprobed.data_bytes, payload.len() as u64);
    assert!(unprobed.request_bytes > hit.request_bytes + payload.len() as u64);
}

#[test]
fn warm_dag_change_is_exact_and_repairs_missing_descendant_closure() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(32768);
    let mut changed = payload.clone();
    changed[1024..1056].fill(255);
    transfer(&payload, Mode::Raw, None, [1; 32], |f| {
        endpoint.handle(&context(), f, 0)
    })
    .unwrap();
    let base = Graph::fold(&payload, 262144).unwrap();
    let target = Graph::fold(&changed, 262144).unwrap();
    // Simulate a sender whose advertised base inventory is optimistic. Its base identity still
    // resolves to the original receiver publication, but it initially suppresses the changed nodes.
    let optimistic = Graph {
        manifest: base.manifest.clone(),
        nodes: target.nodes.clone(),
    };
    let mut calls = 0;
    let mut needs = 0;
    let stats = transfer_batched(
        &changed,
        Mode::Dag,
        Some(&optimistic),
        [2; 32],
        true,
        |frames| {
            calls += 1;
            let replies = process(&mut endpoint, frames)?;
            needs += replies.iter().filter(|r| r.kind == 104).count();
            Ok(replies)
        },
    )
    .unwrap();
    assert_eq!(needs, 1);
    assert_eq!(calls, 3);
    assert!(stats.records_sent > 1);
    assert_eq!(
        endpoint
            .read_object(&context(), object_id(&changed))
            .unwrap(),
        changed
    );
}

#[test]
fn missing_or_extra_nonterminal_replies_are_rejected() {
    for extra in [false, true] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let payload = bytes(1000);
        let result = transfer_batched(&payload, Mode::Raw, None, [1; 32], false, |frames| {
            let mut replies = process(&mut endpoint, frames)?;
            if extra {
                replies.push(replies.last().unwrap().clone());
            } else {
                replies.pop();
            }
            Ok(replies)
        });
        assert!(matches!(result, Err(Error::Integrity)), "{result:?}");
    }
}

#[test]
fn published_on_data_is_not_a_valid_terminal_prefix() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let payload = bytes(1000);
    let result = transfer_batched(&payload, Mode::Raw, None, [1; 32], false, |frames| {
        let ready = endpoint.handle(&context(), &frames[0], 0)?;
        Ok(vec![
            ready,
            frames[1].reply(103, object_id(&payload).to_vec()),
        ])
    });
    assert!(matches!(result, Err(Error::Integrity)));
}

#[test]
fn reply_identity_ready_ack_and_published_digest_are_checked() {
    for fault in 0..6 {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let payload = bytes(1024);
        let result = transfer_batched(&payload, Mode::Raw, None, [1; 32], false, |frames| {
            let mut replies = process(&mut endpoint, frames)?;
            match fault {
                0 => replies[0].transfer[0] ^= 1,
                1 => replies[1].seq += 1,
                2 => replies[0].body[0] = Mode::Dag as u8,
                3 => replies[1].body = u64::MAX.to_le_bytes().to_vec(),
                4 => replies.last_mut().unwrap().body[0] ^= 1,
                5 => replies.clear(),
                _ => unreachable!(),
            }
            Ok(replies)
        });
        assert!(
            matches!(result, Err(Error::Integrity)),
            "fault {fault}: {result:?}"
        );
    }
}

#[test]
fn terminal_error_prefix_is_allowed_but_replies_after_it_are_rejected() {
    for extra in [false, true] {
        let t = Temp::new();
        let mut endpoint = Endpoint::open(
            &t.0,
            Limits {
                resident_bytes: 1,
                ..Limits::default()
            },
            Policy,
        )
        .unwrap();
        let result = transfer_batched(b"quota", Mode::Raw, None, [1; 32], false, |frames| {
            assert_eq!(frames.len(), 3);
            let mut replies = process(&mut endpoint, frames)?;
            assert_eq!(replies.len(), 1);
            assert_eq!(replies[0].kind, 127);
            if extra {
                replies.push(frames[1].reply(102, 5u64.to_le_bytes().to_vec()));
            }
            Ok(replies)
        });
        assert_eq!(
            result.unwrap_err(),
            if extra {
                Error::Integrity
            } else {
                Error::Limit
            }
        );
        assert_eq!(
            endpoint.read_object(&context(), object_id(b"quota")),
            Err(Error::Unknown)
        );
    }
}

#[test]
fn peer_cannot_request_a_record_outside_the_offered_graph() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let mut calls = 0;
    let result = transfer_batched(
        b"scoped repair",
        Mode::Dag,
        None,
        [1; 32],
        false,
        |frames| {
            calls += 1;
            let mut replies = process(&mut endpoint, frames)?;
            let last = replies.last_mut().unwrap();
            last.kind = 104;
            last.body = 1u16.to_le_bytes().to_vec();
            last.body.extend_from_slice(&[99; 32]);
            Ok(replies)
        },
    );
    assert_eq!(result.unwrap_err(), Error::Integrity);
    assert_eq!(
        calls, 1,
        "foreign record request must not cause another upload"
    );
}

#[test]
fn unsuccessful_repair_stops_after_two_rounds() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    let mut calls = 0;
    let result = transfer_batched(&bytes(1024), Mode::Dag, None, [1; 32], false, |frames| {
        calls += 1;
        let delivered: Vec<_> = frames
            .iter()
            .cloned()
            .map(|mut frame| {
                if frame.kind == 2 {
                    frame.body = vec![2, 0, 0, 0, 0, 0, 0];
                }
                frame
            })
            .collect();
        process(&mut endpoint, &delivered)
    });
    assert_eq!(result.unwrap_err(), Error::Missing);
    assert_eq!(calls, 3, "initial upload plus at most two repair rounds");
}

#[test]
fn oversized_campaign_object_is_rejected_before_exchange() {
    let payload = vec![0; 16 * 1024 * 1024 + 1];
    let result = transfer_batched(&payload, Mode::Raw, None, [1; 32], false, |_| {
        panic!("oversized upload")
    });
    assert_eq!(result.unwrap_err(), Error::Limit);
}

#[test]
fn empty_object_uses_begin_and_finish_without_fabricated_data() {
    for warm_probe in [false, true] {
        let t = Temp::new();
        let mut endpoint = t.endpoint();
        let mut calls = 0;
        let stats = transfer_batched(&[], Mode::Raw, None, [1; 32], warm_probe, |frames| {
            calls += 1;
            assert!(frames.iter().all(|f| f.kind != 2));
            process(&mut endpoint, frames)
        })
        .unwrap();
        assert_eq!(stats.requests, 2);
        assert_eq!(calls, if warm_probe { 2 } else { 1 });
        assert_eq!(
            endpoint.read_object(&context(), stats.object).unwrap(),
            Vec::<u8>::new()
        );
    }
}

#[test]
fn large_dag_is_split_with_bounded_batches_and_exact_sequence() {
    let t = Temp::new();
    let mut endpoint = t.endpoint();
    // Nearly 246k unique records exceed both a single 512-frame window and 20 MiB envelope.
    let payload = bytes(5 * 1024 * 1024);
    let mut batches = Vec::new();
    let mut expected_seq = 0;
    let mut request_bytes = 0;
    let stats = transfer_batched(&payload, Mode::Dag, None, [1; 32], false, |frames| {
        let size = 12
            + frames
                .iter()
                .map(|f| 4 + f.encode().unwrap().len())
                .sum::<usize>();
        batches.push((frames.len(), size));
        for frame in frames {
            assert_eq!(frame.seq, expected_seq);
            expected_seq += 1;
            request_bytes += frame.encode().unwrap().len() as u64;
        }
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert!(batches.len() > 1, "must exercise a real split");
    assert_eq!(stats.requests as u64, expected_seq);
    assert_eq!(stats.request_bytes, request_bytes);
    assert_eq!(
        endpoint.read_object(&context(), stats.object).unwrap(),
        payload
    );
}
