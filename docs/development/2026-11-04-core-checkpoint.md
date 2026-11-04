# Development source checkpoint: 2026-11-04

This branch preserves the latest SpiralCore Rust implementation in progress.
It is not a release or a claim that every feature/platform has passed. The
GitHub issue graph #42/#43/#44/#45/#46/#47/#49 remains the delivery authority.

The checkpoint includes Small/Ordered codec and persistent-cache work, TLS/Mio
readiness work, completed-reply-tail work, access mapping and installed-parent/
disposable-store ownership. Different component source revisions have different
qualification. The current canonical-host Cargo pin still names the earlier MVM
fork; a paired immutable dependency update and lockfile qualification follows.
Do not treat this checkpoint's canonical-host feature as a locked release build.

Latest merged ordinary-process FlowMux acceptance remains PR72/PR77. Native MVM
service/helper/telemetry integration, final exact dependency closure, sustained
canonical sessions beyond2048Calls, cross-host/platform/WAN and fleet capacity
remain open. No new performance/energy/FIPS or whole-process memory claim.

The adjacent SHA256 manifest records all changed Rust-crate files included in
this checkpoint. It excludes local runtime/sessions, tooling state, targets,
VM disks and payload stores. The development checkpoint is distinct from main.
