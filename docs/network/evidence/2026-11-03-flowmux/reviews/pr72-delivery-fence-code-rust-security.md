# Independent delivery-fence code, Rust and security review

PASS for the bounded five-file delta over frozen PR72 head57c09d92. Reviewed complete delivery-fence.patch and the retained baseline source review; this is independent of the delta author. This approval does not substitute for successor coverage, native CI or canonical MVM/native/WAN qualification.

The fatal delivery flag is monotonic: stop publishes it before physical close, including cancellation after already-graceful I/O closure. Graceful completion never clears the flag. Data dequeue/copy checks current authenticated scope and fatal status; queued verified data cannot bypass an explicit cancel. The fresh delivery check is an operation-admission point, not a promise to revoke bytes midway through a concurrent copy. Zero-length reads transfer no data.

Normal worker completion is marked only after both logical halves and pending state have drained. It closes physical I/O while preserving authorized application queue drain. Pre-I/O operations see Closed after graceful shutdown; they do not promote a valid drain into cancellation. Failed, timed-out or unjoined workers still cancel. Successful join consumes its original thread and Drop does not invent a later failure; join error/timeout/panic preserves fatal cancellation. Endpoint Tx/Rx Drop does not upgrade an already completed physical close.

The new authenticated regression separately checks live-scope buffered rejection, normal physical closure plus exact reverse drain, cancellation after completion, refused writes after FIN, clone behavior and successful joined-worker drain with reciprocal byte counters. The existing saturated-queue test retains its four publication acknowledgments, fifth refusal, original timeout, failed joins and reentry refusal, and now requires repeated typed Cancelled after that fatal outcome. The original failing expectations and intermediate mistakes remain recorded; this is an intentional cancellation-contract correction rather than evidence those failures never happened.

No wire frame, codec, object identity, TLS pin, authority enrollment, timeout or capacity changes. Baseline33 coverage and all-platform success stay scoped to their old source; the successor34 execution and strict Clippy are supplied author receipts, not independently rerun by this reviewer. Native MVM, shared installed budgets, long-session performance and WAN remain open.

Exact reviewed SHA-256:

- src/tcp.rs: `8a371b15b5110ca4b5f61bfa828638e22188b87560fad10e47ee5408a21abd2e`
- src/stream.rs: `4c1c2a3982bd2286a713272b4230bc3fdd93ad06d769f32d5612e87440104ced`
- src/duplex.rs: `a5b6f8513ee4a65a9b055d49fb492698914edf589cdd55be1eba91782e0fe2ca`
- tests/authenticated_stream.rs: `8e9dc3ce34f9fc7b87aa18f8d2f2c5a3423e09f7cf6d9080673499527561a078`
- tests/duplex_tcp.rs: `b512e64ce7d13a56ccd6f5004bcc1f98a05b6ee0f24b6dd371884898e024ac0a`
