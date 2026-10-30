use spiralcore_transfer::{
    client::{self, AdaptiveStats, Stats},
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    object_id,
    protocol::{Frame, Mode, Offer},
    wan::{self, HttpClient},
    Error, Result,
};
use std::{
    collections::BTreeSet,
    fs,
    net::TcpListener,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-adaptive-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn endpoint(&self) -> Endpoint<Policy> {
        Endpoint::open_with_policy(
            &self.0,
            Limits::default(),
            Policy(Arc::new(AtomicBool::new(true))),
            ColdGraphPolicy::Disabled,
        )
        .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[derive(Clone)]
struct Policy(Arc<AtomicBool>);
impl Authority for Policy {
    fn authorize(&self, _: &Context) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn context() -> Context {
    wan::context([1; 32])
}
fn random(n: usize) -> Vec<u8> {
    let mut state = 0x73914fd23a104ce1u64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}
fn process(endpoint: &mut Endpoint<Policy>, frames: &[Frame]) -> Result<Vec<Frame>> {
    process_context(endpoint, &context(), frames)
}
fn process_context(
    endpoint: &mut Endpoint<Policy>,
    c: &Context,
    frames: &[Frame],
) -> Result<Vec<Frame>> {
    let mut replies = Vec::new();
    for frame in frames {
        let reply = endpoint
            .handle(c, frame, 0)
            .unwrap_or_else(|e| frame.reply(127, (e as u16).to_le_bytes().to_vec()));
        let terminal = matches!(reply.kind, 103 | 127);
        replies.push(reply);
        if terminal {
            break;
        }
    }
    Ok(replies)
}
fn body_bytes(requests: &[Frame], replies: &[Frame]) -> u64 {
    // SCWREP01 replaces the SCWBAT01 magic and adds two u64 timing fields.
    (wan::encode_batch(requests).unwrap().len() + wan::encode_batch(replies).unwrap().len() + 16)
        as u64
}
fn seed(endpoint: &mut Endpoint<Policy>, base: &[u8]) {
    client::transfer_batched(base, Mode::Raw, None, [1; 32], false, |f| {
        process(endpoint, f)
    })
    .unwrap();
}
fn adaptive(target: &[u8], base: Option<&[u8]>, probe: bool) -> (AdaptiveStats, u64) {
    let temp = Temp::new();
    let mut endpoint = temp.endpoint();
    if let Some(base) = base {
        seed(&mut endpoint, base);
    }
    let mut bytes = 0;
    let result = client::transfer_adaptive_batched(target, base, [2; 32], probe, |frames| {
        let replies = process(&mut endpoint, frames)?;
        bytes += body_bytes(frames, &replies);
        Ok(replies)
    })
    .unwrap();
    assert_eq!(result.transfer.object, object_id(target));
    assert_eq!(result.transfer.mode, result.selection.selected);
    drop(endpoint);
    let mut reopened = temp.endpoint();
    assert_eq!(
        reopened.read_object(&context(), object_id(target)).unwrap(),
        target
    );
    (result, bytes)
}
fn forced(target: &[u8], base: Option<&[u8]>, mode: Mode, probe: bool) -> (Stats, u64) {
    let temp = Temp::new();
    let mut endpoint = temp.endpoint();
    if let Some(base) = base {
        seed(&mut endpoint, base);
    }
    let mut bytes = 0;
    let exchange = |frames: &[Frame]| {
        let replies = process(&mut endpoint, frames)?;
        bytes += body_bytes(frames, &replies);
        Ok(replies)
    };
    let result = match mode {
        Mode::Raw | Mode::Zstd => {
            client::transfer_batched(target, mode, None, [2; 32], probe, exchange)
        }
        Mode::SparseV1 => {
            client::transfer_sparse_batched(base.unwrap(), target, [2; 32], probe, exchange)
        }
        Mode::ZstdPrefixV1 => {
            client::transfer_zstd_prefix_batched(base.unwrap(), target, [2; 32], probe, exchange)
        }
        Mode::Dag | Mode::RuntimeV1 | Mode::SchemaV1 | Mode::ZstdDictionaryV1 => unreachable!(),
    }
    .unwrap();
    drop(endpoint);
    let mut reopened = temp.endpoint();
    assert_eq!(
        reopened.read_object(&context(), object_id(target)).unwrap(),
        target
    );
    (result, bytes)
}

#[test]
fn cold_structured_data_selects_zstd_before_begin() {
    let target = vec![42; 65537];
    let (result, bytes) = adaptive(&target, None, false);
    assert_eq!(result.transfer.mode, Mode::Zstd);
    assert_eq!(result.selection.selected, Mode::Zstd);
    assert!(result.selection.expected_http_body_bytes < 1000);
    assert_eq!(result.selection.expected_http_body_bytes, bytes);
    assert_eq!(result.selection.candidates.len(), 2);
}

#[test]
fn empty_and_incompressible_ties_prefer_raw_with_exact_full_cost() {
    for target in [vec![], vec![0], random(65537)] {
        for probe in [false, true] {
            let (result, bytes) = adaptive(&target, None, probe);
            assert_eq!(result.transfer.mode, Mode::Raw);
            assert_eq!(result.selection.expected_http_body_bytes, bytes);
            assert!(!result.selection.exact_repeat_probe);
            let raw = &result.selection.candidates[0];
            let zstd = &result.selection.candidates[1];
            assert_eq!(raw.mode, Mode::Raw);
            assert_eq!(zstd.mode, Mode::Zstd);
            assert_eq!(raw.http_body_bytes, zstd.http_body_bytes);
        }
    }
}

#[test]
fn warm_local_edits_choose_sparse_and_reordered_data_choose_prefix() {
    let base = random(1 << 20);
    let mut edited = base.clone();
    edited[524288..524320].fill(0);
    let (sparse, bytes) = adaptive(&edited, Some(&base), false);
    assert_eq!(
        sparse.transfer.mode,
        Mode::SparseV1,
        "{:?}",
        sparse.selection
    );
    assert_eq!(sparse.selection.expected_http_body_bytes, bytes);
    let base = random(131072);
    let rotated = [&base[65536..], &base[..65536]].concat();
    let (prefix, bytes) = adaptive(&rotated, Some(&base), false);
    assert_eq!(prefix.transfer.mode, Mode::ZstdPrefixV1);
    assert_eq!(prefix.selection.expected_http_body_bytes, bytes);
    assert!(bytes < 4096);
}

#[test]
fn small_localized_edit_uses_prefix_when_it_costs_less_than_sparse() {
    let base = random(131072);
    let mut target = base.clone();
    target[65536..65568].fill(0);
    let (result, bytes) = adaptive(&target, Some(&base), false);
    let (_, sparse_bytes) = forced(&target, Some(&base), Mode::SparseV1, false);
    assert_eq!(result.transfer.mode, Mode::ZstdPrefixV1);
    assert!(bytes < sparse_bytes);
    assert_eq!(result.selection.expected_http_body_bytes, bytes);
}

#[test]
fn selector_cost_matches_actual_forced_arms_and_minimizes_eligible_bytes() {
    let base = random(131073);
    let mut localized = base.clone();
    localized[70000] ^= 7;
    let mut scattered = base.clone();
    for byte in scattered.iter_mut().step_by(4096) {
        *byte ^= 0x5a;
    }
    let rotated = [&base[65536..], &base[..65536]].concat();
    for target in [localized, scattered, rotated, vec![29; base.len()]] {
        for probe in [false, true] {
            let (selected, selected_bytes) = adaptive(&target, Some(&base), probe);
            assert_eq!(selected.selection.expected_http_body_bytes, selected_bytes);
            let mut minimum = u64::MAX;
            for mode in [Mode::Raw, Mode::Zstd, Mode::SparseV1, Mode::ZstdPrefixV1] {
                let (fixed, fixed_bytes) = forced(&target, Some(&base), mode, probe);
                minimum = minimum.min(fixed_bytes);
                if fixed.mode == mode {
                    let candidate = selected
                        .selection
                        .candidates
                        .iter()
                        .find(|candidate| candidate.mode == mode)
                        .unwrap();
                    assert_eq!(candidate.http_body_bytes, fixed_bytes, "mode {mode:?}");
                }
            }
            assert_eq!(selected_bytes, minimum);
            let mut expected = &selected.selection.candidates[0];
            for candidate in &selected.selection.candidates[1..] {
                if candidate.http_body_bytes < expected.http_body_bytes {
                    expected = candidate;
                }
            }
            assert_eq!(selected.selection.selected, expected.mode);
        }
    }
}

#[test]
fn exact_bytes_and_length_survive_boundaries_and_length_changes_after_restart() {
    for size in [0, 1, 31, 32, 33, 61439, 61440, 61441] {
        let base = random(size);
        let mut append = base.clone();
        append.extend_from_slice(&[0, 7, 0]);
        let mut insert = base.clone();
        insert.splice(size / 2..size / 2, [0, 3, 0]);
        let mut cases = vec![vec![], append, insert, vec![0; size + 3]];
        if size > 0 {
            cases.push(base[..size - 1].to_vec());
            cases.push(base[1..].to_vec());
        }
        for target in cases {
            // Empty base plus empty target may hit the seeded object; fidelity still applies.
            let (result, bytes) = adaptive(&target, Some(&base), false);
            if target != base {
                assert_eq!(result.selection.expected_http_body_bytes, bytes);
            }
        }
    }
}

#[test]
fn exact_repeat_probe_sends_raw_identity_only_and_unprobed_hit_counts_upload() {
    let target = random(65537);
    let (probe, bytes) = adaptive(&target, Some(&target), true);
    assert!(probe.selection.exact_repeat_probe);
    assert_eq!(probe.transfer.mode, Mode::Raw);
    assert_eq!(probe.transfer.requests, 1);
    assert_eq!(probe.transfer.data_bytes, 0);
    assert_eq!(bytes, 238);
    let (unprobed, uploaded) = adaptive(&target, Some(&target), false);
    assert!(!unprobed.selection.exact_repeat_probe);
    assert!(unprobed.transfer.requests > 1);
    assert!(uploaded > bytes);
    assert!(uploaded < target.len() as u64);
    // Complete-miss estimate includes replies that a Begin cache hit never emits.
    assert!(unprobed.selection.expected_http_body_bytes > uploaded);
}

#[test]
fn oversized_inputs_fail_before_any_exchange() {
    let oversized = vec![0; client::MAX_BATCHED_OBJECT + 1];
    for (target, base) in [
        (oversized.as_slice(), None),
        (b"small".as_slice(), Some(oversized.as_slice())),
    ] {
        let result = client::transfer_adaptive_batched(target, base, [1; 32], false, |_| {
            panic!("oversized input reached exchange")
        });
        assert!(matches!(result, Err(Error::Limit)));
    }
}

#[test]
fn repeat_probe_with_evicted_remote_object_completes_the_original_raw_offer() {
    let temp = Temp::new();
    let mut endpoint = temp.endpoint();
    let target = vec![17; 65537];
    let mut bytes = 0;
    let mut begins = 0;
    let mut calls = 0;
    let result =
        client::transfer_adaptive_batched(&target, Some(&target), [2; 32], true, |frames| {
            calls += 1;
            for frame in frames.iter().filter(|frame| frame.kind == 1) {
                begins += 1;
                let offer = Offer::decode(&frame.body)?;
                assert_eq!(offer.mode, Mode::Raw);
                assert_eq!(offer.base, None);
            }
            let replies = process(&mut endpoint, frames)?;
            bytes += body_bytes(frames, &replies);
            Ok(replies)
        })
        .unwrap();
    assert!(result.selection.exact_repeat_probe);
    assert_eq!(result.transfer.mode, Mode::Raw);
    assert_eq!(result.selection.expected_http_body_bytes, bytes);
    assert_eq!(calls, 2);
    assert_eq!(begins, 1);
    drop(endpoint);
    let mut reopened = temp.endpoint();
    assert_eq!(
        reopened
            .read_object(&context(), object_id(&target))
            .unwrap(),
        target
    );
}

#[test]
fn missing_base_and_changed_scope_do_not_reoffer_or_publish() {
    let base = random(65537);
    let mut target = base.clone();
    target[33333] ^= 7;
    for seeded in [false, true] {
        let temp = Temp::new();
        let mut endpoint = temp.endpoint();
        if seeded {
            seed(&mut endpoint, &base);
        }
        let mut c = context();
        if seeded {
            c.policy_epoch += 1;
        }
        let mut calls = 0;
        let result = client::transfer_adaptive_batched(&target, Some(&base), [2; 32], false, |f| {
            calls += 1;
            assert!(matches!(
                Offer::decode(&f[0].body)?.mode,
                Mode::SparseV1 | Mode::ZstdPrefixV1
            ));
            process_context(&mut endpoint, &c, f)
        });
        assert!(matches!(result, Err(Error::Missing)));
        assert_eq!(calls, 1);
        assert_eq!(
            endpoint.read_object(&c, object_id(&target)),
            Err(Error::Unknown)
        );
    }
}

#[test]
fn revoked_authority_and_transport_failure_never_retry_selection() {
    let temp = Temp::new();
    let policy = Policy(Arc::new(AtomicBool::new(true)));
    let mut endpoint = Endpoint::open_with_policy(
        &temp.0,
        Limits::default(),
        policy.clone(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let base = random(65537);
    seed(&mut endpoint, &base);
    let mut target = base.clone();
    target[1000] ^= 1;
    let mut calls = 0;
    let result = client::transfer_adaptive_batched(&target, Some(&base), [2; 32], true, |frames| {
        calls += 1;
        if calls == 2 {
            policy.0.store(false, Ordering::SeqCst);
        }
        process(&mut endpoint, frames)
    });
    assert!(matches!(result, Err(Error::Unauthorized)));
    assert_eq!(calls, 2);
    policy.0.store(true, Ordering::SeqCst);
    assert_eq!(
        endpoint.read_object(&context(), object_id(&target)),
        Err(Error::Unknown)
    );
    let mut calls = 0;
    let result = client::transfer_adaptive_batched(&target, Some(&base), [3; 32], false, |_| {
        calls += 1;
        Err(Error::Storage)
    });
    assert!(matches!(result, Err(Error::Storage)));
    assert_eq!(calls, 1);
}

#[test]
fn malicious_receipts_are_checked_after_selection() {
    for bad in 0..4 {
        let temp = Temp::new();
        let mut endpoint = temp.endpoint();
        let target = vec![5; 65537];
        let result = client::transfer_adaptive_batched(&target, None, [2; 32], false, |frames| {
            let mut replies = process(&mut endpoint, frames)?;
            match bad {
                0 => replies[0].body[0] = Mode::Raw as u8,
                1 => replies[1].body = 0u64.to_le_bytes().to_vec(),
                2 => replies.last_mut().unwrap().body[0] ^= 1,
                3 => replies[0].transfer[0] ^= 1,
                _ => unreachable!(),
            }
            Ok(replies)
        });
        assert!(matches!(result, Err(Error::Integrity)));
    }
}

#[test]
fn real_http_codec_selection_cost_matches_bodies_and_reopens_exactly() {
    let temp = Temp::new();
    let endpoint = Endpoint::open_with_policy(
        &temp.0,
        Limits::default(),
        wan::BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let server = thread::spawn(move || {
        wan::serve(
            listener,
            endpoint,
            token,
            &BTreeSet::from([[1; 32]]),
            Duration::from_secs(10),
            3,
        )
        .unwrap()
    });
    let mut http = HttpClient::new(&url, token, [1; 32]).unwrap();
    let base = random(131073);
    client::transfer_batched(&base, Mode::Raw, None, [1; 32], false, |f| http.exchange(f)).unwrap();
    let before = http.metrics();
    let mut target = base.clone();
    target[65536..65568].fill(0);
    let result = client::transfer_adaptive_batched(&target, Some(&base), [2; 32], true, |f| {
        http.exchange(f)
    })
    .unwrap();
    let after = http.metrics();
    assert!(matches!(
        result.transfer.mode,
        Mode::SparseV1 | Mode::ZstdPrefixV1
    ));
    assert_eq!(
        result.selection.expected_http_body_bytes,
        after.request_body_bytes + after.response_body_bytes
            - before.request_body_bytes
            - before.response_body_bytes
    );
    assert_eq!(after.http_requests - before.http_requests, 2);
    server.join().unwrap();
    let mut endpoint = temp.endpoint();
    assert_eq!(
        endpoint
            .read_object(&context(), object_id(&target))
            .unwrap(),
        target
    );
}
