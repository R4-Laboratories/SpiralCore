# Original helper owned by the resident service

2026-11-04. This implements the resident/helper part of [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43). It is a component of the [complete MVM delivery](https://github.com/R4-Laboratories/SpiralCore/issues/42); its shipping resource profile remains inactive.

Code candidate: `d10ca39984f2bc7bbff968d5f34fabff3c20fb2a`, tree `f6b0460457823b5005bae82545f9b56eb40150fe`, parent `56db5291`. The dependency graph remains MVM `bef4146c`, adapter `7f638428`, shared core `ca5d9ca1`, without path overrides. All 131 supplied source inputs match this candidate's Git blobs.

## Implemented behavior

`InstalledResident::start_owned_host_agent` starts only the helper admitted by the original provider and installs its actual registrar into the SAME backend. The resident stores the original helper before the fallible installation, and retains typed failed-start owners. An unrecognized attempted startup retains the error and whole resident, closes admission and refuses completion. A privately observed Unused/no-attempt refusal leaves the resident intact. Startup/unwind and incomplete Drop preserve the original custody rather than refunding uncertain work.

`try_finish` closes admission and requires all original VM operations to retire before helper shutdown. It shuts down the actual helper outside bookkeeping locks, keeps the live table on failure, and drops the entire backend/provider/helper table only after successful shutdown. An external original registrar alias still retains its resource credits; the SAME supervisor can finish only after that alias drops.

A monotonic first-VM-preparation flag prevents late helper installation after a prior operation has genuinely retired unused. It is set immediately after actual `prepare_vm` succeeds, before fallible future construction, and is never reset. The changed inline Live layout is covered by its existing size-based reservation. No new profile constructor, independent quota or VM permission is introduced.

## Executed qualification

| Scope | Actual result |
|---|---|
| Previously qualified library metadata | E0599: resident helper-start API absent; selected metadata hash unchanged |
| Initial release parent scope | 22 PASS, 2 ignored |
| Initial explicit actual-helper case | 1 PASS, 0 ignored |
| Strengthened first-VM-history regression | Build 0; test 101, failed because the old implementation forgot the completed preparation |
| Final locked release parent scope | 22 PASS, 2 ignored |
| Final explicit actual-helper case | 1 PASS, 0 ignored |
| Final release Clippy, library and test targets | Exit 0, zero compiler warnings in emitted diagnostics |

These runs overlap. They establish 22 ordinary parent entries plus the one separately selected new canonical-helper case; they are not summed as distinct tests across iterations. The older standalone real-helper test remains unexecuted by this checkpoint. The parent output includes three intentionally caught panic detectors; none is a failed test in the final scope.

The new actual case uses canonical `mvm-host-agent` and `mvm-signer-helper` release binaries built from the SAME MVM revision, under a fresh synthetic private home with canonical test key enrollment. It uses the canonical LocalBackend configured for Firecracker, without booting a VM. It confirms backend Arc identity, one-time helper installation, positive actual daemon/signer shutdown, destruction of the original backend, credit retention while an external registrar alias survives, and successful original-supervisor finish after its last Drop. The two reserved worker credits after shutdown are retained ownership credits, not two living processes.

The helper binaries were built before the final Root-only history-flag change; their SDK source is unchanged. Their protected hashes agree before/after the final test. The final consumer test object is rebuilt from the corrected Root source. Source manifests also agree before/after all recorded execution and lint phases.

The normal scope runs three added refusal/retention cases: missing profile with no changes, active/previous/closed VM admission refusal, and actual mutex poisoning that retains the whole managed owner without starting any helper or guest. Its synthetic private profile numbers in the real-process case are functional-fixture terms, not resource-model acceptance or measured RSS.

## Reproduction and retained records

From the full source checkout:

```sh
cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib installed_parent:: \
  -- --test-threads=1
cargo +1.97.1 clippy --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib --tests -- -D clippy::all
```

The separately selected case requires the two protected canonical helper paths and a fresh synthetic `MVM_HOME`; exact commands/environment paths are in the archive. It is explicitly selected with `--ignored --exact`, never counted as an ordinary invocation pass. Both canonical keys and all executable bodies remain excluded from this public packet.

A first control-script attempt selected no metadata using an absolute `.d` pathname filter. That prerequisite is recorded as a later summary, not a fabricated Rust failure. The corrected probe selects the retained normal-library compiler artifact and reproduces the actual E0599. Original test failure records and source/binary hashes remain separate from the successor.

`receipts.tar.gz` is 349,762 bytes, SHA-256 `744e1ce08bf0866b14f0e923fb84eff9036eac57e2fee521f3f188fb2e399349`. All 68 indexed members plus the checksum index verify. Raw compiler objects, binaries and synthetic homes remain on Wick; no payload store or VM disk was copied locally.

This case registers no VM, starts no guest and transfers no object. It adds no CPU/byte/WAN performance result, native backend retirement, FIPS/CUI approval or accepted capacity envelope. Retained telemetry ownership, actual endpoint/VM integration, complete audit/FlowMux/access operation, sustained recovery and the cross-platform WAN release remain open.
