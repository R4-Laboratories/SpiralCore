# Original helper custody during configuration unwind

Contributor MVM [`639cdc36ed1a29a85a37e2606075894776747d54`](https://github.com/Casey-allard/mvm/commit/639cdc36ed1a29a85a37e2606075894776747d54)
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
Final source `639cdc36e` passes the complete serial workspace suite and doctests,
strict all-target workspace Clippy, Linux/BDD gated compilation, the helper
spawn inventory and all three separately invoked original-helper process tests.
The original `sc-helper-config-final-checks` unit ended with status 0. This is
component/source qualification, not complete native MVM or release acceptance.

The preferred `just tests::scoped` command could not run because cargo-nextest is
absent on Wick. Its setup failure was retained; focused `cargo test` supplied the
behavioral RED/GREEN instead. The initial extraction also needed an ordinary
`io::Error` to `anyhow::Error` coercion before that behavioral RED could compile.
Neither setup/compile failure is counted as the observed panic regression.

## Retained workspace failures

The first parallel workspace attempt stopped when an existing builder-streaming
fixture could not read its descendant PID file. That test passed in isolation.
Temporary-directory interference is a hypothesis; the cause is not established.
The serial run is a separate configuration, not proof of parallel reliability.

The first serial run omitted the documented `MVM_GPU_TEST_EXPECT_NATIVE` flag
and failed because Wick now has a native driver. The correctly configured
focused GPU test passed. No driver or training configuration changed.

The configured serial run reached the final tooling suite and found a real
mistake in the new fixture: raw `Command::new` violated MVM's helper environment
inventory. Candidate `639cdc36e` replaces it with the canonical `helper_command`;
production custody code is unchanged from `ae2717f0f`. The final four child tests
pass. The inventory, strict/gated, helper-process and complete serial workspace
checks also pass for that final source. All prior failures remain retained.
The final test environment sets `MVM_GPU_TEST_EXPECT_NATIVE=1` and uses one test
thread. The parallel fixture failure is not diagnosed or counted as repaired.

## Explicit original-helper process results

The final-source **three original-helper fixtures pass** after building the
matching `mvm-mailbox` executable. These are real helpers with synthetic guest
streams and VMM-lifetime processes, not booted MVMs. Each verifies exact 257-byte
readback. The reconnect fixture also collects both predecessor helpers and
refuses current revocation. Signed ordinal settlement retires one contiguous
ordinal only after the verified receipt.

| Component case | Sealed relay A to B | Reply relay B to A | Connection through B ACK | A / B unassisted exit |
|---|---:|---:|---:|---:|
| Lost guest reply | 5,534 bytes per attempt | 1,136 bytes per attempt | 57.697 ms | 207.747 / 10.346 ms |
| Helper reconnect | 5,513 bytes per attempt | 1,139 bytes per attempt | 81.901 ms | 222.935 / 15.343 ms |
| Signed ordinal settlement | 5,550 bytes | 1,145 bytes | Unmeasured | 222.875 / 10.331 ms |

The first two cases record turns 0, 1 and 2. Relay counters and B's guest-reader
counters measure separate I/O surfaces; they are not complete transport or WAN
byte totals. The test command took 27.54 s wall and 32.06 + 2.20 s CPU including
compilation; its maximum RSS was 2,577,612 KiB. The fixtures themselves finished
in 0.95 s. Per-transaction process CPU was not measured. These are recovery
checks, not compression or zstd benchmarks.

[Exact measurements and boundaries](original-process-results.json) ·
[Final verification and retained raw-log hashes](validation-receipt.json).
The receipt lists original per-target summaries, including nested fixture
summaries; those rows must not be added as a unique-test count.

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

All **4,675 tracked files**, totalling **64,851,537 bytes**, match the published
candidate's file manifest on Wick. Manifest SHA-256:
`c5836a6fca7b952bbae0c46130c2b23b0aae06a141847211729d45f7f9f52017`.
The retained checkout Git metadata remains at `0909b7ec`; qualification uses the
verified candidate file overlay, not a clean release checkout or official
publisher build. Raw logs and the complete manifest remain under Wick's existing
qualification directory; only compact reports are kept locally.

The shared build scope is limited to two CPU equivalents and 6 GiB RAM, swap
disabled, with one qualification job at a time. Rust 1.97.1, umask 0022, two Cargo
build jobs and the existing shared target are used. No guest image or raw payload
dataset is duplicated on the Mac.

The four configuration controls send no application object: transfer bytes, fidelity comparison,
transfer latency and WAN results are unmeasured. Build/test command CPU and wall
time are compilation-inclusive and cannot establish networking performance.
