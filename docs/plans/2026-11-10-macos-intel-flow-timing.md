# Native Intel macOS FlowMux deadline diagnostic

This is a bounded diagnostic continuation of SpiralCore [#44](https://github.com/R4-Laboratories/SpiralCore/issues/44). It preserves the source and failed gate rather than treating the other six native-role passes as completion.

## Observed failure

Source `661c74e7b223736e440e1e41c3cd70603d050d80`, native component run [42922116102](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42922116102), Intel macOS job [130199439671](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42922116102/job/130199439671), failed the sustained test with timing disabled. The runner reported macOS 26.6.1, build 25G76, image `20260824.0517.1`, target `x86_64-apple-darwin`.

The diagnostic snapshot recorded wall `59,840,499 us`, process CPU to snapshot `35,786,480 us`, write/ACK counts `[1791, 1790]` and exact-read counts `[1791, 1791]`. Left TLS sent/received were `2,359,707 / 2,359,405` bytes; the opposite side reported matching traffic. The failure was `block1790: Timeout`, followed by `block1791: Closed` on the opposite writer. Both original scope checks returned `Ok(())`. The test harness reported 60.17 seconds, which includes additional unwinding after the snapshot.

The two retention snapshots remained bounded at 64 / 63 publications and 1,051,712 / 1,035,279 cache bytes, with 1,727 / 1,728 evictions. The unequal ACK/read counts describe distinct publication, response and application-read stages; they do not establish a byte mismatch. The complete 4,096-block gate failed.

## What this establishes

An existing two-second object-operation deadline failed before the original 240-second overall gate. This execution did not enable phase timing, so it does not identify whether the stalled operation was waiting for Session/TX ownership, TLS/socket readiness, receiver storage/commit, or runner scheduling. Previous Windows phase timings do not establish the cause on Intel macOS. No product change follows from this row alone.

## Next measurement

Add a separate `macos-timing` job to the existing timing workflow, leaving the Windows 2025 job and its artifact identity unchanged. Use native `macos-26-intel`, assert macOS/X64 plus kernel architecture `x86_64` and product version 26, and install/check `1.97.1-x86_64-apple-darwin`. Record actual source/tree, lock hash, image, OS/kernel, feature state and workload bounds. Use a distinct `flowmux-macos26-intel-timing-<attempt>` artifact, always uploaded with diagnostic checksums.

Execute only the existing release `ordered_stream` target with `witness-timing` and `--show-output`. Preserve 4,096 exact blocks per direction, the two-second operation deadlines, original authority and 240-second overall gate. No timer-resolution request, deadline increase, reduced workload or product Rust change is part of this measurement. CPU and nested elapsed spans remain separate; overlapping counters must not be summed as total runtime.

Compare nonzero Session/TX condition-variable waits and outcomes, per-frame sender waits and receiver handlers, readiness polls/parks, progress, traffic and retention on the actual macOS execution. A pass records one bounded native component result; it neither explains the preserved failure nor establishes WAN/MVM qualification. A failure must retain its original error and timing report before selecting one source-backed repair.

## Local validation

Wick actionlint 1.7.12 executed with `-shellcheck=` against the candidate workflow and returned exit 0. `git diff --check` also passed. Native execution is pending review and publication; no diagnostic native result is claimed by this plan.
