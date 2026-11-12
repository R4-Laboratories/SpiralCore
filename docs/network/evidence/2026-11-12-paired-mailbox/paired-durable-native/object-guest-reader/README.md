# Original guest object reader and destination settlement

The Linux guest-agent component carried the exact four application bytes `61 00 62 00` across its host-facing Unix stream. The original staged request survived a lost response. Recreating the guest reader on the same boot and mount required it to recover the original signed V2 receipt again before accepting a different pending operation. The measured test exercised the real guest request handler against a synthetic host peer. A separate test exercised the real destination journal, including close and reopen. [MVM source](https://github.com/Casey-allard/mvm/commit/b04ae4f084ee741bfc58ea51ce7ec672cf9a29cc) · [Recorded inputs and results](linux.json)

## Authority, persistence and ACK

The signed plan selects one pair of credentials. The guest captures its original reader binding and ACK key before dropping privileges. After a host-CID connection on the original mailbox port supplies the signed role, the agent verifies its local boot, both original endpoints, both credentials and the validity window before exposing the application reader. A selected guest never falls through to ordinary Cast outboxes. The unselected guest keeps its original wire representation and mailbox behavior. [Pair enrollment](../object-pair-enrollment.md) · [Guest listener](https://github.com/Casey-allard/mvm/blob/b04ae4f084ee741bfc58ea51ce7ec672cf9a29cc/crates/mvm-agentd/src/bin/mvm-guest-agent/mailbox.rs)

1. **Receive** opens a retained envelope under its original grant and current Receive authority. For an object operation the signed lease also commits to the exact original header, body length, body bytes and selected pair. This allows the workload to recover the authenticated Call’s return route, IDs and epochs. The separate journal test recovered the identical signed lease and payload from a fresh journal handle.
2. **Stage** publishes the complete stable Call or Reply in one boot-scoped guest slot. The agent syncs the new file and its directory. Retrying the same request settles an uncertain publication. A different request cannot displace a pending original. This persistence depends on the same guest mount. New-VM recovery was outside these tests.
3. **Status** drives the exact pending operation on the existing selected reader. The original host must independently recover its immutable sealed request and provide its expected delivery ID, ciphertext commitment and original intent digest. The guest compares those identifiers with the destination-signed V2 and checks its key, address, message ID, epoch and signed validity. A correctly signed receipt for a different seal failed the test. The guest cannot independently derive ciphertext from its plaintext in this trusted-host profile.
4. **ACK** remains a separate application request. B’s guest checks settlement of the Reply tied to its current Call lease before authorizing that Call’s ACK. The SpiralCore reader passes a validated, bounded object frame to the shared endpoint or exposes it to the original caller. The application explicitly issues the ACK after handling the frame. [Guest submit and settlement](https://github.com/Casey-allard/mvm/blob/b04ae4f084ee741bfc58ea51ce7ec672cf9a29cc/crates/mvm-agentd/src/bin/mvm-guest-agent/mailbox_reader/object_submit.rs)

## Completed socket I/O

The counters are completed reads and writes at the guest side of the host-facing AF_UNIX socket pair. Each timer starts just before spawning the exchange and ends when the guest application handler returns. These intervals include encoding, verification, scheduling and response handling. They exclude earlier role and staged-slot setup. The fixture supplies the host verdict and the four-byte Call body is already staged. No data crossed a guest boundary, NIC or WAN.

| Original operation | Guest → fixture host | Fixture host → guest | Interval |
|---|---:|---:|---:|
| Lost verdict, original remains pending | 459 B | 0 B | 0.380213 ms |
| Correctly signed V2 for the wrong seal, refused | 459 B | 1,617 B | 0.470606 ms |
| Bare application V2 presented by the host, refused | 459 B | 1,001 B | 0.416983 ms |
| Wrong destination signer, refused | 459 B | 1,617 B | 0.637163 ms |
| Matched original seal and signed V2 | 459 B | 1,617 B | 0.661018 ms |
| Fresh reader recovered same signed V2 | 459 B | 1,617 B | 0.672199 ms |

The complete one-test process peaked at 9,528 KiB RSS. An independent run of the same executable across all six exchanges used 0.009 s user CPU and 0.001 s system CPU in 0.010 s wall time. This includes fixture, cryptography, filesystem, both endpoints and test harness. Send-only time, per-endpoint CPU, actual journal commit latency and compression gain require a different measurement boundary. [Executable, distinct trial rows and log digests](linux.json)

## Reproduce on Linux

Check out the pinned MVM source as `source/` beside the SpiralCore clone `spiralcore/`. Copy SpiralCore’s `tools/ari-own-mailbox-witness/paired-guest/` into the sibling directory `paired-guest/` following the existing [workload build layout](../../../../../../tools/ari-own-mailbox-witness/paired-native.md). Use Rust 1.97.1 and the retained Cargo locks.

```sh
cd source
cargo test --locked -p mvm-core --lib mailbox::object_pair
cargo test --locked -p mvm-agentd --lib mailbox_outbox::object_pair::tests
cargo test --locked -p mvm-agentd --bin mvm-guest-agent mailbox_reader:: -- --nocapture --test-threads=1
cargo check --locked -p mvm-hostd
cd ../paired-guest
cargo test --locked --lib --features object-reader-v1
```

The selectors passed 3 journal/authority tests, 1 exact staging test and 7 guest-reader tests. The SpiralCore application feature compiled and its two existing controls passed. The ordinary host service compiled and continues to refuse the selected operations until an enrolled host handler supplies them. The selected services, cold and exact-repeat application transfer, guest boundary counters and lifecycle measurements still require an installed two-MVM run. [Acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)

At this revision the existing journal reopen path requests Send for every retained row. A’s Reply-only credential therefore still needs a scoped journal lease and ACK path tied to the original pending Call. The B Call reopen and guest-side V2 control above do not exercise that reverse journal route. A [later Linux journal test](reverse-journal.md) qualifies its scoped Reply lease and signed ACK.
