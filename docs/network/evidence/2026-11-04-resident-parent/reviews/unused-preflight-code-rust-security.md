# Unused host-lifetime preflight: scoped Code/Rust/Security review

2026-11-04. **PASS for the frozen eight-file overlay; no blocking finding.**
The reviewer proposed the preceding contract but did not author this product or
tests. Review used the immutable extracted archive, not the evolving shared
MVM checkout. No build, execution or product edit was performed by the reviewer.

Source archive SHA-256:
`016182f9946d5e6b66566faab43d8ea3436f74aa984dffe509d0e7a025e51a52`.
Its JSON identity record is
`ec8131351950983067d6b213fbf854ff708933bb0d668aedc1d601506a0655de`.
All eight extracted files independently matched that record:

| File under MVM `crates` | SHA-256 |
| --- | --- |
| `mvm-core/src/host_lifetime.rs` | `4f85f0957a3f7e86008ae41edc9233ac71656c31b59ad6576eb1c0008b35e3a3` |
| `mvm-core/tests/unused_preparation.rs` | `eb95160f0ab5eee9459f72cf43467274d6dc86f0cc6e12109d27a9aaf27e56a0` |
| `mvm-client/src/launch/mod.rs` | `15cb1cc50f2271e23a4285d45c05e758669269aad550d0b89a670dec8503cd56` |
| `mvm-client/src/launch/host_lifetime_preflight.rs` | `3683d4d156e7485aa0572543242826279dc8b1884d96ff30a59dd7743f9ee716` |
| `mvm-client/src/launch/request.rs` | `27b3afe83fd354c00e8959922b023bea07a0827b5f4bb3bc44a1a7116653522d` |
| `mvm-client/src/lib.rs` | `15221e829a17bced3d8825948057f13c163d06bb75c84a1285a96d561804527a` |
| `mvm-client/tests/unused_host_lifetime.rs` | `b5153d2543a4736fc75312b2eafe103b9b556b1d5386e37f61b5a2ed1d7d48e6` |
| `mvm-client/tests/unused-preflight-harness/Cargo.toml` | `1f70f01e1998866fdfc0898897fc13ef2f91cc6b9548a6e782270bc260202a85` |

## Positive unused evidence has a precise boundary

The source has exactly four direct calls to the restricted `unused` constructor,
all before `lifetime.into_anchor`: named transient shape, explicit original
backend match, installed-provider static lookup, and backend support. These
branches do not sign/admit a plan, create an anchor or invoke native launch.
They move the SAME lifetime into the sealed wrapper; there is no rebind, lease
duplication or caller-selected success flag.

The read-only request intent getter clones the original builder fields and does
no filesystem/admission work. Conversion follows it. Missing-name propagation,
anchor/backend mismatch, bind_to_config failure and every returned launch error
after conversion use the `converted` constructor, which always holds None.
No generic mapper returns an unused certificate for later errors. Cancellation,
unwind and a dropped async future do not return a certificate and remain
conservative for the armed resident owner.

The legacy method delegates to the additive method and maps into_error. Original
MvmError content/signature and ordinary lifetime destruction are preserved;
discarding a certificate does not manufacture the Root completion proof.
This change does not broaden a native backend's supports_host_lifetime flag.

## Linear wrapper and internal identity

UnusedHostLifetimePreparation has a private original-lifetime field, no public
constructor, Clone or serialization. Its API exposes read-only binding, a
Boolean concrete-owner identity comparison, and consuming into_prepared. Debug
prints binding metadata, not original descriptors or the erased owner. The
failure's private optional Box reduces its inline size without cloning the
preparation; into_parts consumes and unboxes it exactly once. The only creation
helpers are restricted to the launch module's internal scope.

Core's owner erasure adds Any to the private Send+Sync box. The bind input already
requires 'static, so this does not weaken its lifetime or auto-trait constraints.
`retains_parent_owner` internally downcasts only to `Arc<T>` and applies
Arc::ptr_eq. Equal String contents, an independently allocated Arc or a different
erased type refuse. No public Any/downcast/FD/owner getter escapes. Conversion
moves the same private box into the anchor; its destructor still retains the
same actual parent until last owner drop. New binding access is descriptive,
not authorization.

The Root consumer must still apply the separately approved cancellation checks:
full original binding and private grant Arc, cleanup-safe original ledger,
admitted-plan absence and NeverIssued state under the shared lifecycle lock.
The certificate proves only this preparation was unconverted by this invocation;
it does not erase prior work. Retrying after Root cancellation does not undo
Retired or issue fresh resource permission.

## Evidence and limits

I read the retained protected outputs: Core3/3 and Client4/4 pass, with one Client
entry being a helper that is also invoked once in a separate process for the
installed-but-unsupported branch. The nested process reports its own1/1 with
three intentionally filtered siblings; these are not eight unique tests or
native launches. The separate process respects the provider's install-once
registry; there is no reset or test-only public certificate constructor.

Core cases cover actual private SH preparation, exact Arc versus equal metadata,
different erased type, and last-anchor retention. Client cases cover three pure
refusals in one case, the isolated unsupported branch, original ownership across
into_parts/retry, and the legacy error/drop path. Synthetic state bytes are Core
continuity fixtures, not a canonical Root owner/admission or native-exit proof.

The execution record states the eight source inputs, config/lock and two protected
binaries matched afterward. Metadata records one selected identity per critical
package with Tokio1.53.3/Mio1.2.1 under the explicitly intermediate path-overlay
configuration. This is not final published all-Git qualification. Full strict
lint/platform qualification is not inferred from these executions.

The post-anchor None path is source-reviewed but not executed by this default
native-flags-false harness. Actual native retry/cancellation/failed-startup cleanup,
Root cancellation consumer and final combined source remain subsequent gates.
No code change is requested by this scoped review.
