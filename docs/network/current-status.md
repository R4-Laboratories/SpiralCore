# Networking delivery status

The Linux registered host-process FlowMux endpoint now completes exact transfer and original-owner retirement, including the archive retry, on [MVM `31f8a562`](https://github.com/Casey-allard/mvm/commit/31f8a562f19d9eac89cb0d078b507770bb592262). The run verified 5,804 bytes in each direction and counted 6,891 TLS bytes across the cooperating-peer connection. Authorized Open through verified HalfClose took 34.724 ms; retirement took 43.624 ms. The complete test used 0.13 CPU seconds. Linux hostd/VMM libraries pass 2,492/1,112 tests, with 19 existing ignores; original-lease controls pass on Linux and macOS, and all-targets workspace Clippy passes on both. These are host-process results; booted-guest and WAN acceptance on this source remain open. [Measurements, retained failures and qualification limits](evidence/2026-11-13-owned-unix-store/linux-installed-retirement.md).

The shared Rust core now has an opt-in descriptor-rooted SmallV1 store for Linux and macOS. Native primitive checks passed 17/17 on Apple Silicon and 18/18 on Linux, including the held-descriptor mount-ID control. The MVM host adapter consumes the new API, and both original macOS relay-custody fixtures now pass. The successor hostd library suite passed 2,472 tests with 15 existing ignores. The joined canonical-host library passes 170 tests, including all 25 parent/configuration tests, and the socket-backed adapter passes 11; original-parent descriptor/workspace funding is included in those host-process results against the public MVM SDK; installed-native qualification remains open, and this is not an installed-service or WAN qualification. SpiralCore PR 259 remains unmerged because the Windows Server 2025 sustained-stream gate failed its existing deadline. [Integration results and retained failures](evidence/2026-11-13-owned-unix-store/mvm-integration.md) · [Storage contract and resource boundary](owned-unix-store.md) · [Remaining acceptance](https://github.com/R4-Laboratories/SpiralCore/issues/258).

Updated **2026-11-13**. Native Linux x86-64/Firecracker and Apple Silicon/HVF now have clean and lost-first-receipt results on the same corrected MVM host source. Across four single-host trials, B verified, reopened and acknowledged every original byte. In the Linux clean run the relay carried 5,808 B toward B and 1,188 B back, with verified receipt in 105.627 ms and confirmed readback in 0.440133 s. The Linux retry recovered the identical signed receipt in 161.334 ms and completed application readback and the empty probe in 1.361047 s. All original VMMs stopped and all original mailbox helpers exited unassisted. [Linux measurements](evidence/2026-11-12-paired-mailbox/paired-durable-native/linux-823.md) · [Apple measurements](evidence/2026-11-12-paired-mailbox/paired-durable-native/arm64-hvf.md) · [Earlier Linux clean run](evidence/2026-11-12-paired-mailbox/paired-durable-native/graceful.md) · [WAN delivery](https://github.com/R4-Laboratories/SpiralCore/issues/47) · [Platform matrix](https://github.com/R4-Laboratories/SpiralCore/issues/46).

On Linux, two original Firecracker guests recovered ordinal 1 after the controller replaced both mailbox helpers. A recovered the original signed receipt. B read and acknowledged 257 exact bytes, and A settled the ordinal in 1.271 s from staging. Both replacement helpers stopped cleanly. The interrupted helpers yielded partial port-5257 measurements. [Original row, four transcripts and timings](evidence/2026-11-12-paired-mailbox/sender-ordinal/native-helper-restart/README.md).

The joined MVM contributor source passed its locked, offline macOS arm64 workspace test suite and strict workspace Clippy. A preceding workspace run exposed a stale expected cleanup count in a mocked cloud-controller test. The Linux native helper-replacement trial used the recorded parent revision. [Exact revisions, failed and passing commands, and original log hashes](evidence/2026-11-12-paired-mailbox/sender-ordinal/joined-macos-workspace.md).

On Apple Silicon macOS, two actual mailbox helper processes now recover their original receipt and exact 257-byte delivery after both helpers are replaced, without fixture deletion of their service sockets. The original directory watch fences changed paths before interruption and stale-socket removal. This is a host-process component witness with fixture guest streams; live-guest recovery on Apple Silicon remains open. [Source, bytes, timing and scope](evidence/2026-11-12-paired-mailbox/sender-ordinal/darwin-helper-reconnect.md).

After restoring the launch-volume lock to the mailbox SDK, two Firecracker guests admitted under observed registry contention, returned 4,096 exact bytes cold and warm, and both helpers retired on their first stop. [Contended launch, bytes, CPU and elapsed time](evidence/2026-11-12-paired-mailbox/paired-durable-native/volume-contention/README.md). Four fresh pairs on the host revision incorporating MVM #5384 returned all eight cold and repeat objects exactly. Under both raw and whole-object zstd, the identical repeat used one Call, 190 application bytes and 14,226–14,317 sealed bytes. [Per-run CPU, vsock bytes, latency and shutdown](evidence/2026-11-12-paired-mailbox/paired-durable-native/volume-contention/upstream-5384/README.md).

Two matched Firecracker comparisons retained the fixed 5 ms object-reader cadence. Adaptive empty polling saved 7,537–16,027 guest-stream bytes on the warm 4 KiB operation, added 0.55–0.60 s, and gave opposite cold byte results across run orders. [Measurements and the independent helper exit failure](evidence/2026-11-12-paired-mailbox/paired-durable-native/native-object/empty-pacing.md).

The ARM64 object-mailbox guest images have been sealed and reopened on Linux. Both executables and their OCI entrypoints matched the originals. The same guest archive produced matching runtime pieces in a second home. [Images, digests and Apple execution gate](evidence/2026-11-12-paired-mailbox/paired-durable-native/arm64-object-images.md).

Three 4 KiB operations on a fresh Linux Firecracker pair delivered all 12,288 bytes exactly, with three admitted control probes completed between staging and relay forwarding. The median through verified destination readback was 1.192 s. The same driver completed three original 257-byte operations on another fresh pair. The larger profile used 37,972 bytes toward B and 3,608 bytes back across the host relay. [Bytes, CPU, timing and original helper counters](evidence/2026-11-12-paired-mailbox/paired-durable-native/size-control/README.md).

On a separate Linux pair, A retried a 4 KiB object after the relay withheld B’s first signed receipt. B returned the identical receipt after 173.946 ms from A’s first Call. Verified exact readback, ACK and an empty-queue check completed in 774.847 ms. Both attempts sent the full 12,685-byte sealed request. [Per-attempt bytes, guest CPU and setup costs](evidence/2026-11-12-paired-mailbox/paired-durable-native/4k-receipt-loss/README.md).

The shared Raw sender now splits a 4,096-byte literal into two Data frames under a signed 4,152-byte Call cap. In the separately authorized local Call/Reply mapping, the exact repeat used no Data payload and 7,502 encoded envelope bytes. The installed Firecracker trial sent the complete 4 KiB fixture. [Component costs and return-authority gate](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-reply-boundary.md#raw-data-under-a-signed-frame-cap).

The shared sender now validates and settles one application Reply before issuing the next Call. A component run on both Linux and macOS sent 4,658 encoded bytes cold and 190 bytes for the identical 4 KiB repeat, including both directions. [CPU, wall time, ACK failures and measurement boundary](evidence/2026-11-12-paired-mailbox/paired-durable-native/leased-object-sender.md).

The Linux `mvm-mailbox` worker can handle a selected Object-v1 Call or Reply over the original guest agent and two host sockets. A focused check recovered the same sealed Call after a lost response. The executable, 19 component checks and separate guest/host counter definitions are published. [Source, test commands and measurement boundaries](evidence/2026-11-12-paired-mailbox/paired-durable-native/selected-worker.md).

The paired qualification driver supplies the fixture only to A and requires B to return the exact bytes through its own reader on both the cold transfer and the repeat. It checks that the same B worker handled both phases and records application bytes, CPU and elapsed time. [Command and counter definitions](../../tools/ari-own-mailbox-witness/paired-native.md#measuring-an-installed-cold-object-and-its-exact-repeat).

The MVM source owner now stores the exact sealed Call or Reply before forwarding. On Linux, reopening and resending the same five-byte Call to a durable journal returned the identical signed receipt in five socket trials. The median first/retry exchange took 1.098/0.588 ms. [Source, both directions of traffic and CPU](evidence/2026-11-12-paired-mailbox/paired-durable-native/original-object-intent.md).

The source guest worker now stages the object Call on MVM’s local reader, waits for the agent’s verified destination admission, validates B’s application Reply, and ACKs that lease. A live worker can retry the same ambiguous ACK by pair, transfer and sequence. Linux component tests exercised the typed socket operations. The destination worker now handles the original Call in the shared receiver. An isolated Linux process test lost the first host result, retried the exact computed Reply and preserved the original Call without ACK when the fixture refused admission. [Receiver authority, costs and limits](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-guest-reader/destination-worker.md). [Guest worker entry points and reproduction](../../tools/ari-own-mailbox-witness/paired-native.md#application-call-and-reply-on-the-selected-object-pair).

The installed Cast grants carry complete fixtures in one direction. The older authenticated Call/Reply mapping verified an exact 131,090-byte repeat with zero Data payload in a local component. The installed object profile uses a separately admitted application return route. [Grant checks and measured component costs](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-reply-boundary.md). [Signed-plan enrollment at that component revision](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-pair-enrollment.md).

In the selected host provisioning test, A sealed a 5-byte Call and B sealed a 7-byte Reply bound to the active Call lease. Both reopened exactly, and a changed B boot invalidated the earlier V2 acceptance. Linux passed 3 original-pair, 6 custody and 60 core mailbox checks. [Host provisioning and receipt checks](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-pair-host.md).

The Linux SDK can retain the original Firecracker configuration peer and the reader binding written to its guest identity drive. Process and drive tests cover a replacement socket and a refused second identity issue. [Original-owner component results](evidence/2026-11-12-paired-mailbox/paired-durable-native/original-firecracker-handoff.md).

The selected HVF cold-launch path now captures the public reader record written to the original identity drive. Linux component checks distinguished two boots under the same signed plan. Apple Silicon component checks now show two independent observers detecting the same child’s exit and exact nine-byte readback on the canonical port 5257 Unix socket fixture. Native selected-pair guest transfer remains open. [Original worker and bridge checks](evidence/2026-11-12-paired-mailbox/paired-durable-native/hvf-original-worker.md). [HVF reader binding and scope](evidence/2026-11-12-paired-mailbox/paired-durable-native/hvf-original-reader.md). The persistent SDK also retains the original cold-launch owner for each reserved object role through worker startup and verified teardown. [SDK admission, tests and release-runtime prerequisite](evidence/2026-11-12-paired-mailbox/paired-durable-native/hvf-object-sdk.md).

One persistent Linux Firecracker pair verified eight separately addressed operations and eight admitted control Calls. The median time through signed receipt was 220.763 ms, and the median through B’s ACK and exact returned readback was 883.826 ms. Both original helpers exited cleanly. [Operation timing, bytes, CPU and scope](evidence/2026-11-12-paired-mailbox/paired-durable-native/sustained/README.md).

The Linux helper restart test now collects both original port-5257 transcripts across re-entry into the same VM state directory. The original signed receipt survived the restart, and an authenticated reader found the previously acknowledged queue empty. The two sessions recorded 4,818 / 8,722 and 432 / 4,479 bytes read / written at the host. [Host recovery test, exact counters and thread CPU](evidence/2026-11-12-paired-mailbox/recorder-restart/README.md)

Two original Linux Firecracker guests completed a 4,096-byte object transfer and same-worker warm repeat with exact readbacks. A fresh second run closed all six stream records and retired both original workers with exit 0. Cold/warm traffic was 4,658/190 B at Object-v1, 481,989/124,813 B across the two guest-port links and 63,610/14,170 B on the sealed host links. Elapsed through readback was 3.793/1.045 s. Two further fresh installations compared full Raw and fixed whole zstd with identical role images. Cold Raw transferred 4,096 payload bytes; whole zstd transferred 4,110. Both reread all 4,096 original bytes and retired both guests cleanly. Independent work on the host and differing polling counts limit timing comparisons. [Matched installed controls](evidence/2026-11-12-paired-mailbox/paired-durable-native/native-object/matched-codecs.md) A separate native pair withheld the final Published ACK result at A after reading it from the authenticated original reader. The same live worker recovered using one guest-port exchange, with zero sealed peer bytes. [Original-lease reconciliation](evidence/2026-11-12-paired-mailbox/paired-durable-native/native-object/ack-reconciliation.md) The earlier 64-exchange observer overflow remains in the report. [Both native runs and measurement boundaries](evidence/2026-11-12-paired-mailbox/paired-durable-native/native-object/README.md)

The Linux caller and callee workload images for the object pilot are sealed. Their embedded executables were read back byte for byte. Original image and nonce selection passed 4 paired-plan checks and 101 canonical admission checks. [Workload identities and original-plan checks](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-role-images.md)

The selected object Call/Reply reader now checks the original signed lease against the complete header and body. A Linux guest-agent socket test recovered the same signed acceptance after a lost response and a reader restart. Separate Linux journal tests reopened B’s original Call and A’s correlated Reply. A’s signed ACK advanced the file anchor once and replayed unchanged after another reopen. [Reverse route and 61 test results](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-guest-reader/reverse-journal.md). [Exact component results and commands](evidence/2026-11-12-paired-mailbox/paired-durable-native/object-guest-reader/README.md) · [Issue #202](https://github.com/R4-Laboratories/SpiralCore/issues/202)

### Cross-host carrier handoff

Same-host delivery uses each guest’s vsock and original mailbox helper. For different hosts, the proposed MVMD Iroh/Hydro adapter carries the encrypted canonical envelope between the two MVM endpoints. The destination journal still owns the signed acceptance receipt. The adapter and MVMD #254 are private. A versioned test binary with a public invocation contract, explicit direct/relay settings and a documented return path for B’s original signed receipt would allow the two-host test to proceed. A public repository or private signing keys are unnecessary. The WAN trial must record the actual path, each relay hop, exact readback, CPU and elapsed time. [Public ownership discussion](https://github.com/tinylabscom/mvm/issues/4633#issuecomment-6751136680) · [WAN acceptance and handoff](https://github.com/R4-Laboratories/SpiralCore/issues/47)

A Linux regression reproduced the cleanup failure with two real mailbox helpers and two process-lifetime fixtures. After the fix, the source helper exited normally in 207 ms and the destination in 16 ms while the orchestrator and both state directories remained live. [Original-helper test, byte counts and CPU](evidence/2026-11-12-paired-mailbox/persistent-sdk/original-lifetime.md).

A separate two-guest Linux trial withheld B’s first signed receipt. A’s original agent retried the same sealed envelope and B returned a byte-identical receipt. After one confirmed ACK the destination queue was empty. Exact readback and that empty probe took 1.159652 s, and both helpers exited unaided with complete stream meters. [Native retry measurements, controls and reproduction](evidence/2026-11-12-paired-mailbox/paired-durable-native/receipt-loss.md).

The host relay now drains B's buffered signed verdict after a normal Unix socket close under the original transfer deadline. Four connected-socket regressions passed on Apple Silicon macOS and Linux. [Failure, fix and exact test inputs](evidence/2026-11-12-paired-mailbox/paired-durable-native/verdict-deadline.md).

An earlier two-guest test exercised each guest’s own mailbox. A separate addressed attempt recorded a deadline failure and zero bytes on B’s port-5257 stream. [Independent own-mailbox measurements](evidence/2026-11-12-paired-mailbox/firecracker.md) · [Earlier addressed timeout](evidence/2026-11-12-paired-mailbox/addressed-native-launch/paired-deadline-native.md).

The persistent machine SDK can now boot B with its original incoming journal policy and A with the independently authorized source/recipient pair. The orchestrator retains both owners through startup and rollback. Its typed stop waits for original VMM and helper exit before releasing runtime state. Apple Silicon macOS passed 217 launch and 14 original-helper tests. Linux passed 24 focused checks. [API, source and recorded checks](evidence/2026-11-12-paired-mailbox/persistent-sdk/README.md).

The guest worker pool can return its final complete response while retaining the original process for collection. One macOS 257-byte process fixture returned in 60 ms, down from 2,174 ms. [Linux and macOS worker lifecycle tests](evidence/2026-11-12-paired-mailbox/warm-worker/README.md).

A native macOS host test sealed 257 bytes, verified B’s V2 receipt over a Unix socket, reopened the original destination journal and read all 257 bytes back using B’s signed Receive. Its positive exchange used 5,503 bytes toward B and 1,119 bytes in reply. [Destination enrollment, CPU and timing](evidence/2026-11-12-paired-mailbox/durable-guest-binding/incoming-journal.md).

Two original Linux mailbox helper processes exchanged a B-addressed message on Unix sockets. After a lost reply, both retries returned B’s original signed V2 receipt. A fixture reader recovered all 257 bytes and acknowledged them. Each relay turn used 5,539 bytes toward B and 1,145 bytes back. [Two-helper measurements and installed scope](evidence/2026-11-12-paired-mailbox/two-helper-journal/README.md).

Linux host tests now cover original mailbox helper ownership across socket replacement and launcher failure, and exact source ciphertext retries with an independent destination credential. [Source, regression tests and remaining installed gate](evidence/2026-11-12-paired-mailbox/durable-guest-binding/original-journal-host.md).

A guest-side candidate now keeps an explicitly B-addressed pending operation across a lost reply, fences older boot or agent versions, and removes the original bytes after verifying a matching B receipt. Six directed and nine original mailbox component tests passed on macOS. [Original scope, wire costs and installed gates](evidence/2026-11-12-paired-mailbox/durable-guest-binding/directed-source.md).

The [single-guest warm workload](../../crates/spiralcore-mvm-guest/README.md#single-guest-warm-workload) now builds as a static Linux x86-64 executable. Its component tests cover exact file readback before ACK and counts for the local reader socket.

A single Linux/Firecracker guest now recovers its own 313-byte SpiralCore frame under Ari's signed production mailbox, verifies the 257-byte body including its trailing zero, and ACKs after file readback. Its journal reached cursor 3. Three host-side vsock exchanges recorded 2,971 bytes read and 9,272 bytes accepted for writing. The SDK Call took 2.005 s. That single-guest test exercised its own signed mailbox. [Native exact readback, CPU, counts and reproduction](evidence/2026-11-12-paired-mailbox/own-mailbox-native/verified-own-mailbox.md).

In an earlier attempt, a Linux/Firecracker guest submitted a 313-byte SpiralCore frame to its own production mailbox. The host retained one journal entry and an anchor at cursor 1. The guest reader returned an authorization refusal before exact readback or ACK. [Three native attempts, resource use and remaining reader gate](evidence/2026-11-12-paired-mailbox/own-mailbox-native/README.md).

In a follow-up Firecracker guest, the original reader's wall clock was 677 ms before the signed enrollment at the start of Receive and 606 ms before it on refusal. The original verifier returned `NotYetValid` at both samples and accepted at issue time. Two later guests stopped during boot. [Reader measurements, original journal cursors and recovery criterion](evidence/2026-11-12-paired-mailbox/own-mailbox-native/reader-clock.md).

Ari’s [draft trusted-host journal](https://github.com/tinylabscom/mvm/pull/5377) adds a local-durable outbox, boot-bound guest reader and application ACK. A native Linux guest-application component now stages a SpiralCore frame and checks its readback and explicit ACK on a real local Unix stream with a synthetic reader. [257-byte fixture, on-socket counts and CPU](evidence/2026-11-12-paired-mailbox/durable-guest-binding/README.md). For the trusted-host local-durable tier, the destination journal returns a signed V2 receipt. The separately witnessed durable tier retains its V1 receipt. The subsequent paired runs above exercised B’s own admitted reader on Firecracker and HVF. [Paired-route integration points](evidence/2026-11-12-paired-mailbox/addressed-native-launch/ari-journal-binding.md).

Ari's journal draft already contains the verified admission and Reply authorization APIs from the contributor draft. Reconciling the pinned branches produced an identical source tree to Ari's draft. Linux passed 113 scoped checks, including the signed header 7 versus verifier 8 regression. [Source identity, receipt authority and compatibility tests](evidence/2026-11-12-paired-mailbox/addressed-native-launch/journal-reconciliation.md).

A signed, B-addressed SpiralCore frame reached the canonical local-durable journal over a real Unix socket. After the service restarted, it returned the original signed V2 receipt for the same sealed bytes. A separately authorized host reader recovered the 313-byte frame and acknowledged it. The subsequent two-guest runs above exercised that path through guest B. [Host journal exchange, counters and timings](evidence/2026-11-12-paired-mailbox/durable-host-binding/README.md).

The repaired SDK serialized concurrent VM name registration, TTL, readiness and stop updates in 13 passing Linux detached-launch tests. Replaying the two original guest grants independently located a 252 ms clock admission boundary. [SDK regression, original clock replay and limits](evidence/2026-11-12-paired-mailbox/addressed-native-launch/sdk-name-and-clock.md).

Linux process tests for installed Firecracker rollback confirmed exit of the original API peer after its socket path was replaced. The replacement remained live. The earlier four native VMMs required administrative scope retirement. SDK rollback on a deliberately failed native launch remains open. [Original-process controls](evidence/2026-11-12-paired-mailbox/addressed-native-launch/original-firecracker-rollback.md).

The installed addressed SDK now creates private VM and short Unix socket directories under either tested umask. Linux and macOS both passed the ownership and signed-publication controls. [Filesystem evidence](evidence/2026-11-12-paired-mailbox/installed-driver/private-state.md).

The failed launch also exposed a coordinator retirement problem. Session probes repeatedly lost a brief daemon lock and restarted the idle timer. The watcher now retries that contention within its original admission deadline. The macOS host regression suite passed 18/18. [Original Linux observation and host tests](evidence/2026-11-12-paired-mailbox/addressed-native-launch/watcher-retirement.md)

The addressed driver checks materialized role images and preserves the initial failure report through a late coordinator exit. Its Linux library suite passed nine tests. [Input and custody checks](evidence/2026-11-12-paired-mailbox/installed-driver/materialized-driver.md).

### Addressed mailbox delivery

The two addressed workload calls and the signed route now share a 15-second host socket deadline. Linux passed 17 SDK dispatch tests and 11 witness tests. [Call lifecycle and component results](evidence/2026-11-12-paired-mailbox/addressed-native-launch/one-shot-call-deadline.md).

A signed Query can now collect its original mailbox helper after a Stop timeout. Worker credit remains charged until the coordinator observes the child exit, and a later Query returns the same terminal status. The Linux worker owner suite passed 17/17. [Late worker collection](evidence/2026-11-12-paired-mailbox/addressed-native-launch/late-worker-collection.md)

Concurrent SDK launches now wait on the tenant coordinator’s original publication lock. A surviving intent after writer failure still refuses admission. Linux passed ten lifecycle tests. macOS passed the same ten and fifteen public coordinator integration tests. [Publication and crash-recovery results](evidence/2026-11-12-paired-mailbox/addressed-native-launch/concurrent-publication.md)

The intended path is **guest A Submit → original A helper → host coordinator → original B helper → guest B Deliver**. A seals the submission. The coordinator handles the signed Send credential and opaque envelope. B authenticates and opens that envelope before invoking its guest. The following steps have been exercised individually.

| Step | Tested behavior | Evidence |
|---|---|---|
| Launch A and B | Two SDK launches jointly publish complementary signed boot roles. Each workload edge has independent Linux measurements. | [SDK publication and workload tests](evidence/2026-11-12-paired-mailbox/guest-sdk/README.md), [Firecracker guest inputs](evidence/2026-11-12-paired-mailbox/guest-images/README.md), [addressed driver and boot attestation](evidence/2026-11-12-paired-mailbox/installed-driver/README.md) |
| Authorize and seal | Original endpoint keys bind the approved direction. Send and Receive grants are checked independently. A can reuse its original ciphertext after a reconnect. | [Directional keys](evidence/2026-11-12-paired-mailbox/directional-key/README.md), [grant checks](evidence/2026-11-12-paired-mailbox/addressed-route.md), [source sealing](evidence/2026-11-12-paired-mailbox/source-cast.md), [source helper retry and B readback](evidence/2026-11-12-paired-mailbox/source-helper/README.md) |
| Request one addressed transfer | The SDK sends a host-signed turn to its original coordinator. The two original helpers must be Serving with the same direction. Each source generation permits one attempt, including when the reply is lost. | [Signed control, drain and replay checks](evidence/2026-11-12-paired-mailbox/signed-control/README.md) |
| Pull and forward | The coordinator identifies the original source process and reserves memory before its one-byte pull. Reconnecting to A reuses its 32 KiB. A lost response from B retains B’s separate 32 KiB until collection. | [Source pull and exact framing](evidence/2026-11-12-paired-mailbox/source-pull/README.md), [destination ownership](evidence/2026-11-12-paired-mailbox/helper-handoff.md) |
| Open at B | B rechecks policy when the guest answers. A macOS socket test carried a 4,962-byte peer frame and an 11-byte verdict for five exact payload bytes. | [Destination ingress, CPU and elapsed time](evidence/2026-11-12-paired-mailbox/destination-ingress/README.md) |

## Ordered Small FlowMux streams

The Small profile has a bounded receiver cache, independent application and artifact sequence numbers, and versioned scope commitments covering IPv6 scope and flow fields. The endpoint checks the enrolled profile during negotiation. On Windows, quota enumeration opens children relative to the original owned directory handles. Session and transmit waiters recheck their conditions when ownership changes. [Wire and ownership contract](../api/mvm-flowmux-v1.md) · [Windows storage contract](../api/windows-storage-v1.md)

Earlier Windows Server 2022 x64, Server 2025 x64 and Windows 11 ARM64 localhost tests each carried 4,096 blocks of 16 KiB in each direction. In the separately instrumented Server 2025 test, one run timed out near block 1,120 and the unchanged rerun passed in 217.420 seconds. Those measurements belong to the earlier PR source. The current-base qualification and installed MVM FlowMux integration are tracked in [#44](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6827611515).

## Published checkpoints

| Component | What the checkpoint establishes | Evidence |
|---|---|---|
| Shared object core and gateway | Versioned exact reconstruction, runtime/cache/schema selection, scoped verified storage and authenticated QUIC object service | [Object-v1](../specs/spiralcore-v1.md), [gateway](runtime-object-gateway.md), [matched results](cpu-conscious-results-2026-11-02.md) |
| Paired Linux packet gateway | Real IPv4/IPv6 TCP/UDP through TUN/QUIC, independent packet encodings, calibrated local impairment | [Packet report](packet-selection-results-2026-11-01.md) |
| Mailbox/session foundation | Canonical encrypted envelopes, independent keyless relay directions, bounded events, retained identity and explicit incomplete-Call recovery | [Events](mailbox-session-events-results-2026-11-03.md), [reconciliation](session-reconciliation-results-2026-11-03.md) |
| Installation owner and supervisor | Durable epochs, original resource-holder exclusion, exact worker custody, native transfers and a Linux death/recovery witness | [Owner](installed-owner-results-2026-11-03.md), [supervisor/native report](installed-supervisor-results-2026-11-03.md) |
| Shared managed-resource component | One bounded ledger, conservative accounting/fencing and component lifecycle tests | [Published component packet](evidence/2026-11-03-managed-resources/README.md) |
| Native Windows private storage | Isolated Win32 backend, protected NTFS profile and exact shared-core component tests | [Storage contract](../api/windows-storage-v1.md), [platform scope](installed-supervisor-results-2026-11-03.md#platform-scope) |
| FlowMux paired stream component | Authenticated TCP/TLS/object mapping, monotonic fatal delivery fencing and original FIN serialization | [PR72 release](evidence/2026-11-03-flowmux/pr72-release.md) |
| QUIC object finish custody | Original-deadline terminal ownership and retained fault evidence | [Finish packet](evidence/2026-11-03-object-finish/README.md) |

The earlier FlowMux baseline is [PR72](https://github.com/R4-Laboratories/SpiralCore/pull/72),
merged at `25a1c786263afcc0a955927f10b441cea401dbd2`. Its
[PR77 evidence publication](https://github.com/R4-Laboratories/SpiralCore/pull/77)
preserves the older baseline and rejected intermediate rather than relabeling
them as the accepted source. The final seven-role run passed **34 tests per
role**, across nine nonzero scopes, with original ZIP digests and inner checksums
verified. These are 34 tests repeated across seven native process roles, not
238 distinct tests, cross-platform network pairs, or booted MVMs.

The earlier FlowMux readiness and closure tests were published in
[PR104](https://github.com/R4-Laboratories/SpiralCore/pull/104), merged at
`4f9b4fbbf4ae2ea2d1b5c18550bb26ded7372485`. Its exact-source seven native
process roles each passed 47 component tests and strict Clippy. The
[report](evidence/2026-11-06-flowmux-readiness-main/README.md) retains the
failed Windows wake and macOS fixture attempts. This is accepted component
progress, not installed MVM or WAN qualification.

The ordered-cache and scope changes merged through [PR194](https://github.com/R4-Laboratories/SpiralCore/pull/194) at `9fc3ce12`. All seven native roles recovered 64 MiB in each direction over two local TLS endpoints. The measured streams carried 5,395,050 bytes A→B and 5,395,015 bytes B→A. The ordinary Windows Server 2025 run took 180.537 s wall and 131.625 s process CPU. Earlier Windows runs exceeded their deadlines. The final Server 2022 diagnostic recorded eight samples at or above 1.25 s against a 2 s operation deadline. Installed FlowMux forwarding and cross-host WAN acceptance remain open. [Seven-role CPU and timing results](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6833354143) · [Earlier native trials](evidence/2026-11-12-ordered-flowmux/README.md)

Before the final merge, a Windows Server 2025 ordinary stream stopped at block 2,352. In the same workflow, a separate Linux persistent HTTP test failed to reopen its cache. The Unix reopen correction passed the original two concurrent tests 30 times on Linux and preserves exclusive access during handoff. [Cache-reopen trace and results](evidence/2026-11-12-ordered-flowmux/cache-reopen.md) · [Latest Windows timing and quota measurements](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6831573987).

The verified application readback and final TLS shutdown changes merged in PR #223. Its ordinary local TCP/TLS runs on Windows Server 2022, Server 2025 and Windows 11 ARM64 each recovered 64 MiB in both directions in 184.194, 227.042 and 167.307 seconds respectively. A separate Server 2025 run with identical FlowMux and transfer crate contents reached block 3,742 before the 240-second deadline. In PR #236 the receiver kept a single checked file handle from header validation through reading its body. All three ordinary Windows runs and both instrumented runs completed on the final candidate. Two earlier receiver operations hit their 2-second deadline, including one observed during Finish processing. Repeatable Windows latency remains open under #44. [Windows source, traffic and CPU results](evidence/2026-11-12-windows-ordered-stream/README.md) · [FlowMux tracker](https://github.com/R4-Laboratories/SpiralCore/issues/44)

## Active upstream MVM coordination

As of **2026-11-13**, the published host integration is MVM fork [`7b224540`](https://github.com/Casey-allard/mvm/commit/7b22454077530e96146d68204b711de53e3d05a8). It incorporates upstream through `c0f15d82` and Ari's public local-durable PR #5377 through `f7536f65`. The public installed FlowMux result above qualifies its recorded host/guest combination. A newer upstream refresh found main `87010fc3` and mailbox PR head `2e5ec98d`; those later changes are reviewed but are not part of that native result. Ari's journal/signer/service ownership remains intact. [Refresh and integration consequences](https://github.com/R4-Laboratories/SpiralCore/issues/42#issuecomment-6849111548)

The verified-admission/Reply-authority API contribution is open upstream as draft
[MVM #5328](https://github.com/tinylabscom/mvm/pull/5328), with verified head
`78d30afb8c2d988657951933b01dad2945b7328a`. Its implemented admission capsule
retains the original ciphertext and canonical verification result; the journal
receives a borrowed authenticated view with no plaintext accessor. The
[capsule qualification](evidence/2026-11-11-verified-admission/README.md) records
24 crypto tests, seven privacy compile-fail checks, strict scoped Clippy and
independent code/Rust/security reviews. These are API checks, not durable service
or native/WAN acceptance.

The [updated ownership split](https://github.com/tinylabscom/mvm/pull/5328#issuecomment-6801025652)
is [accepted by this contribution](https://github.com/tinylabscom/mvm/pull/5328#issuecomment-6801088479):
the separate guest Receive/ACK owner retains hostd mailbox process/service/reader
and agentd guest integration; the journal owner retains the existing core journal
and focused tests/docs. We retain the four #5328 API files and disjoint original-child/
lifecycle work. This supersedes the earlier service-file assignment. The previously private
guest/journal stack is now public in [MVM #5377](https://github.com/tinylabscom/mvm/pull/5377); overlapping edits still require owner coordination. The [journal handoff proposal](../specs/2026-11-10-canonical-mailbox-journal-handoff.md)
records this split and the accepted versioned history-key lookup contract, including
the distinct envelope/history keys and the remaining key-custody trust-boundary
decision. The original-child broker work continues independently. Durable acceptance
requires current destination/Receive authority, resource admission, journal and
independent anchor commit, recoverable history-key custody and qualified service
integration; the capsule itself establishes none of those later decisions.

A narrow SDK fix is submitted separately as
[MVM #5336](https://github.com/tinylabscom/mvm/pull/5336), now at
`9105b02b4228a744c23b9742890c39db345f96cd` after merging upstream `c0f15d82` without rewriting its original source history. Its [CI run](https://github.com/tinylabscom/mvm/actions/runs/43383672321) requires maintainer approval and has executed no checks. The following regression/native measurements belong to the original `18b58e1a` head. A persistent request dropped its
mailbox grant before canonical admission. The fix forwards it for that start
while ordinary restart remains ungranted. The [recorded regression qualification](evidence/2026-11-11-mvm-sdk-mailbox/README.md)
contains actual RED/GREEN, 55 passing launch-module tests, overlapping 31-test
persistent scope, strict Clippy, source hashes and limitations. A subsequent [canonical native diagnostic](evidence/2026-11-11-canonical-native-sdk/README.md)
now verifies all 1,024 binary bytes after this fix. Setup was 658.948 ms,
functional receive 104.022 ms (including bounded probe delay), attempted lifecycle
2,821.558 ms and host-orchestrator CPU 475.934 ms. The complete gate remains
failed: Stop/remove returned success but the original helper was still a zombie.
[Issue #111](https://github.com/R4-Laboratories/SpiralCore/issues/111) tracks the
missing lifetime owner/collected exit. This is one local raw-mailbox case,
without wire counts, guest CPU, compression/zstd comparison or WAN qualification.

An earlier proposed reliable local-service contract and bounded frame codec are public
on the [development branch](https://github.com/Casey-allard/mvm/tree/89c24132851b7b7e78eb2ec4e45a3153f2718247).
It incorporates the SDK grant fix and upstream through `46d50a5f`; runtime
activation remains unavailable, and qualification of the composed source is
separate from earlier component tests. QueueFull retention and detached lifecycle
changes from upstream #5325 have merged. Current upstream `a707f5e8` also includes
[#5339](https://github.com/tinylabscom/mvm/pull/5339), which stops replying to a
peer that sent no frame; that host-side service fix must be incorporated before
new integrated acceptance. Its live HVF recheck remains separate. Lost-verdict
recovery, bidirectional typed
operations, aggregate service ownership, cross-host key establishment and final
native/WAN/platform/security acceptance remain open.

The [inactive worker-control foundation](evidence/2026-11-11-mailbox-worker-foundation/README.md)
is public at MVM fork `b08bbebd`. Twenty control/descriptor tests pass on its frozen
Linux source. It adds no active production child owner; non-Linux delegation
remains Unsupported at that revision. Original-child runtime ownership is a
separate development successor. Native delivery, cleanup and the complete
production signer/worker integration remain required by issue #111.

The public worker-owner successor is MVM fork
[`058d6a94b`](https://github.com/Casey-allard/mvm/commit/058d6a94b4cda0803f1a4c399c9c08cc6e948116).
It retains original worker children and bounded in-process reservations, with
default activation still unavailable. The existing resource-lease component is
now also imported at
[`c009524ea`](https://github.com/Casey-allard/mvm/commit/c009524ea1da35db52ea909084220b10c36aafd2):
11 integration entries and six private units pass on the declared Linux
composition, with strict scoped Clippy and preserved MIT provenance. The
[source-bound report](evidence/2026-11-11-mvm-resource-owner/README.md) distinguishes
participating-holder exclusion from original-child collection and records the
additive qualification overlay. [Issue #114](https://github.com/R4-Laboratories/SpiralCore/issues/114)
tracks cooperative admission fencing and watcher/worker/signer drain. The first
integrated fresh Linux host-process idle witness now passes, with original signer
collection before exit42; strict and restart-consumer gates remain open as
recorded below.
The [upstream ownership index](https://github.com/tinylabscom/mvm/issues/4629#issuecomment-6794294611)
keeps these implementations visible before final service/WAN qualification.

The resource primitive also has standalone upstream draft
[MVM #5340](https://github.com/tinylabscom/mvm/pull/5340), exact head
`7cd704aefe01ed90169037415dd1574f876fdfea`. Fresh Linux execution passes its
11 integration entries, six private units and strict scoped Clippy on the
immutable export, with all 4,446 regular source files unchanged. The process
handoff successor
[`b0958e006`](https://github.com/Casey-allard/mvm/commit/b0958e006a9bd6b0dcdb0dcf771442fc60ce762c)
adds real CLI/signer lease adoption and retained watcher jobs. Its clean export
passes three handoff entries, five watcher entries, three actual CLI entries
and three strict scopes, with all 4,466 regular source files unchanged.
The [new report and retained runtime failure](evidence/2026-11-11-mvm-process-handoff/README.md)
record source hashes, fixtures, the inherited-EX-lock repair and the CLI build's
missing embedded-host assets. These are qualified components; installed
coordinator/service activation, native retirement and WAN acceptance remain open.

The lifecycle/admission successor
[`50877f564`](https://github.com/Casey-allard/mvm/commit/50877f5641e96d32838120706c185db225195d80)
is also public. It adds the four-phase tenant lifecycle gate, original preboot
reservations and Running-publication fences. The [source-scoped checkpoint](https://github.com/tinylabscom/mvm/pull/5328#issuecomment-6800386474)
records 14 lifecycle, three classifier and 107 client tests plus strict checks on
the declared development composition. Separately, both required gated compile
commands passed on an immutable export of that exact commit, with all 4,471
regular source files unchanged. Original failed client fixtures and the missing-Zig
PATH attempt remain retained. These are component and compile results; actual
cooperative coordinator activation, installed retirement and final native/WAN
acceptance remain open.

The [cooperative-idle checkpoint](evidence/2026-11-11-cooperative-idle/README.md)
is public at MVM fork
[`0aa190b32`](https://github.com/Casey-allard/mvm/commit/0aa190b32753b6862bed51a6c7a2d2a0b6b701e4).
Its declared runtime composition passes 25 selected outer cases, including the
actual production agent/signing-helper process: ready1, one original collection,
zero fixture-adopted reaping, then exit42 in a 1.03-second test. The packet keeps
all source differences explicit; the older core mailbox API in that runtime
composition is not relabeled as whole-commit runtime execution. Both required
gated compile commands pass separately on the exact immutable commit, with
4,478 regular source hashes unchanged. At that baseline, **strict Clippy fails,
and all four original restart consumers fail during initial startup before
registration or replay.** This repairs a scoped shutdown result and does not
complete the installed service or #114.

The newer [frontend/recovery checkpoint](evidence/2026-11-11-frontend-recovery/README.md)
is public at
[`98ec9d270`](https://github.com/Casey-allard/mvm/commit/98ec9d270f5f88606b49677c266fd08f26e97369).
It wires canonical local-home preparation before CLI and four SDK entrypoint
writers, preserving pure SDK construction, existing CLI early paths and separate
native admission. Its selected Core publication scopes pass 23 cases and strict
checks; the SDK launch scope passes 56 cases; the actual CLI first-use regression
and two caller-identity controls pass. Original API/regression failures and an
intermediate 26-case SDK fixture failure remain recorded. SDK strict checking
still fails in the Hostd dependency, so this is not workspace strict acceptance.

With canonical fresh provisioning in the original restart fixture, the four
unchanged scenarios now reach their intended setup: **two pass and two fail**.
Idle after deregistration and wrapper restart pass; worker restart and crash
mid-flight still fail during recovery. Controlled startup distinguishes a real
unprovisioned refusal from provisioned Ready/original-signer collection. It does
not qualify a nonfresh installation with missing inventory. The supplied
qualified fixture revisions are separate from published commit `98ec9d270`.
All runtime compositions and their source differences are explicit; both gated
compile commands separately pass on the immutable commit, with every one of its
4,478 regular files independently matched to Git and unchanged. Trusted Preparing
replay, stale-socket/retained-operation recovery, installed native retirement and
WAN/platform/capacity/security qualification remain open.

The [startup/publication successor](evidence/2026-11-11-startup-publication/README.md)
now records actual RED/GREEN for missing inventory after a prior resource epoch
and for direct-worker home selection. The final process composition passes three
selected cases: strict no-home refusal, the unchanged three-profile startup
control, and original signer collection before idle exit. The same control
distinguishes unprovisioned refusal, genuine-fresh readiness and used-installation
missing-journal refusal. Source and all three executable hashes remain unchanged
through execution. Separately, 21 publication/cell/token/two-record-replay cases
pass on their pinned composition; the old real-latch contention failure remains
retained. The supporting Core/VMM changes are published at
[`081901d02`](https://github.com/Casey-allard/mvm/commit/081901d0285ccf00d9dd824746354e9f82d8b6ce),
with both exact immutable gated compile commands passing and 4,478 source hashes
matched to Git. The supplied Hostd runtime overlay is distinct from that public
commit and is not an enabled production release. Interrupted-mutation caller
recovery, historical socket ownership, actual assurance activation, original-four
restart acceptance and installed native/WAN/platform gates remain open. These
functional results add no transfer-speed, capacity or FIPS claim.

The [interrupted-registration checkpoint](evidence/2026-11-11-retained-recovery/README.md)
adds a passing signed deregistration after a cancelled replacement and a physical
journal regression: a repeated VM record cannot substitute for a missing one.
Both pre-fix failures, three passing inventory cases, and the source delta are
retained. The [original-row successor](evidence/2026-11-11-retained-recovery/original-row/README.md)
reproduced two stale callers settling replacement work. Both now refuse while
preserving the replacement charge; all eight selected tests passed on one binary.
The [Linux host-agent shutdown report](evidence/2026-11-11-retained-recovery/worker-shutdown/README.md)
records all nine focused checks passing on the repaired source. After a journal
failure the client received its deregistration reply, the journal was empty,
and the worker collected its original signer and exited 1. The ordinary idle
control exited 42. The [nine-check source revision](https://github.com/Casey-allard/mvm/tree/fc7c4ab41dc8900c8f736ef5c105f814feb4e3f4) is available in one checkout.

The [installed-helper report](evidence/2026-11-11-retained-recovery/installed-helper/README.md) records 27 passing Linux checks. The coordinator collected an injected per-VM child through its original owner and preserved exit code 0. A follow-up Linux run verified that the final receipt releases its shared quota. On macOS the real helper delivered 4,098 exact bytes over a synthetic guest socket. These helper fixtures covered local control and queue operations. On MVM 0.28.0-rc.1, 19 Linux guest-agent and outbox checks passed. Two further Linux tests started the installed helper, read its empty canonical queue and collected it through signed Stop or coordinator shutdown. Each took 0.10 s with a fixture supplying the factory. [Test scope and measurements](evidence/2026-11-11-retained-recovery/installed-helper/README.md#linux-helper-shutdown-on-mvm-0280-rc1).

The [installed SDK results](evidence/2026-11-11-retained-recovery/installed-helper/README.md#installed-sdk-launch-and-reservation-recovery) cover nine macOS ownership and launch checks. A subsequent broker repair returns the sole worker slot when the factory proves that no child was created. An uncertain spawn retains that slot. The first installed profile is limited to one local worker. The [first installed Firecracker boot](evidence/2026-11-11-retained-recovery/installed-helper/README.md#first-installed-guest-boot) reached the guest control agent. The host then lost its tenant control socket before the first receive. The [repaired Linux run](evidence/2026-11-11-retained-recovery/installed-helper/README.md#installed-guest-delivered-1024-exact-bytes) delivered 1,024 exact bytes in 106.467 ms of receive time including polling. Signed Stop and VM cleanup succeeded. The tenant coordinator exceeded the first witness deadline. The [complete lifecycle run](evidence/2026-11-11-retained-recovery/installed-helper/README.md#complete-linux-guest-to-host-mailbox-lifecycle) retained the original handle through worker-receipt expiry and idle shutdown, delivered the same exact message and reached a drained tenant with zero pending operations. Direct guest-to-guest and two-host WAN qualification are next for this installed profile.

[PR #95](https://github.com/R4-Laboratories/SpiralCore/pull/95) adds canonical
transport-binding revocation to the mailbox adapter. Live Call and Reply keep
both checked policy generations through transport admission. Prepared receipts
recheck the source and return bindings before sending. The installed MVM owner
still needs to distribute current revocations and restore verified policy before
restart admission.

On that earlier merged tree, each of seven native process roles passed 113 mailbox
tests, 24 storage/publication tests and 34 FlowMux component tests. The locked
Linux host job also passed its host and shared conformance suites, strict Clippy
and the guest-entry build check. The matrix covers native process roles.
Installed guest, direct/relay WAN, capacity and FIPS gates have separate
acceptance criteria. [Verified artifacts](https://github.com/R4-Laboratories/SpiralCore/issues/46#issuecomment-6670157754)

## Full MVM acceptance remains open

The local destination journal now reuses record capacity after signed retention and compaction in the contributor candidate. Linux verified the repair and the original sender's 64-identity boundary. An opt-in original sender ordinal now passes the 64/65 capacity and settlement-retry controls, including a two-helper Linux process fixture. The guest sender now stages one original payload, requests its receipt through the original host and settles the verified receipt digest. Guest startup refuses a missing identity on an ordinal boot. One installed Linux Firecracker pair delivered 65 distinct 257-byte bodies, independently verified every signed receipt and readback, advanced the settled prefix to 65 and retired both original helpers cleanly. A Linux two-helper component recovered the same sealed request, signed receipt and 257-byte readback after both helpers were replaced. The first 2,049-transfer trial completed 1,480 rows. The B reader timed out on 1,481 after its original journal recorded a lease. B exited with code 1, and both original scopes retired. A fresh pair with bounded checkpoint deferral completed all 2,049 exact readbacks and settlements under the original one-hour grant. Live-guest helper replacement and multi-tenant capacity remain open. [2,049-transfer result](evidence/2026-11-12-paired-mailbox/sender-ordinal/native-2049-checkpoint.md). [First sustained trial](evidence/2026-11-12-paired-mailbox/sender-ordinal/native-2049-initial.md). [Original helper recovery](evidence/2026-11-12-paired-mailbox/sender-ordinal/helper-reconnect.md). [Sender window and original receipt evidence](evidence/2026-11-12-paired-mailbox/sender-ordinal/README.md) [Journal capacity evidence](evidence/2026-11-12-paired-mailbox/durable-journal-capacity/README.md) · [Sender settlement, #195](https://github.com/R4-Laboratories/SpiralCore/issues/195)

All five outcomes in the [Ari delivery plan](../plans/2026-11-03-ari-delivery.md)
are required. The linked issues were open at this snapshot.

| Gate | Required integrated result | Current boundary |
|---|---|---|
| [#43 mailbox](https://github.com/R4-Laboratories/SpiralCore/issues/43) | Canonical resident service, original admitted-plan/child ownership, aggregate plaintext/cache/workspace budgets, shutdown and orphan recovery | Published development witnesses and prerequisites do not complete the installed service |
| [#43 / #49 sustained sessions](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Final acknowledgment recovery, safe retention/expiry, native qualification beyond the 2,048-Call witness guard and measured control/data coexistence | An installed Firecracker session completed 2,311 Calls and recovered a withheld Finish receipt after replay tracker expiry. A separate durable mailbox pair verified 2,049 distinct sender ordinals. Measured concurrency and production retention remain open. |
| [#44 FlowMux](https://github.com/R4-Laboratories/SpiralCore/issues/44) | Actual admitted destination/protocol, independent queues/audit/rates, installed lifecycle and shared-core mapping | Paired stream components and one installed Linux guest passed. Full package release, sustained operation, concurrent capacity and WAN remain open. |
| [#45 owner access](https://github.com/R4-Laboratories/SpiralCore/issues/45) | Separately admitted development access and sealed typed operations, bounded lifecycle and shared-core payload mapping | Implementation is active and unmerged; no implicit shell/PTY permission or development fallback in sealed mode |
| [#46 platforms](https://github.com/R4-Laboratories/SpiralCore/issues/46) | Exact OS/version/architecture/role matrix and durable client/gateway qualification | Native component rows are recorded; remaining end-to-end platform and backend gates stay open |
| [#47 WAN/upstream](https://github.com/R4-Laboratories/SpiralCore/issues/47) | Current canonical direct/relay two-host WAN, reproducible supported package and reviewed upstream contribution | Historical WAN is a different source/transport; final canonical WAN and upstream package remain open |
| [#49 capacity](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Aggregate budgets, bounded retained state, concurrency/fairness, overload/recovery and tail latency | Two guests and logical-endpoint loads do not establish a customer fleet envelope |
| [#83 security/FIPS profile](https://github.com/R4-Laboratories/SpiralCore/issues/83) | Concrete validated-module/build/environment mapping, approved-mode enforcement, canonical crypto integration and customer security-control evidence | Explicit unclassified/CUI submission target; current builds have no established validated deployment profile |

## Installed Firecracker mailbox qualification

The [2,311-Call sustained run](https://github.com/R4-Laboratories/SpiralCore/blob/d0ede350f3055d8cd9bf6eabaaf814296ee4ef4f/docs/network/evidence/2026-11-05-installed-sustained-campaign/README.md)
transferred three 3 MiB objects on one authenticated carrier and observed
tracker expiry on both sides. The original MVM owner retired the guest before
the host independently read back all 9,437,184 bytes.

The [Finish receipt recovery run](https://github.com/R4-Laboratories/SpiralCore/blob/19cd77e39e4fc10d99431736082234a3fa22394c/docs/network/evidence/2026-11-05-installed-object-finish-redial/README.md)
withheld the receipt before send, reauthenticated the replacement and replayed
one exact encrypted Reply. The same guest finished all 20 Calls and the host
verified all seven publications after retirement. A separate focused test
refused replacement after a partial old frame.

The [combined sustained recovery run](https://github.com/R4-Laboratories/SpiralCore/blob/efcec38e265e883f8380f80e5e6cd43b622dbb5a/docs/network/evidence/2026-11-05-installed-sustained-finish-redial/README.md)
replaced the carrier after 1,540 completed Calls and observed tracker expiry
on both endpoints before the cut. The host replayed one cached Reply and
the guest resent zero Calls. The session completed all 2,311 Calls and the
host independently verified the three 3 MiB objects after original retirement.

These were signed synthetic Linux/Firecracker fixtures. Larger concurrent
data/control workloads, ambiguous partial receipt writes, the installed
release and cross-platform WAN remain open. The reports retain byte counts,
CPU, elapsed time, source pins and failed runs.

## Current installed FlowMux qualification

The public installed FlowMux runner completed a fresh signed invocation on Linux x86-64/Firecracker. At final source `a7eea837`, the guest observed the refused destination and recovered the exact 5,804-byte echo in 199.140 ms with 4.863 ms guest process CPU. The peer counted 3,661 incoming and 3,234 outgoing TLS bytes. Preparation through original-owner retirement took 15.090 s; the original guest and workers retired and four retained reservation identities were verified. The locked consumer builds use public MVM host source `7b224540`, paired with a separately pinned official rc.2 guest runtime. Both Linux feature configurations passed strict Clippy; six public-package tests and 160 default-parallel canonical controller library tests passed. Signed host audit decisions are bound to the original session/workload, while their plan/image fields remain unbound. Cross-host WAN, concurrent capacity, complete storage reclamation and final package acceptance remain open. [Final source, CPU, bytes and reproduction](evidence/2026-11-12-installed-flowmux-startup/public-final-run.md) · [First public run and retained failures](evidence/2026-11-12-installed-flowmux-startup/public-run.md) · [Earlier trial 9](evidence/2026-11-12-installed-flowmux-startup/verified-echo.md)

The [MVM contribution fork checkpoint](https://github.com/Casey-allard/mvm/commit/9c2361423c86082a48d23e3f5651cb3ff0881e3f)
includes the original host-agent registration and signer check. The
[hosted helper run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42135824037)
passed all eight tests, including foreign signer, changed registration facts,
parent revocation and same-name replacement controls.

Earlier pins on the same branch passed [x86 store cleanup and flag controls](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42132613337)
and [pre-TLS carrier and resource checks](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42134384311).
Both resource refusals emitted zero TLS bytes on an already connected loopback
socket. The funded failed handshake returned the original credit. The
[hosted scope check](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42135240936)
verified kernel CPU, memory, swap and task limits and scope retirement.

The [Runtime retention run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42140574877)
passed both focused tests. An inert endpoint and mock backend showed that the
original handle survives a boot-preparation failure, a competing operation is
fenced during cleanup, and retirement needs the original receipt. The fixture
did not boot a guest.

The [canonical executable baseline](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42141356436)
built and reproduced early EOF before the first framed Challenge. The
[reviewed fork candidate](https://github.com/Casey-allard/mvm/commit/9df8ceec88b4a49c1065719f147f323fbda75e86)
routes the actual binary into the original installed protocol. All 10 entries
passed in the [exact-source hosted run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42142321535).
The actual executable issued Challenge, adopted a valid Prepare and withheld
Ready until the audited controller is available. It also rejected an
incomplete Prepare and mixed installed/probe arguments. The next gate carries
the original audit and records descriptors through Prepare and attaches the
owned collector and FlowMux controller before Ready.

The [Hostd records-adoption run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42143892252)
passed all 13 integration tests, including inherited records identity, holder
alias refusal, and the one-time claim. Its VMM unit build exposed an existing
fixture missing the new optional field. The fixture-only successor preserved
the previous wire encoding and passed all nine selected installed VMM tests
in the [hosted follow-up](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42144793237).
The parent still needs to send both original audit and records descriptors,
and the controller still needs to adopt them before readiness.

The [original signer handoff run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42146422478)
passed 10 real-helper integration tests, three Hostd descriptor sideband tests,
and four VMM parser and readiness tests. The original signed registration can
claim the signer capability once and cannot deregister while it is held. The
parent and child still need to pass that capability through the full installed
endpoint startup and use it in the audited controller.

The [endpoint Prepare run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42147389249)
passed 15 Hostd and 12 VMM tests. The executable adopted audit and records
descriptors supplied by a process fixture. It rejected foreign VM identity,
and the VMM cases verified distinct process targets and refusal on event loss.
The registered VMM producer still needs to send the original descriptors and
the audited controller still needs to consume them before readiness.

Two earlier installed-path reports are retained here with their original source
and binary identities. The [canonical endpoint executable replay](evidence/2026-11-05-installed-endpoint-normal/README.md)
exercised the original producer, audit, collector and worker shutdown with
synthetic peers. The [installed Linux/Firecracker witness](evidence/2026-11-05-first-installed-firecracker-transfer/README.md)
booted one signed guest and verified exact 11,600-byte readbacks in each
direction and original endpoint-store retirement on its protected build.
Those results describe their recorded builds and same-host fixtures. The
current upstream candidate still needs its own installed guest and WAN runs.

The [matched cold trial](evidence/2026-11-05-installed-flowmux-matched-cold/README.md)
verified an exact 11,600-byte payload and echo on a private LAN in each arm.
Combined used 8,190 both-direction TLS bytes, compared with 16,729 for Raw
and 5,233 for whole-object zstd. The report separates peer CPU, verified
wall time and the supervised lifecycle from codec-worker CPU.
The [mixed-workload preflight](evidence/2026-11-05-installed-flowmux-mixed-preflight/README.md)
records the 1,245,184-byte fixture and the native macOS peer's refusal of a
mismatched enrollment before TLS. The matched mixed-workload transfer is
still pending.

These checkpoints cover components and host prerequisites. The current
combined installed guest transfer, original-owner retirement and real-WAN
acceptance remain open. Follow
[#44](https://github.com/R4-Laboratories/SpiralCore/issues/44) for exact pins,
failures and successor runs.

## Active work that is not an accepted main release

Development checkpoints are published for source inspection:

The [resident startup reservation report](evidence/2026-11-06-resident-startup-quote/README.md)
records the original-grant accounting tests. Its implementation remains on
the [startup source checkpoint](https://github.com/R4-Laboratories/SpiralCore/commit/76008adb31fae145f5ecc78dc2c7bee64967bc76).
The reservation still needs a verified Runtime storage-retirement receipt
before an installed launch can release the final startup charge.

| Repository / development branch | Immutable checkpoint | Acceptance |
|---|---|---|
| SpiralCore FlowMux component | [#101](https://github.com/R4-Laboratories/SpiralCore/pull/101) and [#102](https://github.com/R4-Laboratories/SpiralCore/pull/102) | Pre-authentication EOF and original-reader deadline/terminal handling are on main. Installed and real-WAN acceptance remain open. |
| [SpiralCore integration checkpoint](https://github.com/R4-Laboratories/SpiralCore/tree/codex/spiralcore-retained-catalog-20261104) | [`8476d7583e083e02d8b6bc571fc505815b85de90`](https://github.com/R4-Laboratories/SpiralCore/commit/8476d7583e083e02d8b6bc571fc505815b85de90) | Original retained-file ownership, checked disk split and catalog-loss denial; 152 library entries pass, two ignored; scoped Clippy passes with three warnings; full installed acceptance open |
| [MVM fork installed-service checkpoint](https://github.com/Casey-allard/mvm/tree/codex/spiralcore-retained-telemetry-20261104) | [`9a3a964bd540d5e74c71f9b7df93cc5593295546`](https://github.com/Casey-allard/mvm/commit/9a3a964bd540d5e74c71f9b7df93cc5593295546) | Original retained-file handoff and canonical runtime cleanup protection; non-main contribution-preparation fork |

The latest [resident/helper witness](https://github.com/R4-Laboratories/SpiralCore/blob/7a080af1049455f6cb0e07f511cb467d05a902e8/docs/network/evidence/2026-11-04-resident-helper/README.md)
executes the original canonical daemon/signer through the resident's same backend,
then verifies shutdown and final-alias resource drain. It registers no VM and
boots no guest. The [capacity prerequisite](https://github.com/R4-Laboratories/SpiralCore/blob/f7d06da6b4aa148cd348dd91df3342f1bc53e399/docs/network/evidence/2026-11-04-retained-telemetry/README.md)
records a reproduced undersized-total defect and pre-effect refusal: 23 ordinary
parent checks pass, two actual-helper cases remain ignored in that invocation,
and strict Clippy emits no warnings. Both packets bind their exact Git sources;
iterations and overlapping test scopes are not summed.

The [retained-catalog successor](https://github.com/R4-Laboratories/SpiralCore/blob/8476d7583e083e02d8b6bc571fc505815b85de90/docs/network/evidence/2026-11-04-retained-catalog/README.md)
implements the original retained-file owner, disposable/retained disk split,
bounded catalog and durable denial after catalog loss. Actual fault tests cover
new append and restart after process exit, unreadable records and changed
directory privacy. Final focused five entries overlap the 152 passing library
entries; the broad run ignores two helper cases. Scoped Clippy passes with three
unconnected-constructor warnings. All 134 supplied compiled inputs match Git;
independent source/privacy/publication reviews and original failures are retained.
Incomplete-I/O recovery, closed-debt adoption, actual producer/controller coupling
and positive installed-native retirement remain open. The installed profile
remains inactive; these results add no native transfer, new WAN measurement,
accepted capacity envelope or FIPS claim.
Upstream MVM has also changed endpoint wait-failure finality, transient
volume/secret-session ownership and HVF stop handling; the final integration must
preserve those semantics before its own qualification. Earlier witnesses retain
their original revisions.

The [immutable graph packet](https://github.com/R4-Laboratories/SpiralCore/tree/4cece3276120606b85de91c554c3d96dc6e5f6d1/docs/network/evidence/2026-11-04-immutable-graph)
records an actual locked release build and seven installed-parent component
tests. It resolves one MVM Core/Contract revision and one shared transfer core,
with no local path overrides. These tests boot no guest. The newer
[disk-retirement packet](https://github.com/R4-Laboratories/SpiralCore/tree/665e23349e8cf3c644af989b2069815f7885b779/docs/network/evidence/2026-11-04-live-disk-retirement)
records ten Linux disk-module entries (including one process helper), one
scope-binding test, and five new macOS filesystem tests that overlap that module.
It preserves the APFS retry failure and independent source review. Its typed
retirement wrapper still needs the resident consumer and a positive sealed-receipt
integration witness; it is not an installed native recovery result.

New helper deadline/admission fixes passed 18 unique selected library tests,
nine resource tests and seven helper integration tests (five start original
helper children), with protected binary and source hashes unchanged. Component
strict Clippy also passed. Three GitHub
fork pushes failed with server errors; the exact successor commit and its
original failure/success records are backed up in a
[verified incremental source bundle](https://github.com/R4-Laboratories/SpiralCore/blob/665e23349e8cf3c644af989b2069815f7885b779/docs/development/mvm-helper-deadline-20261104.md).
The fork publication subsequently recovered: the exact helper files are now
included starting at checkpoint `77752e19` and retained in the successor above.
The failed pushes and bundle remain
historical evidence.

The [earlier parent/cancellation packet](https://github.com/R4-Laboratories/SpiralCore/blob/7ff84ca7f808e1bc6d9c6d4c2ba5599207c7be47/docs/network/evidence/2026-11-04-unused-parent/README.md)
records actual locked Linux release builds against MVM777, shared coreCA and
adapter13d, without local dependency overrides. Four retirement-policy, three
resource-accounting and eight parent integration entries passed; the eight
include one API-shape test. A separate opaque Mock-refusal harness and three
unused-cancellation tests passed. Protected source/executable identities match.
These scopes overlap older tests; no guest is booted. Safe cancellation requires
the same sealed unused preparation and actual private owner, and prior admission
or uncertain effects refuse. Positive native retirement and resident endpoint
ownership remain open. Known unconnected endpoint/disk dead-code warnings remain
in the build receipt.

The [new resident-service packet](https://github.com/R4-Laboratories/SpiralCore/blob/ef4f5530cb4df41ef534dd63b149f2de5dd107b3/docs/network/evidence/2026-11-04-resident-service/README.md)
records an implemented owner of one original backend and two stored canonical
launch/stop operations. Canceling a driver wait preserves custody; caught
operation panic permanently quarantines it. Weak install-once parent registration
fixes the reproduced permanent 4 KiB charge while preserving scoped ownership
and refusal after expiry. Three private driver-policy tests and one actual
canonical two-slot unused-preflight/global-drain case pass on the final source.
Both release builds and Clippy `-D clippy::all` exit zero; six acknowledged
unconnected endpoint/disk warnings remain. Nine selected source inputs and two
protected binaries match. These tests boot no guest and supply no synthetic
native receipt. The earlier HOLD, panic RED and lint failure remain recorded.
The public graph's serde_json version/features remain separate from the active
RawValue/bootstrap model; its full composed qualification is still required.

The successor [endpoint-owner packet](https://github.com/R4-Laboratories/SpiralCore/blob/24e6c96df30a84ecb486e788a3f2b70fbfbe5412/docs/network/evidence/2026-11-04-endpoint-owner/README.md)
implements one-shot canonical partition issuance from the actual original guest,
endpoint memory/worker reservation and disposable store. The same guard remains
owned across handoff; uncertain Drop retains credit, and resident cleanup keeps
the sealed endpoint receipt outside that guard. It preserves the default 4 KiB
registry footprint. Three endpoint, four resident and four retirement-policy
entries pass on the final source; strict Clippy passes and seven selected source
inputs/protected binary match before and after. A real foreign-supervisor
regression and a caught synchronous cleanup-unwind regression were reproduced
before their fixes. Both original failures and the independent HOLD/successor
review remain recorded in 5,567 bytes of compressed official receipts.
The profile has no production activation constructor yet. Actual positive
sealed endpoint retirement, failed-disk-barrier retry, complete managed helper
model and native installed execution remain open. These tests boot no guest,
transfer no object and supply no new CPU, RSS, latency or WAN result.

The [parent/helper successor](https://github.com/R4-Laboratories/SpiralCore/blob/0bd0f4d0fc8200467a7164c7d6564adeadf27595/docs/network/evidence/2026-11-04-parent-host-agent/README.md)
retains one actual helper and original installation holder under the SAME parent
reservation. It separately pins MVM's canonical shortened broker directory;
the prior state-only check accepted a replaced broker parent, reproduced in a
real filesystem countercase before the fix. Fifteen parent checks pass, with
one real-helper case normally ignored and then explicitly executed separately.
That case proves daemon/signer startup, shutdown after admission closes,
idempotence, retained credits through owner Drop while a registrar alias exists,
and global finish only after the final alias drops. No VM or registration is
issued. Six source/manifest/lock blobs match the published tree, all 97 receipt
members verify, and scoped Clippy passes with 18 acknowledged unwired-code
warnings. Independent source review preserves the original HOLD and successor
PASS. Shipping profile, actual opaque-plan/config/anchor hook, per-VM registration
and resident/native integration remain open.

The [config-admission successor](https://github.com/R4-Laboratories/SpiralCore/blob/56db5291bf53433340eaf7102041370ee48aa7cc/docs/network/evidence/2026-11-04-parent-config/README.md)
binds actual opaque admission to the SAME original config/VM anchor and reserves
registration facts before one atomic lifecycle commit. An unsupported preflight
first consumed admission in an actual regression; the fix leaves it available
for a corrected config. Four new cases and 15 existing parent checks pass on the
final public Git dependency graph; one real-helper case is ignored by this run.
Strict Clippy exits zero with 13 acknowledged unconnected Rust warnings, all 131
supplied inputs match the code candidate, and the SDK tree matches its commit.
The earlier mixed Core/Contract graph and extraction/compiler failures remain
recorded. The 854,025-byte packet retains 83 hashed members and its index. These
checks boot no VM, register no VM and transfer no object; source/model/native
activation gates remain separate.

A native macOS probe of the newer installed audit capability found that its
process-exit event descriptor cannot cross the private descriptor-transfer path;
a separate truncated-ancillary case also exposed a receiver failure. Both are
recorded in [the platform checkpoint](https://github.com/R4-Laboratories/SpiralCore/issues/46#issuecomment-6639662172).
An early refusal and native lifetime-pipe/checked-receiver implementation are
active work. Linux component success does not qualify that new macOS mapping;
the macOS host role remains a required final delivery gate.

The current provisional two-endpoint collector allowance plus helper already
exceeds the existing 256 MiB tenant ceiling before carrier/core and other terms.
This is an admission/model gate, not a measured capacity result. The
[capacity issue](https://github.com/R4-Laboratories/SpiralCore/issues/49)
records the arithmetic; constructor measurements alone do not qualify arbitrary
PII search storage. No private per-process allocation or hidden accounting bucket
may bypass the shared host/tenant budgets.

None of these checkpoints qualifies the full combined service, WAN performance
or formal upstream acceptance. Branches may advance; immutable commits identify
each recorded source.

The [reader-cleanup successor](https://github.com/R4-Laboratories/SpiralCore/blob/6c885fe5a1a03d65a6f9093edc407c98feff31be/docs/network/evidence/2026-11-03-flowmux/validation/reader-completion/README.md)
retains the original reader handle across deadline expiry, preserves its semantic
result and shares completion custody across mapped-stream aliases. It also fixes
a terminal-close race reproduced in 9/10 original and 9/10 reader-only trials.
Only a next-frame EOF after the authenticated application-completion barrier can
complete; missing FIN/publication and fatal cancellation remain failures. Final
protected execution passes 39 entries across six objects, plus 10/10 repetitions
of the original positive fixture; scoped strict Clippy passes. All 384 executed
source inputs match committed blobs. The 21,206-byte archive preserves 182 hashed
receipts and its index. Independent Code/Rust/Security review passes this exact
component. A separate installed setup test originally failed when TLS stalled
after ClientHello; its original deadline/child-reap failure is retained.

The [TLS startup successor](evidence/2026-11-03-flowmux/validation/handshake-startup/README.md)
fixes physical EOF during authentication without changing TLS parsing or
authentication. Both socket roles first failed with timeout; final qualification
passes 43 entries, ten terminal-close repetitions and eight scoped strict Clippy
targets. All 385 executed source inputs match published Git blobs. Its 9,008-byte
archive retains 73 hashed receipts and the original index, with independent
Code/Rust/Security review. The newer installed consumer separately passes 16
focused entries for setup cancellation, reader/relay custody and close/poison
races; full resource integration and native qualification remain separate gates.
These results do not boot a VM or establish a WAN speedup.

The [requested-allocation witness](evidence/2026-11-04-owned-pii-requested/README.md)
executes all 325 nonempty ordered selections of the five canonical redaction
rules, comparing exact legacy outputs and returning every selection to its
requested-allocation baseline after drop. Its maximum observed constructor peak
is 2,254,054 bytes, above the earlier default-policy sample. Search input,
whole-process RSS and full installed budget acceptance remain explicitly open.
The compact source/results archive is 163,638 bytes; heavy binaries stay remote.

The development tree contains canonical resident-parent and launcher work,
original-process guardian custody, shared parent partitions, disposable endpoint
disk inventory, and owner-access/FlowMux activation. Focused source reviews or
local process proofs for a slice are not final native lifecycle acceptance.
The native entry must carry the original authority, actual children and held
resources through every hop; DTO fields, hashes and persisted PIDs cannot replace
those owners.

Small-resource and Ordered-retention/shared-core candidates, their TLS readiness
optimization and MVM endpoint-pool integration are also separate from the frozen
PR72 component. A macOS sustained candidate missed its original 90-second gate;
a separate shadow diagnostic completion does not turn that normal gate into a
PASS or establish a released speedup. Source qualification, capacity accounting
and matched measurements must follow the final candidate, not an earlier result.

Resource models distinguish declared logical reservations from whole-process
RSS. Disk cleanup must retain both durable inventory and actual credit through
uncertainty; stream/driver metadata must remain bounded over sustained use.
No finite lifetime request counter or periodic all-VM restart is presented as
the completed sustained model. These are active integration gates, not a claim
that every existing component already satisfies the combined envelope.

The user added [security/FIPS qualification, #83](https://github.com/R4-Laboratories/SpiralCore/issues/83),
to the MVM submission goal on 2026-11-04. FIPS 140-3 validates cryptographic
modules; exact module/version/build, operating environment, approved services,
mode and security-policy conditions must match the deployment. Transport and
canonical mailbox/access cryptography both require inventory and qualification;
changing only outer TLS cannot establish the inner cryptographic boundary.
The current source uses ring-backed transport and independent Rust crypto paths
without an established certificate mapping. No current FIPS-compliance claim is
made. Applicable CUI/contract requirements and customer authorization extend
beyond cryptographic validation. See [NIST FIPS 140-3](https://csrc.nist.gov/pubs/fips/140-3/final)
and the [CMVP register](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules).

## Read the native evidence by its actual scope

The [installed-supervisor report](installed-supervisor-results-2026-11-03.md)
contains current published local Linux/Firecracker and macOS/HVF pairs, exact
reopened lengths/bytes, lifecycle costs and the Linux original-process death
witness. Its larger tier has eight **14,851,384-byte** targets. Each guest has
one vCPU/1 GiB, at most two run concurrently, and no guest NIC is admitted by
these mailbox experiments. Configured limits are not measured RSS.

Canonical stream counts include both directions. A two-guest relay has two
vsock hops, so accounted stream bytes on both hops are twice the logical count;
neither is captured physical-network traffic. Sender/receiver guest CPU and
host/VMM CPU are separate, overlapping domains. No energy or send-only time is
derived from CPU. Fixed-order local pairs do not establish a causal OS comparison.

The [sustained broker report](sustained-broker-results-2026-11-02.md) records
13,684 Linux logical-endpoint operations and 20,537 exact readbacks, alongside
separate bounded native pilots. Logical endpoints are not the same number of
booted VMs. The complete installed cross-process resource/fairness envelope
requires its own saturation and recovery qualification.

## Platform interpretation

- Native MVM hosts in this program are Linux x86_64/Firecracker and Apple Silicon
  macOS/HVF, within their recorded profiles. Native HVF death recovery remains open.
- Linux ARM64, macOS Intel and Windows native client/component results establish
  their named process roles, not additional native MVM backends.
- Windows Server 2022/2025 x64 and Windows 11 ARM64 have native private-storage
  and portable stream/client component results. The private-store profile is
  NTFS-specific; full durable gateway/restart/WAN acceptance remains open.
- TUN packet attachment is Linux-native. A portable TLS/object component does
  not by itself supply a Windows or macOS TUN adapter.
- Cross-compilation and WSL do not substitute for native Windows execution.

See the [platform contract](../specs/2026-11-02-mvm-platforms.md) and original
artifact identities for exact versions. Do not pool a newer local host,
historical native backend and seven CI component roles into one platform claim.

## Failures and measurement rules remain visible

The linked reports retain smaller-session CPU/lifecycle losses, unrelated-data
overhead, schema/drift costs, congestion drops, original platform/fixture failures,
and rejected source/coverage attempts. A source correction or same-source rerun
does not erase the first failure or prove a speculative cause.

Object-finish tests distinguish authenticated completion from honest bounded
failure. A dropped encrypted close-phase datagram is not automatically a faithful
first-CLOSE-loss witness. No universal terminal-packet-loss recovery is claimed;
read the [finish evidence](evidence/2026-11-03-object-finish/README.md) for that gap.

Matched results require the same input, setup, policy and accounting boundary,
exact reconstruction/readback, both-direction bytes, endpoint CPU, completion
and lifecycle costs. Historical HTTP bodies, captured IP and encrypted canonical
streams are different counters. Existing gains are workload-specific; failures
and finite-queue losses remain part of the result.

[Public artifact index](public-artifacts.md) · [Optimization history](optimization-ledger.md) ·
[Delivery guide](delivery.md) · [Contribution workflow](../../CONTRIBUTING.md)

No FIPS/CUI, host-blind security, physical-energy, exact cross-node clock,
general-purpose router or production-fleet qualification is claimed.

The one-tenant worker-control tests distinguish mutation-lane EOF from a full-workspace connection reset. Both targeted controls now pass within the original 1 MiB limit. [Contention results and open collection assertion](evidence/2026-11-12-paired-mailbox/addressed-native-launch/worker-control-contention.md).


## Latest native platform checkpoint

[PR #256](https://github.com/R4-Laboratories/SpiralCore/pull/256) merged the public installed FlowMux package and controller/parent ownership work at `e1a662c3`, after 28 executed CI jobs passed. The original Windows Server 2025 sustained component gate recovered 64 MiB in each direction in 211.688 s total lifecycle time, using 88.859 s process CPU, while retaining 64 publications per endpoint. A separate native coverage run measured 94.94% production-line coverage for the changed Windows quota module only. Prior deadline failures remain recorded; these unmatched hosted runs do not isolate an optimization speedup. [Bytes, CPU, elapsed time, source/artifact pins and coverage](evidence/2026-11-13-windows-sustained-stream/README.md).

This does not qualify a Windows MVM host, canonical two-host WAN, customer capacity or the newer MVM mailbox integration. The current public installed native witness keeps its original `a7eea837` controller and `7b224540` MVM host source identities.
