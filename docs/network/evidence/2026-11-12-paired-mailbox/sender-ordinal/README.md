# Original sender ordinal and receipt settlement

The opt-in `OrdinalV1` profile gave the original Linux sender a bounded retry window. In one component check B opened sequences 1 through 65 and recovered each three-byte payload exactly. The source reopened the identical 5,159-byte frame for ordinal 1 after losing its in-memory state. Separate original Linux helper processes exercised the normal durable-journal transport with a 257-byte input and fixture guest sockets. [Reviewed contributor source](https://github.com/Casey-allard/mvm/commit/121ff94ea1a22d5d5156230318aa139264ccea3a) · [Scale gate](https://github.com/R4-Laboratories/SpiralCore/issues/195)

| Linux check | Observation |
| --- | --- |
| Sender capacity | 64 unresolved ordinals filled the window. Ordinal 65 returned `QueueFull(64)` and 66 returned `SenderOrdinalGap`. Ordinal 1 reopened with its original credential and ciphertext. Changed content was refused. |
| Contiguous settlement | The original A guest key signed each stored B receipt digest. Settling ordinals 64 through 2 left the prefix at zero and 65 still refused. Settling 1 advanced the prefix to 64. A repeated signed settlement returned 64 after reopening. Resubmission of retired 1 refused. The next accepted sequence was 65. |
| Original-generation loss | A partially written replacement snapshot was ignored. A different boot-direction commitment refused. Removing the state directory while retaining its separate generation marker prevented reinitialization. |
| Two helper processes | The fixture closed A’s first submission stream before receiving its response. Replaying the identical submission returned the signed B V2. B recovered the exact 257 bytes and signed its delivery ACK. After a lost settlement response, the original guest’s signed settlement replay returned prefix 1. A fresh submission for 1 was refused. |

One source-to-destination relay turn wrote 5,517 bytes and read 1,136 bytes. B's fixture guest sent 349 bytes and received 5,630 on the read. Its subsequent ACK sent 2,204 and received 2,208. These are completed userspace socket counts at their listed boundaries. A's guest-local socket byte totals and per-endpoint transfer-only CPU were unmeasured. The two-helper fixture and its children took 0.28 s wall, 0.02 s user and 0.01 s system, with a maximum RSS of 44,668 KiB as reported by GNU time. Original helper collection took 222.733 ms for A and 10.306 ms for B. Both exited successfully without assistance. [Machine-readable results](results.json)

## What persists

A signed outbound journal direction selects the version and commits the original plans, boot and credentials. Its derived message ID binds the direction and positive ordinal. The source rechecks the current three capabilities before disclosing a receipt or accepting settlement. Each unresolved entry stores a digest of the requested recipient, message ID and body, its original sealed frame, the bounded attempt count and any verified B receipt. The original A guest signer authorizes retirement by signing the digest of that receipt. A receiving guest ACK continues to settle B’s own lease. Existing arbitrary-ID senders retain their original behavior. [Sender protocol](https://github.com/Casey-allard/mvm/blob/121ff94ea1a22d5d5156230318aa139264ccea3a/crates/mvm-core/src/mailbox/sender_ordinal.rs) · [Original host dispatch](https://github.com/Casey-allard/mvm/blob/121ff94ea1a22d5d5156230318aa139264ccea3a/crates/mvm-hostd/src/mailbox_service/directed_journal.rs)

One private snapshot atomically commits the contiguous retired prefix and all later live ordinals. Their count and the prefix jointly determine the issued high-water. The limit is 64 entries, eight attempts per entry, 8,192 bytes of application payload subject to the signed grant, and 65,536 bytes per sealed frame. The encoded snapshot limit is 8,913,920 bytes. Publication can additionally occupy one replacement snapshot of the same maximum size, plus fixed lock and original-generation metadata. These are configured file-content bounds. This test did not measure filesystem allocation or maximum resident storage at saturation. Recovery requires the original protected state directory and generation. [Snapshot implementation](https://github.com/Casey-allard/mvm/blob/121ff94ea1a22d5d5156230318aa139264ccea3a/crates/mvm-hostd/src/mailbox_service/sender_window.rs)

## Reproduction and remaining qualification

At the pinned MVM source on x86-64 Linux, use Rust 1.97.1 with two Cargo jobs, disabled incremental compilation and debug symbols disabled for the test and dev profiles.

```sh
cargo build --locked --offline -p mvm-hostd --bin mvm-mailbox
cargo test --locked --offline -p mvm-hostd --lib mailbox_service:: -- --nocapture --test-threads=1
cargo test --locked --offline -p mvm-hostd --lib original_helpers_settle_ordinal_only_on_signed_receipt -- --ignored --nocapture
```

The first selector passed 26 tests, including the legacy FIFO and directed arbitrary-ID capacity checks. The separate executable selector passed once and created two genuine Linux mailbox helpers. Its private guest streams and observed lifetime processes were test fixtures. Reopening A’s store in the capacity test measured persistent-state recovery within the same test process. This component preceded the native 65- and 2,049-transfer trials. Restart of both helpers on the same still-authorized guests and aggregate capacity remain open. [Acceptance criteria](https://github.com/R4-Laboratories/SpiralCore/issues/195)

## Guest sender and original boot admission

The original guest application can now stage one payload on its existing reader socket, request its status and explicitly settle the digest of B’s signed receipt. Staging acknowledges a local write. Status asks the original host to reauthorize the operation before disclosing a receipt. The host journal can retain 64 unsettled ordinals, while this version of the guest API admits one active application payload. [Guest API](https://github.com/Casey-allard/mvm/blob/4dcf544bed90bfc2cb90c065998a025b859ddab4/public/src/content/docs/reference/guest-agent.md)

| macOS component check | Observed behavior |
| --- | --- |
| Original identity before activation | The initial regression test failed because a missing identity returned success on an ordinal boot. With the fix, a required but missing identity, a malformed identity and an orphaned key refuse startup. An ordinary sender with neither file keeps its Send-only path. |
| Application and host reader frames | Eight guest reader tests passed, including an ordinal request on an ordinary boot. The separate application wire test round-tripped the allowed requests and refused an application-supplied settlement statement. |
| Original guest outbox | Three store tests at `295792422` reopened one durable operation with the original in-memory key, bound settlement to the verified receipt, and refused a different key, missing active data or a damaged authenticated cursor. |

The host projects its admitted sender choice as `mvm.mailbox.sender=ordinal-v1`. PID 1 checks the boot record against this token and requires the matching ACK key before starting an ordinal workload. The outbox encryption key lives in the original guest process. Recovery demonstrated by reopening those files applies while that key remains available. [Host projection](https://github.com/Casey-allard/mvm/blob/4dcf544bed90bfc2cb90c065998a025b859ddab4/crates/mvm-vmm/src/host/egress_bridge.rs) · [Original guest store](https://github.com/Casey-allard/mvm/blob/295792422ebc9ba537a084b3143f464c667f7039/crates/mvm-agentd/src/mailbox_outbox/ordinal.rs)

The joined contributor revision is `4dcf544bed90bfc2cb90c065998a025b859ddab4`, Git tree `1565d3bdd9bdbf095f213fad3e5777efbae9199f`. It incorporates upstream MVM `b359f25f`. Use the following focused checks from that checkout.

```sh
cargo test --locked -p mvm-agentd --bin mvm-guest-agent mailbox::tests::
cargo test --locked -p mvm-agentd --bin mvm-guest-agent mailbox_reader::
cargo test --locked -p mvm-core --lib application_sender_requests
cargo test --locked -p mvm-vmm --lib workload_cmdline_asserts_a_mailbox_only_for_a_granted_plan
cargo test --locked -p mvm-agentd --lib mailbox_outbox::ordinal::tests::
```

These checks measure local startup, persistence and authorization. Application bytes, sealed bytes, guest vsock bytes, per-endpoint CPU and elapsed transfer time for the installed ordinal path require the two-guest run. [Installed acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/195)

## Two original Firecracker guests, 65 distinct ordinals

A single admitted pair on x86-64 Linux delivered 65 different 257-byte bodies, 16,705 bytes in total. Each used a 313-byte application frame. B read its synced file back and ACKed the lease. A recovered exactly the signed B V2 receipt observed on the original relay and settled its digest. The retired prefix reached 65. Both SDK starts used one vCPU and 256 MiB guests with no guest NIC. The signed source boot selected `ordinal_v1`, a one-hour capability window and 313-byte frames. [Full measurements and binary identities](native-65.json) · [Witness invocation](../../../../../tools/ari-own-mailbox-witness/paired-native.md)

| Observed boundary | A | B |
| --- | ---: | ---: |
| Workload CPU for all operations, ms | 25.803 | 39.082 |
| Application → original guest reader, bytes | 85,061 | 138,749 |
| Original guest reader → application, bytes | 122,014 | 318,089 |
| SDK workload stdin / stdout, bytes | 28,573 / 43,026 | 10,187 / 58,611 |
| Original host port 5257 read / accepted write, bytes | 85,628 / 108,178 | 195,615 / 771,394 |
| Port 5257 returned exchanges / read errors / write errors | 131 / 0 / 1 | 196 / 0 / 1 |

The original relay handled 379,499 sealed bytes toward B and 78,825 signed reply bytes toward A. Both full-session port-5257 observers closed. Each recorded one write error and the byte counts reflect positive system-call returns. These aggregate files do not identify the operation that saw the error. The SDK stdin/stdout counts exclude outer RPC framing. The application, relay and guest socket counts describe different layers.

Setup took 6.163 s. The series completed in 98.492 s. Measured from the start of each ordinal, the median / p95 times were 243.80 / 273.98 ms to the host-verified B receipt, 1.409 / 1.507 s through the destination readback, and 1.487 / 1.595 s through A's settlement. Each ordinal included an authenticated empty Receive at B after local staging and before forwarding. All 65 controls were sequential and their median elapsed time was 76.14 ms. The driver recorded no pending-status retries. The driver used 8.277 s of process CPU. GNU time recorded 104.98 s wall, 16.12 s user CPU, 0.88 s system CPU and 82,440 KiB maximum RSS for its measured process tree. Guest workloads used release builds and the host SDK driver and helpers used debug builds.

Both original helpers exited with code 0 on their first unassisted stop. The audit chain verified 17 entries, two sessions and two seals. No Firecracker process, mailbox helper or active MVM scope remained. The test kept one outstanding operation at a time. Live both-helper restart, larger sustained counts, multi-tenant contention and WAN operation have separate acceptance gates. [Original host and guest source](https://github.com/Casey-allard/mvm/commit/4dcf544bed90bfc2cb90c065998a025b859ddab4) · [Sender retention tracker](https://github.com/R4-Laboratories/SpiralCore/issues/195)

The later sustained trial completed 2,049 distinct transfers on one original pair, with the same one-hour grant. [Sustained measurements and earlier failure](native-2049-checkpoint.md)
