# Draining a signed verdict after the sender closes its socket

A complete B journal verdict can remain in the receiving kernel buffer after B closes its end of the host Unix stream. The paired HVF trial localized its failure to the host relay's read of that verdict. On macOS, setting `SO_RCVTIMEO` again on the disconnected stream returned `EINVAL` before the relay read the buffered bytes. This happened after the relay checked A's peer identity and wrote the sealed frame to B. [Canonical relay](https://github.com/Casey-allard/mvm/blob/27038e3f322e4336df180073e12cae59cc4dba54/crates/mvm-hostd/src/mailbox_process/original_journal/relay.rs) · [Platform tracker](https://github.com/R4-Laboratories/SpiralCore/issues/46).

The fixed deadline reader takes ownership of the already authenticated connection and switches it to nonblocking mode once. Each read or write uses the original absolute deadline. On `WouldBlock`, the executor polls for readiness and checks the deadline again. The reader drains buffered bytes and propagates EOF so the canonical verifier can parse and authenticate the signed receipt. An empty, truncated, oversized or incorrectly signed verdict still cannot complete delivery. [Source and tests](https://github.com/Casey-allard/mvm/commit/8238000662f59893b6ec703ab4b37d7a86c08340) · [Reproducible patch](verdict-deadline.patch).

## Connected-socket regression

The test writes a known 1,197-byte body on an actual Unix socket, closes the writer before the first read, then compares every recovered byte and checks the following read for EOF. The other controls cover an empty peer close, a reader that sees one byte before the absolute deadline, and a writer whose peer leaves its receive buffer full.

| Environment | Before | After |
|---|---|---|
| Apple Silicon macOS | 1 of 4 checks passed. Buffered and empty peer close both returned `EINVAL`. The partial read returned `WouldBlock` at deadline. | 4 of 4 passed in 0.11 s |
| Linux x86-64 | Pre-fix suite not run in this packet | 4 of 4 passed in 0.10 s |

The standalone suites compiled the exact Rust module under each revision. They measured local socket behavior. Their CPU measurements round to zero at the process timer's reporting precision. The Linux service-level regression uses a real pair of `mvm-mailbox` executables, independent role and grant checks, and fixture guest sockets. Its byte, time and CPU measurements are recorded separately. [Source pins, executable hashes, service-level results and accounting](verdict-deadline.json).

With the opt-in recorder enabled, all 25 mailbox service tests and the separately executed original-helper pair passed on Linux. The pair recovered all 257 bytes, verified the destination ACK, and collected both original helpers unassisted. The three relay turns each wrote 5,540 bytes toward B and returned 1,145 bytes. The complete sequence through the ACK took 59.032 ms. The timed Cargo invocation consumed 0.17 seconds of user CPU and 0.05 seconds of system CPU, including its waited children.

The metered mailbox process suite passed 40 checks and failed one restart check; the two-helper test is selected separately above. The failing check restores the same VM directory and starts another helper. The original's recorder had already created its protected counts file; the second recorder's exclusive creation refuses to replace it. The restart's guest fixture timed out waiting for a connection. This concerns reuse of the optional witness recording directory, independently of socket handling. The original counts must remain reviewable across any future re-entry.

## Reproduction

From the fixed MVM revision, use the same Cargo target for the helper and host test. To reproduce the macOS failure, run the first test command on the regression revision `3323941c66bcd6522d87c3172bfe6ab8d842a791`. [Regression source](https://github.com/Casey-allard/mvm/commit/3323941c66bcd6522d87c3172bfe6ab8d842a791).

```sh
cargo test --locked -p mvm-hostd --lib mailbox_service::durable::guest_deadline::tests -- --test-threads=1
cargo build --locked -p mvm-hostd --features durable-guest-io-witness --bin mvm-mailbox
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib mailbox_service:: -- --test-threads=1
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib mailbox_process:: -- --test-threads=1
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib original_a_and_b_helpers_relay_same_sealed_intent \
  -- --ignored --nocapture --test-threads=1
```
