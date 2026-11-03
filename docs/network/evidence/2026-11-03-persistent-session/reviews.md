# C7 source and execution identity

Reviewed product source: `91baf28efa56da94336d413ca8e912266079f654`,
merged through PR #56 as `0e02dd76c5f9064cdfc82fe21d2f831f56da454b`.
The preceding feature commit is `df8ee07`; the final commit changes only the
Clippy-requested let-chain style. All hosted checks and normal binaries use
the final source. Source manifests bind all 276 tracked crate files.

Independent code/Rust and security review approved the retained pre-send identity,
original lifetime, generation/request-pinned send and corrected completion order.
The review found clear-before-Reply and mixed-registration-generation risks;
both were corrected and tested before publication. Infrastructure review approved
the required portable test target and bounded host/native runners. A separate
performance/evidence review recomputed the data and corrected the CPU-scope
wording: measured setup includes fixture/service/codec preparation; untimed gaps
and final verification outside those intervals remain excluded.

| Product file | SHA-256 |
|---|---|
| Adapter `src/service.rs` | `7a58e929401c68f40461a762b72f88b46ef221bc83e33875013005f394e10fcd` |
| Live `src/campaign.rs` | `6ffd202bdab5f0c6bededbba68ce8062cfa97f1c44a582aef7737f0d7a1b30f2` |
| Live `src/campaign/canonical.rs` | `1bf506c5caa53fd524060e42f2bf117afe09fafebfdaecc13aec5c864ea0fe2a` |
| Live `src/campaign/session.rs` | `5a8ca6e5a6fe472e41da6b20f21e2ced72832cb4bf7c75e24e13a79b8c862820` |
| Adapter `tests/call_send.rs` | `58f474764e92b5f47967c71886505f5ac3a98fe37b43d76a8a0246161a847c61` |
| Live pending-state tests | `0c1b2743b07f8c8daf0ec5648d4a9cca8934910c3c5546b37b2cfbf6178753a1` |
| Live actual caller receipt-only regression | `090f06aa3133c671f42153357e66f76e86ee2b3b2f025e5925cc4b1b138794c4` |

Normal, uninstrumented binaries:

| Role | SHA-256 |
|---|---|
| Before object probe (product bytes of `cc94f31`/`7531985`) | `fd741385ce73485dbe204376e3d5dfe1c90bac66ac0c562982e2609aeb3b525e` |
| After object probe | `879111c94d8ed8753526758997e497ee4391b8893040b6ef1067068fe96309e5` |
| After Linux host | `7e521902a921567d9b8d8096171c5887d4a3278838d398ac70d121036a943c32` |
| After GNU static guest | `85e704f11f122b20984ba3bf6e761e7b7c5a5f7e9d76d2bfa2d2f7672220bea9` |

Builds used Rust 1.97.1, locked dependencies, release mode and two jobs in the
existing owned Wick target. Normal host/probe builds unset instrumentation
flags/wrappers; the guest adds only `-C target-feature=+crt-static` with an explicit
x86_64-unknown-linux-gnu target. Protected normal binaries remained unchanged
before/after isolated coverage. The native archive SHA-256 is
`cd21e3542233698f61e8df3d3303e8cdf1540be316d056759727706ef7943286`;
all 34 member checks passed on the Mac. No VM disk or payload is included.

Hosted portable run 41598883594 and Linux run 41598883618 both succeeded on
PR merge `065abeb6ea81896cd85ca4f660a891c93c4c5423`. Its tree and reviewed
head tree both equal `0d414f9d56929506413da64dce73f4016ecd3c18`. Original
seven archive digests and 236 inner members verified. Windows manifests use
CRLF; only the checksum parsing stream was normalized, never original files.
The literal `/quality-gate` command is unavailable in this host; independent
code/security reviews ran in parallel instead. This packet records their
declared scope, not an invocation of that unavailable command.

Coverage exports use LLVM 22.1.6 with one separate profile/merged file/export per
test binary. Forty-six tests passed with zero test/export stderr; see the exact
commands, line hits and uncovered lines under `coverage/`. Shared unrelated UOR
builds used another target and were not modified. No instrumented measurement
is credited as normal performance. C8/C9, host-wide plaintext/installation
ownership, WAN and remaining platform qualification remain open.
