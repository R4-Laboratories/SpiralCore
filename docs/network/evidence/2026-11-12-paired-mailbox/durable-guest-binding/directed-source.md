# Guest A's pending send to B's local journal

The candidate extends Ari's trusted-host journal draft with one outbound direction on A's signed plan. The direction identifies A's sender, B's original plan digest, nonce, boot, complete mailbox address, receipt verification key and epochs, three signed credential digests, the allowed time interval and maximum payload. The original A identity-drive projection binds those fields to A's final signed plan, nonce and cold-boot identity. A plan without the field keeps its previous serialized form. The generic plan issuer refuses a proposed direction until the trusted paired admission path supplies both originals. [Contributor source](https://github.com/Casey-allard/mvm/commit/af1aaca5b3f926f80f7abf428342ac8b12a12ff5) · [Base MVM draft](https://github.com/tinylabscom/mvm/pull/5377)

The workload can submit one immutable message through the existing local reader and writer. A versioned availability request must match the commitment, recipient, size and validity interval. The file holds the canonical recipient, stable message ID and plaintext bytes. The slot name commits to the original direction and boot. An older reader refuses the new operation before the writer stages the file. A replacement generation uses another slot. At dispatch the guest checks the original scope again. It requires B's signed V2 local-durable receipt, full B address, execution epoch, exact message ID and valid observation time before unlink and directory sync. B's journal and the authorized host helper retain responsibility for encryption, ciphertext correlation and durable acceptance. The availability reply confirms readiness only.

This first profile has one pending slot per original cold boot in guest tmpfs, a three-dispatch limit per live guest listener and the existing bounded terminal archive. A lost response keeps the original bytes. Exhaustion preserves them under the original commitment and ID. An interrupted listener can lose its in-memory attempt count before archiving. Cold reboot or VM migration provides no automatic recovery of those bytes. The workload must retain its original intent separately if it needs cross-boot reconciliation. The helper still enforces revocation, credentials, current epoch and the destination's receipt; the guest's local availability check supplies none of that authority.

## Executed on macOS arm64

Six directed component tests and nine original guest-agent mailbox tests passed. The real Unix stream fixture used a synthetic destination signer and one 257-byte binary payload ending in zero. The first peer consumed the complete request and closed. The second accepted the byte-for-byte identical request and returned a signed B receipt. The guest file remained pending after the loss and was removed after receipt verification. The 325 bytes sent on each turn comprise 4 length bytes, 32 recipient bytes, 32 message ID bytes and 257 body bytes.

| One isolated process sample | A to peer | Peer to A | Guest-component wall |
|---|---:|---:|---:|
| Receipt lost | 325 B | 0 B | 1.383 ms |
| Exact retry, signed B acceptance | 325 B | 1,009 B | 7.350 ms |

Across both turns and the test harness, the process used 4.195 ms user CPU and 12.735 ms system CPU over 50.252 ms wall. The table times begin at guest forwarding, after test socket setup, and end after outbox disposition. This fixture uses the MVM wire directly. A booted two-guest transfer of an encoded SpiralCore frame, actual guest-vsock counts, WAN time and any compression comparison remain open. [Machine-readable measurements](directed-source.json) · [Paired integration tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168)

## Reproduce

Check out MVM commit `543efc7f94aa756c09785b38bdd024b6ba8199c7`, apply the [recorded patch](directed-source.patch), then run the tests with Rust 1.97.1.

```sh
cargo test --locked -p mvm-agentd --lib journal_direction -- --test-threads=1 --nocapture
cargo test --locked -p mvm-agentd --bin mvm-guest-agent mailbox -- --test-threads=1
```
