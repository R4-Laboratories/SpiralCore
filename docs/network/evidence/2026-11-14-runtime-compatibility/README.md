# Authenticated mailbox runtime compatibility

The contributor host now checks the selected runtime's authenticated mailbox
capabilities before dispatch. A valid signature alone cannot establish that an
old guest understands a new boot record or directed/object journal semantics.
This is the host implementation checkpoint for [#268](https://github.com/R4-Laboratories/SpiralCore/issues/268), not a completed installed transfer.

## Source and behavior

Contributor MVM [9394d74f26d13424afeaade1ace9ece0f7a29dd8](https://github.com/Casey-allard/mvm/commit/9394d74f26d13424afeaade1ace9ece0f7a29dd8)
incorporates upstream main `50b1a239b75855150fa7fc2fd185b202c733ccdd`.
The [design](../../proposals/2026-11-14-runtime-mailbox-compatibility.md)
and contributor [contract](https://github.com/Casey-allard/mvm/blob/9394d74f26d13424afeaade1ace9ece0f7a29dd8/specs/contracts/mailbox-reader-runtime-compatibility.md)
define the optional closed declaration in the existing runtime assembly.

- Legacy absence means a 2048-byte baseline with no new mailbox extensions.
- Directed journals, object pairs and sender ordinals require their declared
  semantics even when a boot record would fit the old byte limit.
- The actual serialized binding must fit both the selected runtime's bound and
  the host's bound before it is written or execution begins.
- The canonical producer verifies compiled protocol and guest implementation
  inputs and rechecks captured source fingerprints. Assembling arbitrary guest
  bytes cannot give them the current host's capability declaration.
- Verified provenance stays private and comes through the existing runtime
  lifecycle gate. Source evidence stays distinct from release authentication.
- Existing baseline snapshot behavior remains. New-extension snapshot claims
  refuse until an independent original-reader rekey is implemented.

## Executed checks

Wick's tracked source matched all **4,673 files / 64,833,785 bytes** at the
published revision. The compact [results](results.json) retain the source
manifest identity and original log hashes. Builds reused the shared Cargo
cache with two jobs; no new local Mac build tree or VM disk was created.

| Check | Result |
| --- | --- |
| Focused compatibility, producer and launch checks | 86 passed; one existing ignored check |
| `just check::gated` at `92f86e6c` | Passed Linux all-targets and BDD-feature compilation before publication |
| Full workspace tests at final `9394d74f` | Passed |
| Strict workspace all-targets Clippy at final `9394d74f` | Passed |
| Final transferred-source comparison | No mismatches |

The final commands were `cargo +1.97.1 test --locked --offline --workspace`
and `cargo +1.97.1 clippy --locked --offline --workspace --all-targets -- -D warnings`.
Tests used `RUST_TEST_THREADS=2` and the existing documented
`MVM_GPU_TEST_EXPECT_NATIVE=1` because Wick has an installed NVIDIA GPU.
This is validation of host code, not a transfer CPU measurement.

Retained failures matter: an invalid Ed25519 fixture was repaired with a real
verifying key; an initial GPU probe expected a driverless host; and Clippy
required an idiomatic conditional guard. No production authorization check or
lint was disabled to pass these gates. A source/security review also caught an
incomplete producer-input guard; the corrected guard includes actual guest
reader, journal and dispatch implementations, with stale-input refusal tests.

## Guest artifact and remaining acceptance

The fork's existing unsigned `guest-bins` workflow dispatch returned HTTP 422
that Actions was disabled, despite its settings reporting enabled. No hosted
run was created. The same canonical producer subsequently completed on Wick for x86-64 and
AArch64 using pinned Zig 0.13.0 and cargo-zigbuild 0.23.0. The clean-source
archive passed MVM's canonical reader and has all three expected compatibility
features for both architectures. See the [guest-build receipt](guest-build.md),
including the retained dirty-provenance failure and separate clean rebuild.
Its output remains an unsigned contributor build.

The old signed runtime stays immutable. This checkpoint does not provide an
accepted signed replacement, a new positive native application transfer,
WAN speed, or FIPS qualification. Consequently bytes, transfer CPU and transfer
time are **not measured here**. Canonical signing/publication and actual paired
delivery, ACK, independent readback and original-owner retirement remain open.
