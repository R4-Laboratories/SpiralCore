# Independent SmallV1 CODE/RUST and scoped trust-boundary review

PASS for the additive shared-core component at the source identities below.
No builds, tests or shared-core edits were performed by this reviewer. The
component does not establish a complete process, carrier or native-host budget.

Reviewed the current transfer delta, constructor/profile plumbing, sender
candidate/prediction/base/copy retention, receiver preallocation/admission and
history/namespace ceilings. General delegates the existing settings; profile1
and2 refuse mixed agreement. Small supports64-bit source-qualified bundled
zstd1.5.7 only, refuses graph/trained-dictionary/decoder-cache activation, and
checks target/capacity/artifact and recipe output shape before relevant work.
The receiver also refuses an oversized existing General publication used as a
base before reading its bytes or admitting an attempt.

Read the pinned C implementation: fast omits chain, row and optimal tables;
minMatch4 bounds sequence count by blockSize/4, with8-byte SeqDef and three
sequence-code bytes. The4*n token allowance plus fixed context/entropy/temp/
alignment slack and4*2^14 hash charge bounds the one-shot source shape; workspace
replacement frees the old workspace before allocation. Prefix remains by-reference;
one-shot decoder does not allocate an attacker-sized streaming window. Runtime
uses integer output-shape preflight before evolution and reserves its explicit
prediction cap. Existing conservative schema and ledger scratch remain charged.
The source digest gate rejects unqualified/replaced C sources before setup;
sizeof is a drift guard and is not used as an allocation proof.

Two actual review findings were fixed with retained semantic REDs: a Small
PreparedSelector could take/prune the Sparse path before a per-codec bound, and
Small PreparedPrediction::prepare did not retain the constructor's profile.
Current selector guards target/base before delta/hash/probe/copy; prediction
stores its profile and applies the Small bound before residual creation. The
reviewer's initial32MiB description of General MAX_OUTPUT was incorrect: both
current and pinned earlier source use16MiB. No General regression was observed;
the corrected branch preserves its old explicit MAX_OUTPUT guard verbatim.

Read actual author receipts: both finding regressions exit101 on old behavior;
prepared-bounds-green exits0 with7 passes; final release scopes total144 passes,
0 failures and one explicitly unrun historical CPU diagnostic (not an all-tests
zero-ignored claim). Strict all-target Clippy receipt exits0. General/Small
measurement comparisons and their losses remain separate evidence; this review
makes no universal byte/time improvement, RSS, energy, WAN or native-MVM claim.

Exact declaration:8,284,092 bytes for ONE local ObjectSender+ObjectEndpoint
engine, held together through all retained state. A two-peer exchange has two
such engines. Parent admission must additionally bound TLS, plaintext/transport
queues, worker stacks, allocator/process overhead and all concurrent engines;
serialized quotas or registry/window credits are not that resource grant.
The remaining actual parent/standard-child/native/scale gates stay open.

Source SHA-256 pins:

- resource_profile.rs:8adf3036094baf62c7b15c239b4fb0d3de4fa1e8e720730887ba18d4d5e95875
- schema_selection.rs:ae2bdd4c9d5effd5070edb94b49e7b7795fdb779b6f2dafcc197f1d2f1fd3c44
- shared_transfer.rs:9ecf96dbcf0565b22f1a2ec3769251d86c6499f68b6dba93b2d154e39c28df8c
- endpoint.rs:b77de06664915727d22e6c1adef089dc3394fcaaa90722d5d521eba8dda25577
- session.rs:27dd1727c60bc7cf40dfa16d64f39a5d2a5d1277892582bd97415c89c99987c3
- zstd_whole.rs:cdf61642cd959cae38023857889e157bcd28c2dfee3c0ab1230cf9ba913004e3
