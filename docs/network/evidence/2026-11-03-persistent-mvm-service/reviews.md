# Review and execution identities

Product revision: `753198531d4f3ef87e8a79c63d44f4f081ce4077`.

| File | Independently reviewed SHA-256 |
|---|---|
| `crates/spiralcore-mvm-live/src/campaign/canonical.rs` | `65128439c949c07abb6ae4f6b754f02d898d0652e847488087c145edb71d3cfe` |
| `crates/spiralcore-mvm-live/src/host_budget.rs` | `38dbeef65ebe0b3be0c7c1626ef1b7644915fc0481c263282da8301e38f26405` |
| `crates/spiralcore-mvm-live/src/orchestrator.rs` | `509cab6964a8175414e3b4323395796a669b94c32b0d9ef71338f9d51a744212` |
| `crates/spiralcore-mvm-adapter/tests/service_object.rs` | `595b89b615512c48a433f397d1899fa648e73abf6ab49e8c92a7e1e47e0b74b6` |
| Native runner | `abd6fe1c4d6a21076bfce12450e6cb09c82919fd0ba7f1ea32c6980fd3a8b3d8` |

Independent Rust/code review and security review approved the declared first
slice. Both found that the initial Stage One report implied frame accounting
which that endpoint path did not perform. The budgeted API now rejects that
topology before activation; the ordinary path explicitly reports no coverage.
The source tests and actual native one-guest scope regression passed afterward.
Infrastructure review required refusing existing Firecracker processes before
launch, rather than discovering them at final cleanup; that guard was added.
No reviewer ran tests: execution evidence is the root agent's separate records.
The literal `/quality-gate` command is not exposed by this host; the required
independent code/security reviews ran in parallel instead.

The same security reviewer independently recomputed the 18 host-cell medians,
five native tables, 44 publication/36 keyless counts, two-hop byte relationships,
reported improvements and retained losses, and approved the bounded results
claims. A separate scoped coverage derivation reproduced new-module 90.40% and
changed-production 87.02%; the broader three-file 64.65% remains explicit.
Final hosted checks passed on the identical reviewed/executed source tree; the
qualification packet preserves all seven original artifact digests and 222
verified inner files. Product PR #54 merged at
`47324dca9f42e5da5685af872334c155aa19901f`.

The dispatch boundary regression was observed RED, then GREEN. Budget integration
tests were observed RED with unresolved new API, then GREEN. Relay credit tests
were observed RED with missing forwarding helper, then GREEN. Early test harness
closure/toolchain/coercion compile errors were corrected before those semantic
GREEN results and are not performance measurements. Exact pre-change RED source
variants/raw output were not separately archived; final tests and source are.

Normal uninstrumented binaries on Wick:

| Executable | SHA-256 |
|---|---|
| Before `broker_objects` | `1f3ce1caa4e9ea9ea3f833888a4a87a8b71688b819a8f0f36aaa93ad7e9e3b7f` |
| After `broker_objects` | `fd741385ce73485dbe204376e3d5dfe1c90bac66ac0c562982e2609aeb3b525e` |
| After native host | `289bb89749dd6190b244a278c77183ea1634532b92256e4b8e1b64c5683dc9e4` |
| After static GNU guest | `9fdd15d61292cd64aedee71861e7623c9be03476adf5be518e778f02508f2fa1` |

Before source is `b154fb75`, exported tracked crates. After source is `7531985`;
all 272 tracked crate hashes were checked against the remote exported source
using `sha256sum --status -c source-sha256.txt`. Builds use Rust 1.97.1, release,
locked dependencies, two jobs and the existing shared Wick target. Host/example
builds unset compiler wrappers and Rust flags. Guest adds only
`-C target-feature=+crt-static`, explicitly targets x86_64-unknown-linux-gnu,
and its retained ELF inspection has no `INTERP` entry.

The original native public-receipt archive SHA-256 is
`7f9087cdf9ed076091958a630d3b6ea4d6d278d320f24edc0ddd59b72441bcd2`.
All 56 member hashes were verified after transfer to the Mac. The archive and
expanded public members exclude guest disks, cache payloads, private bootstrap
material and launch directories. Old official evidence remains unchanged.
