# SC-MVM-SCALE-002 evidence

Public interpretation: [sustained broker results](../../sustained-broker-results-2026-11-02.md).
Plan: [implementation](../../../plans/2026-11-02-sustained-broker.md),
[design](../../../specs/2026-11-02-sustained-broker-design.md).

The five registered Linux logical-endpoint cells are complete and pass their
declared gates, with two explicit hot-peer incomplete admissions. This packet
preserves each original measurement source rather than labeling later source
edits as the executable that produced earlier timings.

| Scope | Source | Evidence |
|---|---|---|
| Initial Mac recovery and logical endpoints | `d53cb335ee3884644bfe1a26296923c1288fa540` | [registration](registration.json), [identity](run-identity.json), [commands](commands.json), [source hashes](measurement-source.sha256), [recovery](recovery.jsonl), [2-endpoint run](steady-2.jsonl), [8-endpoint failure](steady-8.jsonl), [deferred cells](mac-campaign-status.json) |
| Linux logical endpoints | `d94020382a1c838a0794af14989d687985c26b69`; adapter/probe source hashes identical to the Mac source | [registration](linux-registration.json), [normal binary identity](linux-build-identity.json), [summary](linux-summary.json), [original rows/custody hashes](linux/) |
| Matched object pilots | `03f05ab7bf830dc82531c7c20e0f847b35a23211` | [registration](objects-registration.json), [original records](objects/), [identity](objects/identity.txt), [runner](objects/runner.jsonl) |
| Failed-object recorder follow-up | `d94020382a1c838a0794af14989d687985c26b69` | [preregistration](objects-followup-registration.json), [failed row](objects/followup-d940203-16777216-raw-mixed-trial0.jsonl), [identity](objects/followup-identity.txt) |
| Native Linux seven-cell pilot | Product source `d940203`; runner/registration `6257930` | [registration](native-registration.json), [runner](native-linux.sh), [summary](native/summary.jsonl), [original compressed receipts](native/native-originals.tar.gz), [member hashes](native/native-public-files.sha256), [owned payload cleanup](native/cleanup.jsonl) |
| Coverage | `d940203` | [scope, actual tests and source/line identities](qualification/coverage/README.md) |
| Initial seven-platform CI | `d53cb335`, executed merge `18e7a6c42c73ed1ec7ad71731240710f5ed0621a` | [original ZIPs and verified manifest](qualification/platforms-initial/README.md); this precedes RawMixed |
| Final source CI | Head `6257930015c68d663958e208f42143d045aa1507`, executed merge `3866c1eb3c559835a750e93fadc8620552d3f9f7`, identical tree `c5cc520b63d74f4b4dbd2f4b6eb51b920904d6f6` | [original ZIPs, source and actual test counts](qualification/platforms-final/README.md) |

The native archive has **74 original members**, no cache payload or VM disk, and
SHA-256 `f9baeb667a5c7c075ad3b70bd9345eda03c578eaa918d3a735d409945851bad4`.
Root independently checked every member against its source manifest and rejected
absolute/traversal paths. All seven native cells, exact reopened publications,
two-way counters and applicable keyless recovery receipts passed before deleting
the seven owned successful payload stores. Removed apparent bytes are not a
measurement of globally reclaimed disk space.

## Accounting and failure boundaries

- Byte totals are actual canonical encrypted envelopes and framing in both
  directions, including setup. TCP/IP, retransmissions and virtio overhead are
  excluded unless the original record explicitly includes them.
- A process-clock interval includes its process's recorder work. Logical broker
  CPU includes repeated state snapshots; it is not isolated codec CPU. Object
  process CPU is measured once, with role thread CPU separately retained.
- All performed payload comparisons must match. Incomplete admissions, an empty
  failed recorder file and unavailable full-transfer counters remain explicit;
  no partial publication file count is promoted into full fidelity success.
- Mac one-second deadline failure, local shared-budget large raw failure and
  native separate-budget success are different outcomes and retained together.
- Local, native, initial CI and future WAN evidence are not pooled as the same
  experiment. The native pilot does not exercise fleet-wide aggregate ownership.

## Local storage

Use the shared Cargo target and compact records. Synthetic datasets are generated
in bounded memory or temporary owned stores. No new payload collection or VM disk
was copied to the Mac. The failed object stores remain on Wick pending their
separate verification; only successful verified native stores were removed.
User sessions, `.serena/`, historical evidence and other labs are preserved.
