# Ordered Commit observer and private fixture handoff

SpiralCore source `757b41913f1b2e46ddf18392551fcf27020432ae` adds an opt-in observer inside the existing ordered application Commit and the operator-requested private fixture handoff. The default transfer wire, deadlines and storage work remain unchanged. This report records the semantic before/after controls and the later complete workloads; it does not diagnose or claim to fix the earlier intermittent Windows timeout.

The hosted candidate used merge `01fd1ae0d5f9d854c11e07d1df5ea3a8c4603e66`, whose tree `2a2dc2dda5f9f3f5d0fbc71292cef54662689811` equals the source head's tree. [Windows run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43516951657) · [Linux run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43516951478) · [Verified artifact identities and original result lines](commit-stage-observer.json).

## Controls before and after implementation

The unchanged Windows control at original source `0669c8f4` failed once on both Server 2022 and Server 2025: it observed `("endpoint_handle", 3, 2024)` where `("commit_lookup", 3, 2024)` was required. Each run had zero passing tests, one failure and 32 filtered tests. [Original RED](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43513736637).

On the successor, the same control passed **1/1 with 32 filtered** on both platforms. The enabled observer keeps the original frame kind/ordinal/deadline and records the actual lookup, quota scan, staging, sync, publication, verification and readback stage through the existing sample word. It adds no diagnostic worker or protocol queue. Default builds execute the original storage expressions without observer work.

The separate original handoff baseline passed one control and failed four, because it did not publish/retain the requested ready state. [Original handoff RED](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43513736548). The repaired five controls passed **5/5**, both hosted and on Wick. The genuine native export and its private ownership transfer then passed separately before the [installed guest run](native-signed-fixture.md). A ready filename alone is never authority; consumers require the original export process's successful exit, original directory identity and archive digest.

## Complete Windows workloads

| Native process workload | Lifecycle wall | Both endpoints' process CPU | Exact application bytes per direction |
| --- | ---: | ---: | ---: |
| [Server 2025, stall observer](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43516951657/job/132201225326) | 161.331402 s | 61.265625 s | 67,108,864 |
| [Server 2022, stall observer](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43516951657/job/132201225314) | 133.023343 s | 37.312500 s | 67,108,864 |
| [Server 2025, timing](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43516951657/job/132201225341) | 213.574448 s | 83.687500 s | 67,108,864 |

All three completed 4,096 16-KiB blocks in each direction under the existing **2-second operation** and **240-second workload** limits. TLS counts were 5,395,050 B left-to-right and 5,395,015 B right-to-left; each endpoint retained 64 publications. The two stall runs reported **zero observed and zero retained qualifying stalls** at the existing 1.25-second sampling threshold. There is therefore no actual stalled ordinal or inner stage to assign to the earlier failure, and no measured optimization benefit to attribute to this observer.

Three Windows ZIP digests and all 17 manifest-listed member checksums were independently verified, with the executable's before/after hash unchanged for each original invocation and exit status zero. The Linux artifact ZIP was also verified; its individual member hashes were independently computed because that artifact contains no producer-provided `SHA256SUMS`. Linux passed its metadata control 1/1, producer controls 5/5, handoff controls 5/5, strict fixture Clippy and host contract job. Its manually enabled genuine-export job was skipped; the separately executed Wick export and guest witness are recorded separately.

These native Windows process tests exercise both endpoints on one hosted machine. They are not a Windows MVM-host claim, cross-platform peer pair or WAN result. The earlier d725 deadline failure remains retained negative evidence; passing runs with no stall samples do not establish its root cause.
