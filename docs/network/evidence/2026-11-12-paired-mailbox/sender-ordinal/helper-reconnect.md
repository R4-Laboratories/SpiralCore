# Replacing a collected mailbox helper

The Linux SDK controller recovered the same 257-byte message and destination-signed receipt after both original mailbox helpers had exited. Two real helper executables exchanged the sealed request through local Unix sockets. The fixture supplied the guest streams, guest signing keys and two live process handles standing in for the VMMs. [MVM source](https://github.com/Casey-allard/mvm/commit/e003cf04dd589993ca5372baf1b95dd72cbd8565) · [Counters and artifact identities](helper-reconnect.json)

| Original submission | Sealed A → B | B → A verdict | Elapsed to the fixture's verified V2 |
|---|---:|---:|---:|
| Before helper exit, return deliberately discarded at guest | 5,531 B | 1,144 B | Unobserved |
| Retry before helper exit | 5,531 B | 1,144 B | 18.711 ms |
| Retry after both replacements | 5,531 B | 1,144 B | 12.266 ms |

All three canonical envelopes and raw verdicts compared equal. The guest submission was 325 B per turn. The reader reopened the original delivery, checked its signed lease against the original message and ciphertext and read all 257 bytes exactly. Its exchange used 348 B toward the helper and 5,695 B back. Its signed ACK exchange used 2,217 B toward the helper and 2,221 B back. These counts cover the named exchanges. Total traffic for both port links was unmeasured.

The interval from the first connection through the reader ACK was 73.251 ms, including the two helper replacements and fixture checks. GNU time measured 0.32 s elapsed, 0.05 s user CPU, 0.01 s system CPU and 46,216 KiB maximum RSS over the entire test. After their respective original lifetime handles exited, the new helpers retired without assistance in 207.587 ms and 10.385 ms.

## Ownership and retained failures

The controller holds the actual original children, checks both VMM identities, signed grants, revocations and original reader enrollments, and preserves the exclusive owner marker. The test refused replacement while A was alive and again when its grant was revoked. B's first replacement failed on a deliberately damaged journal anchor. The controller kept its child handle and original ownership through the failed readiness check. After restoring the fixture's retained anchor, the next replacement reopened B's original journal. A new controller cannot reconstruct this custody from a PID file.

An earlier Linux run refused the group-writable Cargo install directory. A second run used a private installation, observed two pre-replacement relay turns, then hit a broken pipe on a queued predecessor dial. The final fixture pins each unserved connection to its still-live original child, collects that child, accepts at most the original CONNECT prefix through EOF, and checks that no further connection was skipped. On macOS the earlier fixture separately recorded an unobserved turn, an inherited nonblocking socket and a dead-peer identity query refused with ENOTCONN. The kernel no longer exposes that peer PID after exit. [Retained failure and control hashes](helper-reconnect.json)

Linux can remove a stale service socket only after matching its retained inode and observing connection refusal. On macOS the SDK requires an already-vacant pathname. A live-guest helper-replacement trial and the sustained ordinal gate have separate acceptance criteria. [Issue #195](https://github.com/R4-Laboratories/SpiralCore/issues/195)

## macOS arm64 with a vacant service pathname

The Darwin fixture verified each original peer PID and armed its read timeout while that peer was live. It then collected the original child and checked the retained stream through EOF. An earlier run had reached the first verified receipt but returned EINVAL when setting the timeout after the peer exited. The final test confirmed both occupied service paths refused recovery, then explicitly removed its own fixture paths after collection to exercise the vacant-path branch. [Fixture correction](https://github.com/Casey-allard/mvm/commit/01af074b50a15e43cf86088e8de52757cb273d91)

Three turns carried the same 5,518 B envelope and 1,129 B signed verdict. The fixture independently verified the post-replacement receipt in 235.718 ms and read all 257 B from B. The reader exchange used 340/5,665 B and the signed ACK exchange used 2,204/2,208 B, in guest-to-helper/helper-to-guest order. The first connection through ACK took 1.296 s. The whole fixture measured 4.20 s elapsed, 0.17 s user CPU and 0.14 s system CPU. Both successors exited without assistance. The JSON pins the merged macOS source, its helper executable and the separately compiled libtest. Each platform supplied its own fixture credentials and was measured once.

## Reproduction

On the pinned MVM source, build the helper first, then execute the ignored Linux component. Its outer test installs that binary in a fresh private directory and runs one inner fixture. The two libtest summaries describe the same case.

```sh
cargo build --locked -p mvm-hostd --bin mvm-mailbox
cargo test --locked -p mvm-hostd --lib original_helper_reconnect_preserves_receipt_under_both_live_vms -- --ignored --nocapture
cargo test --locked -p mvm-hostd --lib mailbox_process::original_journal::
cargo test --locked -p mvm-hostd --lib helper_restart_reuses_exact_signed_receive_and_enrollment
cargo test --locked -p mvm-hostd --lib dropping_the_launch_ends_an_idle_dialer_and_one_stuck_on_a_silent_guest
```

The original-journal group passed 23 tests with three opt-in cases skipped. The signed reader and the isolated idle/silent-guest teardown each passed separately. The upstream full-suite stall remains outside this isolated execution. [Ari's upstream validation record](https://github.com/tinylabscom/mvm/pull/5387#issuecomment-6834451721)

The exact Linux workspace and all its targets compiled. All conformance targets compiled with the BDD feature enabled. These commands did not execute scenarios. The frozen source verification matched 4,539 file contents and modes.

```sh
cargo check --locked --workspace --all-targets
cargo check --locked -p mvm-conformance --all-targets --features bdd
```
