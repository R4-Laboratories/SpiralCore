# Installed local mailbox launch selection

This candidate covers the signed launch gate for a first Linux/Firecracker cold installation. It leaves the MVM mailbox envelope, capabilities, wire format, port, rate profile and message lifecycle under their existing owners. The installed service remains disabled at dispatch until the original parent owns the dedicated listener and worker.

Use the existing signed ExecutionPlan.asset_identities field. The provisional asset is kind Policy, name installed_mailbox, and lowercase SHA-256 of the exact ASCII bytes mvm/installed-mailbox/local-firecracker/v1 followed by one NUL. This version selects one dedicated local canonical mailbox, with its own listener, worker, credits and shutdown. It accepts no socket path, endpoint key, queue limit, port or bandwidth from the plan asset. This selection is candidate scope for #5057, and changes no production port reservation.

The trusted host selects the same fixed profile through RunPosture. The existing canonical admission verifies the signed plan, bundle, host ceiling and actual backend first. Before spending the plan nonce it requires exactly one asset with that name, the precise kind and digest, Firecracker, HostVsockProxy and a verified bundle. With neither selection nor asset, ordinary admission is unchanged. A selection without the asset, an asset without host selection, duplicate names, wrong kind/digest, wrong backend, missing bundle or incompatible network mode refuses. FlowMux policy and egress grants do not imply mailbox selection.

The original opaque AdmittedPlan exposes a read-only selection for the parent admission provider. A selected plan refuses at the existing start_admitted gate before native start until a later change proves original dedicated listener/worker custody. It must never silently fall back to FlowMux, control or an unbound guest service. This first slice exposes no listener and grants no operation. Each future mailbox Call, Cast and Reply must still satisfy canonical per-message authorization.

Regression requirements use real signed plans and the actual nonce ledger. An unselected/no-asset plan succeeds, either side alone refuses before nonce spend, wrong/duplicate data refuses, a matched plan yields the opaque selected token, replay of that accepted nonce refuses, and an attempt to start a selected plan cannot start a backend without the original listener.

## Compatibility refinement after source review

With no operator selection, the reserved asset name is enforced within
AssetKind::Policy. A legacy non-policy asset with the same caller-chosen name
keeps its ordinary admission behavior. With an explicit selection, count all
assets bearing the name across kinds, then require one exact Policy identity.
Exercise both one-sided refusals against the same nonce ledger before valid
admission and require the canonical typed NonceReplay error after admission.
