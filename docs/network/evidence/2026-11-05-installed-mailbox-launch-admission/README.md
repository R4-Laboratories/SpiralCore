# Signed launch selection for the installed MVM mailbox

This checkpoint adds a fail-closed launch admission gate to the isolated MVM
SDK composition used for SpiralCore's installed-service integration. It
requires an explicit host selection and a unique matching signed policy
identity, after signature, validity and bundle verification and before the
nonce is spent. The first profile is Linux Firecracker with a verified bundle
and HostVsockProxy. Ordinary launches keep unrelated asset names.

A matched selection carries launch intent. The audited Hostd dispatch gate
refuses before backend start until the original dedicated mailbox listener
and worker are available. FlowMux policy and per-message mailbox capability
checks retain their existing authority. This checkpoint started no guests and
measured no transfer performance.

- [Launch contract](launch-contract.md)
- [Next listener custody and shutdown contract](custody-contract.md)
- [Replayable admission and regression patch](installed-mailbox-admission.patch)
- [Expected test-first compiler diagnostics](red-expected-diagnostics.json)

The patch applies to the installed SDK composition's
`crates/mvm-hostd/src/plan_admission.rs` with SHA-256
`3113d9e0f33442fed290cf1fed53bf18061fa023b35161d724d27d3189ff0f65`.
Patch replay matched both final input files byte for byte. The compatibility
review preserves ordinary admission when a non-policy asset happens to share
the reserved name, while selected admission still checks every asset with
that name across kinds.

The standalone service, guest image, actual rate enforcement, and installed
native tests remain tracked in [SpiralCore #43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and [MVM #4629](https://github.com/tinylabscom/mvm/issues/4629).

## Verification

The pinned test-first build exited 101 on the expected missing enum and
methods. The initial implementation passed its focused test. The
compatibility successor passed the expanded regression and three adjacent
Hostd controls using the same protected Linux release test binary. All
5,410 source entries matched their before and after inventories in each
phase. Independent code, Rust and security reviews accepted the final
source scope.

The protected build used locked, offline `cargo test --release --lib
installed_mailbox_selection_precedes_nonce_spend_and_unbound_dispatch
--no-run` for `mvm-hostd` in the installed SDK composition. The protected
test binary was hashed before and after execution. [Phase identities and
test results](result.json).
