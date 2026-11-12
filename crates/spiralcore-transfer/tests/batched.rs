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

#[test]
fn capped_raw_fits_each_frame_and_preserves_legacy_wire_at_the_default_cap() {
    use spiralcore_transfer::client::{transfer_raw_capped_batched, RawFrameLimits};
    let body = bytes(4096);
    for (cap, payloads) in [(4152, vec![4091, 5]), (4157, vec![4096])] {
        let root = Temp::new();
        let mut endpoint = root.endpoint();
        let mut sent = Vec::new();
        let mut returned = Vec::new();
        let stats = transfer_raw_capped_batched(
            &body,
            [45; 32],
            false,
            RawFrameLimits {
                request_bytes: cap,
                reply_bytes: 88,
            },
            |frames| {
                sent.extend_from_slice(frames);
                let replies = process(&mut endpoint, frames)?;
                returned.extend_from_slice(&replies);
                Ok(replies)
            },
        )
        .unwrap();
        assert_eq!(stats.data_bytes, 4096);
        assert_eq!(
            endpoint.read_object(&context(), stats.object).unwrap(),
            body
        );
        assert_eq!(
            sent.iter()
                .filter(|f| f.kind == 2)
                .map(|f| f.body.len() - 5)
                .collect::<Vec<_>>(),
            payloads
        );
        assert!(sent.iter().all(|f| f.encode().unwrap().len() <= cap));
        assert!(returned.iter().all(|f| f.encode().unwrap().len() <= 88));
        if cap == 4157 {
            let mut legacy = Vec::new();
            let root = Temp::new();
            let mut endpoint = root.endpoint();
            transfer_batched(&body, Mode::Raw, None, [45; 32], false, |frames| {
                legacy.extend_from_slice(frames);
                process(&mut endpoint, frames)
            })
            .unwrap();
            assert_eq!(sent, legacy);
        }
    }
}

#[test]
fn capped_raw_checks_offer_reply_and_sequence_limits_before_exchange() {
    use spiralcore_transfer::client::{transfer_raw_capped_batched, RawFrameLimits};
    for (request_bytes, reply_bytes, len) in [
        (0, 88, 1),
        (101, 88, 0),
        (102, 87, 0),
        (102, 88, 41 * 4094 + 1),
    ] {
        let result = transfer_raw_capped_batched(
            &vec![0; len],
            [46; 32],
            true,
            RawFrameLimits {
                request_bytes,
                reply_bytes,
            },
            |_| panic!("unusable cold fallback reached the carrier"),
        );
        assert!(matches!(result, Err(Error::Limit)));
    }
    let limits = RawFrameLimits {
        request_bytes: 102,
        reply_bytes: 88,
    };
    let result = transfer_raw_capped_batched(&[], [47; 32], true, limits, |frames| {
        Ok(vec![frames[0].reply(103, vec![0; 33])])
    });
    assert!(matches!(result, Err(Error::Limit)));
    let root = Temp::new();
    let mut endpoint = root.endpoint();
    let empty = transfer_raw_capped_batched(&[], [48; 32], true, limits, |frames| {
        process(&mut endpoint, frames)
    })
    .unwrap();
    assert!(endpoint
        .read_object(&context(), empty.object)
        .unwrap()
        .is_empty());
}

#[test]
fn leased_sender_settles_each_validated_reply_before_next_call_and_warm_repeat() {
    use cpu_time::ThreadTime;
    use spiralcore_transfer::client::{transfer_raw_leased, RawFrameLimits};
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    let root = Temp::new();
    let mut receiver = root.endpoint();
    let body = bytes(4096);
    for (id, data_bytes, calls) in [(51, 4096, 4), (52, 0, 1)] {
        let pending = Cell::new(None);
        let start = Instant::now();
        let cpu = ThreadTime::now();
        let mut receiver_cpu = Duration::ZERO;
        let mut request_bytes = 0;
        let mut reply_bytes = 0;
        let stats = transfer_raw_leased(
            &body,
            [id; 32],
            RawFrameLimits {
                request_bytes: 4152,
                reply_bytes: 88,
            },
            |frame| -> Result<_> {
                assert_eq!(pending.replace(Some(frame.seq)), None);
                // Exercise the bytes crossing the application boundary in each direction.
                let encoded = frame.encode()?;
                request_bytes += encoded.len();
                let cpu = ThreadTime::now();
                let reply = receiver
                    .handle(&context(), &Frame::decode(&encoded)?, 0)?
                    .encode()?;
                receiver_cpu += cpu.elapsed();
                reply_bytes += reply.len();
                Ok((Frame::decode(&reply)?, frame.seq))
            },
            |lease| {
                assert_eq!(pending.take(), Some(lease));
                Ok(())
            },
        )
        .unwrap();
        let until_settled = start.elapsed();
        let through_settlement_cpu = cpu.elapsed();
        assert_eq!(
            (stats.request_bytes, stats.response_bytes),
            (request_bytes as u64, reply_bytes as u64)
        );
        assert_eq!(stats.data_bytes, data_bytes);
        assert_eq!(stats.requests, calls);
        assert!(pending.get().is_none());
        assert_eq!(
            receiver.read_object(&context(), stats.object).unwrap(),
            body
        );
        eprintln!("data={data_bytes} call_bytes={request_bytes} reply_bytes={reply_bytes} through_settlement_us={} through_readback_us={} sender_and_harness_cpu_us={} receiver_handle_cpu_us={}",
            until_settled.as_micros(), start.elapsed().as_micros(),
            through_settlement_cpu.saturating_sub(receiver_cpu).as_micros(), receiver_cpu.as_micros());
    }
}

#[test]
fn leased_sender_keeps_malformed_or_unrelated_reply_unsettled() {
    use spiralcore_transfer::client::{transfer_raw_leased, RawFrameLimits};
    for case in [
        "transfer",
        "seq",
        "digest",
        "oversized",
        "mode",
        "count",
        "bad-refusal",
        "published-on-data",
    ] {
        let root = Temp::new();
        let mut receiver = root.endpoint();
        let mut settled = Vec::new();
        let result = transfer_raw_leased(
            &bytes(4096),
            [53; 32],
            RawFrameLimits {
                request_bytes: 4152,
                reply_bytes: 88,
            },
            |request| -> Result<_> {
                let mut reply = receiver.handle(&context(), request, 0)?;
                match case {
                    "transfer" => reply.transfer[0] ^= 1,
                    "seq" => reply.seq += 1,
                    "digest" => reply = request.reply(103, vec![0; 32]),
                    "oversized" => reply = request.reply(103, vec![1; 33]),
                    "mode" => reply.body[0] ^= 1,
                    "count" if request.kind == 2 => reply.body = 1u64.to_le_bytes().to_vec(),
                    "bad-refusal" => reply = request.reply(127, vec![2, 0, 0]),
                    "published-on-data" if request.kind == 2 => {
                        reply = request.reply(103, object_id(&bytes(4096)).to_vec())
                    }
                    _ => (),
                }
                Ok((reply, request.seq))
            },
            |lease| {
                settled.push(lease);
                Ok(())
            },
        );
        assert!(result.is_err(), "{case}");
        assert_eq!(
            settled,
            if matches!(case, "count" | "published-on-data") {
                vec![0]
            } else {
                vec![]
            },
            "{case}"
        );
    }
}

#[test]
fn leased_sender_settles_valid_refusal_and_stops_on_ambiguous_ack() {
    use spiralcore_transfer::client::{transfer_raw_leased, RawFrameLimits};
    let limits = RawFrameLimits {
        request_bytes: 4152,
        reply_bytes: 88,
    };
    for (refuse, fail_ack, expected) in [
        (true, false, Error::Unauthorized),
        (true, true, Error::Storage),
        (false, true, Error::Storage),
    ] {
        let mut exchanged = 0;
        let mut acknowledged = 0;
        let body = b"exact\0";
        let result = transfer_raw_leased(
            body,
            [54; 32],
            limits,
            |req| -> Result<_> {
                exchanged += 1;
                let reply = if refuse {
                    req.reply(127, (Error::Unauthorized as u16).to_le_bytes().to_vec())
                } else {
                    req.reply(103, object_id(body).to_vec())
                };
                Ok((reply, 9))
            },
            |lease| {
                assert_eq!(lease, 9);
                acknowledged += 1;
                if fail_ack {
                    Err(Error::Storage)
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(result.unwrap_err(), expected);
        assert_eq!((exchanged, acknowledged), (1, 1));
    }
}

#[test]
fn leased_whole_zstd_counts_exact_encoded_fragments_even_when_they_expand() {
    use spiralcore_transfer::client::{transfer_zstd_whole_leased, RawFrameLimits};
    use spiralcore_transfer::zstd_whole::PreparedEncoder;
    use std::cell::Cell;
    let mut encoder = PreparedEncoder::new().unwrap();
    for original in [vec![], bytes(4096), vec![0; 4096]] {
        let tmp = Temp::new();
        let mut endpoint = tmp.endpoint();
        let expected = encoder.encode(&original).unwrap();
        let limits = RawFrameLimits {
            request_bytes: 4152,
            reply_bytes: 88,
        };
        for (id, cold) in [(61, true), (62, false)] {
            let mut received = Vec::new();
            let pending = Cell::new(None);
            let stats = transfer_zstd_whole_leased(
                &mut encoder,
                &original,
                [id; 32],
                limits,
                |request| -> Result<_> {
                    assert_eq!(pending.replace(Some(request.seq)), None);
                    assert!(request.encoded_len()? <= limits.request_bytes);
                    if request.kind == 2 {
                        assert_eq!(request.body[0], Mode::ZstdWholeV1 as u8);
                        assert_eq!(request.body[5], 1);
                        received.extend_from_slice(&request.body[6..]);
                    }
                    Ok((endpoint.handle(&context(), request, 0)?, request.seq))
                },
                |seq| {
                    assert_eq!(pending.take(), Some(seq));
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(stats.mode, Mode::ZstdWholeV1);
            assert_eq!(stats.object, object_id(&original));
            assert_eq!(received, if cold { expected.as_slice() } else { &[] });
            assert_eq!(stats.data_bytes, received.len() as u64);
            assert_eq!(
                stats.requests,
                if cold {
                    2 + received.len().div_ceil(4090)
                } else {
                    1
                }
            );
            assert_eq!(
                endpoint.read_object(&context(), stats.object).unwrap(),
                original
            );
            assert!(pending.get().is_none());
        }
    }
}

#[test]
fn leased_whole_zstd_refuses_more_fragments_than_receiver_can_accept() {
    use spiralcore_transfer::client::{transfer_zstd_whole_leased, RawFrameLimits};
    use spiralcore_transfer::zstd_whole::PreparedEncoder;
    let result: Result<_> = transfer_zstd_whole_leased(
        &mut PreparedEncoder::new().unwrap(),
        &bytes(33_000),
        [63; 32],
        RawFrameLimits {
            request_bytes: 102,
            reply_bytes: 88,
        },
        |_| -> Result<(Frame, ())> { panic!("unusable offer reached the carrier") },
        |_| panic!("no lease to acknowledge"),
    );
    assert!(matches!(result, Err(Error::Limit)));
}
