# Original-parent unused cancellation: scoped delta review

2026-11-04. **Code/Rust/Security source PASS; no actionable blocker.** This
reviews cancellation additions over Parent source `1770b2a0...`, its three
new standalone test sources and the explicit Git pin update. The reviewer
authored the preceding contract, not this product/test implementation. No build,
test, native process or product edit was performed by this reviewer.

| Input | SHA-256 |
| --- | --- |
| `src/installed_parent.rs` | `2d75dc18d99054666e90de18a4d0f1587c417e3293bb140fc3ea02377eb2e4c2` |
| `src/installed_resources.rs`, unchanged | `02a7e34868816ec751e249997622979eed4bf8d81dfcad96ece9234c46f6c1e6` |
| `tests/parent-retirement-harness/unused.rs` | `74c5629182e80e78064dd89a2f062bfeeffb226bb89520e5cf914eb6524979de` |
| Harness `Cargo.toml` | `ae151bcd514c6b03d4410ae8b7d3dac0929f7d61e5dd572e07f78a956f39f472` |
| Live `Cargo.toml` | `6445be44bc03022d90ee16cffe11453739548790ae6183a3b98b9d4d40dc1eb8` |
| Existing corrected negative harness `retirement.rs` | `d24958c1508d8c822a459580e05dfdf8a3107905c09b484f71c5edb518677bc8` |

Paths are relative to `crates/spiralcore-mvm-live`. The approved unused contract
remains `docs/api/unused-host-lifetime-preparation-v1.md`, SHA-256
`31b08a740530ac89926a0b0bfbe4f4e7edf1ac1f8beb86bdafa27a874143cdfd`.

## Cancellation cannot manufacture or substitute ownership

The consumer borrows the sealed canonical unused preparation. It compares the
full original binding and exact private Arc<VmGrant> identity before reading the
ledger or mutating lifecycle state. The certificate has no public constructor;
the earlier reviewed canonical API returns it only before conversion/invocation.
A correctly formed foreign preparation with copied binding and an Arc<String>
therefore cannot cancel this native grant or fence its valid ledger.

After identity, the existing cleanup-safe retained check validates the same
original slot/serial, tenant and non-underflowing accounting. Normal admission
close is allowed; actual accounting fence/poison still refuses. Ledger and
lifecycle mutexes are not nested, and no filesystem or native operation occurs.
The certificate remains borrowed throughout, retaining its original SH files and
parent Arc. There is no replacement resource pool or release/reacquire.

The private `seal_unused` requires admitted_plan=None and exactly NeverIssued,
then sets Retired under the SAME grant lifecycle lock used by canonical bind.
A racing bind either records admission first and makes cancellation refuse, or
observes Retired and refuses; cancellation cannot erase admission. Future
endpoint issuance must use that same lock before effects, as already contracted.
Preparing, Issued, Uncertain and CleanupComplete cannot masquerade as NeverIssued.

Only successful sealing sets the owner's completed disposition. Refusal leaves
the original conservative Drop unchanged. Repeated cancellation with the same
certificate is idempotent and does not subtract counters. The reservation stays
in the grant until its last actual Arc drops, including the certificate or an
anchor recovered via into_prepared. Retired continues to block canonical bind
after cancellation, so the descriptive preparation cannot reauthorize a VM.
The existing native and pending-retirement entries remain distinct and unchanged.

## Test-source quality and dependency identity

The three new tests were reported authored before implementation/API-absence RED.
Their source targets meaningful boundaries:

- Foreign full binding with a different erased owner is refused; the original
  certificate succeeds after both normal closes, repeated cancellation is safe,
  and guest credit remains until the certificate is dropped.
- Retry consumes the same preparation and retains its serial/one guest charge;
  actual opaque canonical admission cannot bind the retired grant. A surviving
  anchor still retains the charge until its own Drop.
- A previous real opaque native-shaped admission makes unused cancellation
  refuse, preserving the original uncertain charge. No native launcher runs.

Private fixtures explicitly create0700 parent directories. They do not weaken
the installation owner contract or create a public test receipt. The foreign
fixture's separate real SH file is continuity test data, not native authority.
These tests neither install the full global parent factory nor exercise native
boot/retirement; they must not be presented as those witnesses.

The manifests consistently select MVM
`77752e1967d20caf8bad710de864a0ccd967f9e6`, shared core
`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1` and adapter
`13d6ec22bab21f60bdf2a184fa9eb069e5537200`. I also read the adapter's immutable
manifest: it selects the SAME MVM777 Core/Contract and CAcore. No dependency path
override appears in the reviewed manifests; the standalone harness intentionally
selects its actual local Live crate. Root separately verified metadata with one
critical package identity; this review does not repeat that whole graph census.

The actual new public-graph build was still running when this source review was
written. No GREEN is inferred from compilation progress. The older seven
protected preflight cases retain their original source/execution identities.
Native positive cancellation/retry, complete resident wiring, final source/lint/
platform qualification and whole-service acceptance remain separate gates.
