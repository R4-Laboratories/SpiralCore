# PR #68 installed-owner workflow: Step 2 INFRA

**PASS — scoped workflow source; required hosted/custody gate still OPEN.**
Follows pipeline-audit.md Step1 PROCEDURAL PASS. Reviewed
64d93fed72fdac6784d99e6e1a258052bba605f6 and its corrected
.github/workflows/spiralcore-installed-owner.yml, SHA-256
d93408234be35bcc80b8e701026e03623537a1a774b81a47f93ec112ec60ffe8.
The correction changes only that workflow: runner.temp target selection moves
from job env into the execution step env. No product/test change accompanies it.

The four explicit native runner/compiler rows match the bounded component
qualification: Ubuntu24.04 x86_64 and ARM64 (4 unit+9 lifecycle entries each),
macOS26 ARM64 and Intel (4+10 each). They record uname, exact checkout and Rust
host triple, and require the expected host. Expected aggregate is54 entries
including four subprocess helpers, not54 distinct behavioral scenarios. These
rows are Unix component execution, not native guest/VMM or Windows qualification.

Both test invocations are serial, preserving valid Busy refusal across
fork/pre-exec inheritance. Exact summary guards require4 unit entries and each
row's9/10 lifecycle entries, zero failed/ignored/filtered. Bash set-euo-pipefail
preserves Cargo failure through tee. Locked Rust1.97.1, a target under runner.temp,
jobs2, incremental disabled and a ten-minute job timeout bound the run. Clippy
checks all focused targets with -D warnings. Five source files are hashed before
execution and checked afterward. This is the exact-file focused harness, not a
claim of full-main release testing on all four platforms.

Permissions are contents:read; checkout persists no credentials. Checkout and
upload actions are pinned by full commit IDs. The workflow requests no secrets,
write token, SSH, deployment, sudo, VM boot or privileged runner. PR path filters
and explicit dispatch cover the component/workflow boundary. Artifact upload is
always attempted, including failures, with per-target/attempt names and14-day
retention; that does not relabel failed jobs as successful qualifications.

Direct API observation of run41697837824 confirms the correction is accepted:
all four matrix jobs exist at exact head64d. At review, Ubuntu x86_64, Ubuntu
ARM64 and macOS Intel jobs report success; macOS ARM64 remains queued. Original
41697401676 remains a failed pre-job validation run, not platform execution.
Partial success/job creation is not all-platform PASS. Root still must obtain
the remaining required successes and verify exact-source/native-platform/test
counts plus original artifact custody; the separate Linux main run also remains
subject to its actual final receipt. No merge disposition is granted here.

No infrastructure code blocker was found. Only this review was authored;
no workflow edit, build/test, launch/rerun/cancellation or artifact download was
performed. Full B6a/native lifetime handoff, B6b, scale and WAN remain open.
