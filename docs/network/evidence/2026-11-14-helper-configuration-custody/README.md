# Original helper custody during configuration unwind

Contributor MVM [`ae2717f0f5c95fa5c6c5cf1f3c4ae98852f392ca`](https://github.com/Casey-allard/mvm/commit/ae2717f0f5c95fa5c6c5cf1f3c4ae98852f392ca)
repairs a post-spawn configuration unwind window in the preview installed journal
path. The actual child, launch marker and restart custody stay outside the
configuration callback. A panic becomes fixed failed-readiness and returns the
same original child through the existing A/B owner. The SDK stores that owner
before checking readiness, so its existing stop and collection path applies.
No wire format, codec, signing authority or guest interface changes.

At the compiled regression source `59665aa08`, successful configuration and
ordinary refusal passed, but both panic cases failed: **2 passed, 2 failed**.
The fixtures collected their actual children before reporting failure. The fixed
source passes **4/4** controls. They verify the same original child identity,
actual exit status 17, original-marker exclusion and explicit clearance only
after child collection. These are host component tests, with no booted MVM.

The existing journal suite passed **39 tests**, including these four, with three
actual-helper fixtures ignored because they require a separately compiled helper.
Strict all-target workspace Clippy and Linux/BDD gated compilation passed.
Workspace and explicit original-helper results are recorded separately once the
owned qualification jobs finish. No completed release claim follows from a live
job or a compilation result.

The preferred `just tests::scoped` command could not run because cargo-nextest is
absent on Wick. Its setup failure was retained; focused `cargo test` supplied the
behavioral RED/GREEN instead. The initial extraction also needed an ordinary
`io::Error` to `anyhow::Error` coercion before that behavioral RED could compile.
Neither setup/compile failure is counted as the observed panic regression.

## Exceptional state and limits

The child and marker remain in their original owners. A caught panic is neither
Ready nor NoExec, and cannot establish VM retirement, release parent credit or
clear a durable marker. Ordinary configuration errors keep their existing
returned diagnostics. Partially configured restart state is never advertised as
ready. The SDK's outer uncertainty fence remains for failures outside this
configuration boundary.

The exceptional panic payload is deliberately retained without running an
arbitrary destructor that might panic again and lose custody. The dedicated
payload-destructor test verifies this disposition. This is at most one retained
payload per failed construction, not a general memory-reclamation or sustained
fault-loop capacity guarantee. The process's existing panic hook can already
print a payload before it is caught: this repair adds no panic-payload logging or
returned diagnostics and does not install a global hook.

Process abort/OOM, later constructor unwinds, detached/restart service recovery,
retained-state expiry, supported fleet capacity, signed-runtime guest execution
and canonical WAN acceptance remain separate gates. The compatible guest runtime
still requires Ari's canonical publication. No signature check is bypassed.

## Source and resource boundary

All **4,675 tracked files**, totalling **64,851,526 bytes**, match the published
candidate's file manifest on Wick. Manifest SHA-256:
`0bdaafe9e1e631033c92ee4660cee4a0a6c13fce5089ccbc180718bb2db4f0df`.
The retained checkout Git metadata remains at `0909b7ec`; qualification uses the
verified candidate file overlay, not a clean release checkout or official
publisher build. Raw logs and the complete manifest remain under Wick's existing
qualification directory; only compact reports are kept locally.

The shared build scope is limited to two CPU equivalents and 6 GiB RAM, swap
disabled, with one qualification job at a time. Rust 1.97.1, umask 0022, two Cargo
build jobs and the existing shared target are used. No guest image or raw payload
dataset is duplicated on the Mac.

These controls send no application object: transfer bytes, fidelity comparison,
transfer latency and WAN results are unmeasured. Build/test command CPU and wall
time are compilation-inclusive and cannot establish networking performance.
