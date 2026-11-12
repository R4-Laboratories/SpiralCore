# Concurrent SDK name updates and original guest clock admission

Two installed Firecracker attempts reached guest startup and admitted the source. On each attempt the destination SDK returned `machine not found` while applying its post-boot TTL. The persistent launch registers each name before boot and applies its TTL after boot. Those operations previously read and replaced the shared registry without coordinating with concurrent writers. [Installed attempts](paced-runs.md)

## SDK writer regression

On Linux the first regression registered a VM through the public SDK while another writer held the canonical registry lock with an uncommitted row. Registration returned immediately. The repaired SDK uses the existing stable sibling file lock around the read, mutation and atomic save. The regression exercised registration, expiry, readiness and stop individually against a held writer. It verified the old registration, both new registrations, the exact expiry, readiness and removal of the intended name. The complete detached-launch unit suite passed 13/13 in 0.41 seconds. It includes an explicit refusal when setting TTL on an unknown machine. [Repaired MVM source](https://github.com/Casey-allard/mvm/commit/3e10d998e3692ebb94d56d00edd930b51ec71350)

The locks cover participating SDK writers. Registration and readiness keep their existing best-effort behavior on I/O failure. The supervisor’s separate periodic reaper can still publish an old snapshot when it removes or sleeps a VM. Cleanup after a post-boot launch error and simultaneous supervisor reaping require separate qualification.

## Original signed clock replay

The earlier unpaced guests each received a whole-second host clock seed. Their signed admission windows opened 252 ms later. An offline Linux probe ran the real source and destination `open_original` checks against each role’s retained verb grant, original boot signer, session, nonce, verified root and mount commitment. Only the time input varied.

| Time after original seed | Source admitted | Destination admitted |
|---:|:---:|:---:|
| 0 ms | No | No |
| 251 ms | No | No |
| 252 ms | Yes | Yes |
| 1,000 ms | Yes | Yes |
| 181,000 ms | No | No |

The optional Linux witness control `SPIRALCORE_PACE_ORIGINAL_CAST_SEED=1` delays host launch until a whole-second seed can represent the time observed after signing and rechecks the original deadline. One Linux unit test passed for fractional seconds, integral seconds and overflow. The original guest’s precise validation instant was unavailable. The replay identifies a reproducible boundary but cannot assign exclusive causality to the earlier boot refusal.

## Reproduction

On the MVM source commit above, run `cargo test --locked -p mvm-client --lib launch::detached::tests:: -- --test-threads=1`. On its test-only parent, use `overlapping_sdk_name_updates_keep_both_originals_and_existing_rows` as the selector to reproduce the original failure. The pacing test uses the repository’s addressed witness manifest and the selector `whole_second_guest_seed_must_reach_original_issue_time` with the local MVM source paths populated as described in the witness setup. Per-run signatures and boot inputs remain in the originating operator’s custody. These components emit no transfer measurements. [Recorded measurements and artifact hashes](sdk-name-and-clock.json)
