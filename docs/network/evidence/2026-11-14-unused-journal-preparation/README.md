# Preserve the original unused journal preparation on early refusal

Contributor MVM [0909b7ec11942abbc48c47229e27533ba170bbcb](https://github.com/Casey-allard/mvm/commit/0909b7ec11942abbc48c47229e27533ba170bbcb)
fixes an early-start ownership defect in the installed journal SDK. Previously,
the SDK converted its original `PreparedHostLifetime` before loading the stopped
machine definition. A missing definition created no VM but prevented returning
the original unused token, leaving only conservative retained custody.

The SDK now retains that exact preparation until journal preparation begins.
The original `into_unused` API can return it after an early refusal. The new
`into_unused_for_parent` method applies the same guard and returns the existing
opaque `UnusedHostLifetimePreparation` accepted by the original parent's
cancellation API. It adds no public proof constructor, owner reconstruction,
authorization path or automatic credit release.

## Ownership boundary

The transition is one-way: original unused preparation → original anchor.
It occurs before preparation callbacks or config exposure. Entered preparation
cannot become unused again, even if no boot result or VM ID was returned. Any
retained boot, support, native, helper, retirement or completion state refuses
extraction and preserves the original failure. Abandonment retains the actual
unused or anchored lifetime in quarantine.

Returning the token does not prove that earlier image resolution or filesystem
work had no effects; their existing owners govern rollback. It is also **not an
executed parent-credit refund**. The journal caller still needs to consume the
opaque proof through its original parent, and that integration needs measured
resource acceptance. General uncertain creation and detached/restart recovery
remain open in [#111](https://github.com/R4-Laboratories/SpiralCore/issues/111).

## Executed regression and controls

The actual stopped-definition-load path was exercised with a hermetic mock
backend, no supplied journal authority and a real original holder lease.
At [baseline 0f9df579a](https://github.com/Casey-allard/mvm/commit/0f9df579a), the
new test failed with exit 101 at unused-token extraction, after the expected
missing-definition refusal and an empty backend inventory. This is a component
regression, not a booted Firecracker witness.

After the fix, the returned token retained the exact original parent Arc and
binding. An independent holder could not acquire an exclusive lock while the
returned preparation remained alive; it could after that original holder was
released. Negative controls cover entered preparation with no boot result,
repeated entry, retained completion/native/helper state, missing transition
state, failed extraction followed by retry and abandonment quarantine.

| Final check | Result |
| --- | --- |
| Focused SDK journal/verb/recovery checks | 16 passed, no failures or ignored checks |
| External proof-constructor compile-fail doctest | Passed |
| Changed-crate all-targets strict Clippy | Passed |
| Workspace all-targets strict Clippy | Passed |
| `just check::gated` Linux all-targets and BDD compilation | Passed |
| Full workspace tests under umask 0022 | Passed |
| Tracked source comparison | 4,674 files / 64,846,786 bytes; zero mismatches |

Full tests used `MVM_GPU_TEST_EXPECT_NATIVE=1` for Wick's installed GPU and
`RUST_TEST_THREADS=2`. Two preexisting entrypoint tests now have Linux guards
matching the Linux-only method they call. This does not establish native macOS
or Windows qualification of the new recovery path.

The source-byte manifest matches the published final commit. Test checkout Git
metadata retained base `9394d74f`; tests ran as an observed source patch, not a
clean release build. The [structured receipt](results.json) records both facts
and original log identities. No release artifact is attributed to that test
checkout.

## Retained failures and limits

The first added negative controls had a test-only `unwrap_err` type-bound error
because the linear preparation does not implement Debug. The controls were
corrected without changing production ownership behavior.

The first workspace attempt inherited systemd umask 0002 and failed eight
existing filesystem/overlay fixtures. Private writable-image checks correctly
rejected group-writable 0775 temporary directories, and the overlay fixture
observed a file-mode mismatch. The same source passed under 0022, matching the
previous workspace qualification environment. These failures remain recorded;
no permission check, assertion or production refusal was weakened.

This checkpoint changes host SDK custody only. It adds no mailbox frame,
codec, data-fidelity or transfer-performance claim. It does not qualify actual
journal caller credit release, new native transfer/recovery, WAN, fleet capacity
or FIPS operation. [Guest-runtime build and publication gate](../2026-11-14-runtime-compatibility/guest-build.md).
