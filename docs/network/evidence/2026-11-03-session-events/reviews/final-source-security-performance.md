# Composed C8 source and registered performance review

Date: 2026-11-03. Scoped source verdict: no remaining blocking security finding
in the three reviewed production files. Performance design is approved within
its registered bounds, subject to exact source/executable binding before launch
and the host-cleanup wording clarification below. No measurements, builds,
remote actions or product edits were performed by this reviewer.

| Production file | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `8540ea581d6b7e323b6502e945031cb187cdf47ead20a26eaa242fa8e0d8969c` |
| `campaign/session.rs` | `8d8f27e50fe0a6dca43c317b26f8a3eeffce5ab2b60f6f060d442b2153010e13` |
| Adapter `receipt.rs` | `f72d2f84a675cd7221dd94fa4a10c7d84331fb7c04a95d7200d782795d07c305` |

## Composed security boundary

The earlier optional-observer approval, [caller review](caller-late-security.md)
and [receiver finding/resolution](receiver-replay-security.md) compose for this
slice. Canonical verification remains authoritative; unknown observations add no
retained state and known mismatch/expiry/conflict errors remain errors. Earlier,
Late and Duplicate events cannot substitute current tracker handles or invent
acceptance levels. The caller retains charged Reply payload until fresh original
deadline checks and completed receipt transmission. Receiver cached Reply lookup
and exact comparison precede candidate admission; replay retains its original
tracker identity and does not rerun object work or allocate another sequence.
The receiver fixes Instant/Unix/owner-monotonic lifetime, bounds events/sends and
fences every incomplete invocation before another read. No borrowed receipt
permit is persisted. Existing canonical wire, signed admission and route authority
are unchanged. Source approval does not replace separately recorded focused,
full-release, platform or actual native execution gates.

Automatic timer retry, reconnect/resume, autonomous idle maintenance, production
retention beyond 2,048 Calls, larger windows, host-wide plaintext/cache ownership
and WAN/platform acceptance remain open. This is not full C8 completion.

## Matched measurement scope and baseline applicability

The host registration contains 22 cells: nine small arm/trial pairs and two large
arm/trial pairs, each before/after in a fresh process. The pinned C7 endpoint
binary `879111c94d8ed8753526758997e497ee4391b8893040b6ef1067068fe96309e5`
is applicable to this unchanged direct-TCP broker_objects harness; the later
duplex relay is not in that execution path. Identify the baseline as C7 endpoint
behavior, not the complete later native source or a failure-recovery benchmark.
The native four cells must use the protected final duplex `62cb6042` host/guest
as baseline, then current C8 host/guest, with the same relay and two-slot peak.
Bind both baseline native binaries and reviewed after commit, full crate manifest
and all executable hashes before running; registration placeholders are not pins.

Keep one active cell, host 512 MiB owned-store/1 GiB sampled RSS ceilings and the
30-minute batch bound. Native stays at two NIC-less guests, one vCPU/1 GiB each,
256 MiB disk per cell and the same batch bound. No capacity or rate increase is
implied. All arms compare the same eight target positions; Combined additionally
pays checkpoint/schema setup. These mixed/repeated targets are not eight unrelated
cold files. Record actual lengths/readbacks and all transmitted framing/envelopes
in both directions; native two-hop physical totals are separate from logical
mailbox totals. Linux experimental64 remains unenforced, and all cells are local.

Host aggregate process CPU is measured once across both roles and setup; per-role
ThreadCPU is separate. Do not sum overlapping campaign ProcessTime intervals as
endpoint CPU. Host verified transfer includes computation/waits and receiver
readback; offline re-verification and cleanup/lifecycle remain separately labeled.
Native endpoint CPU sums timed setup/operation intervals, excluding untimed gaps,
record output and independent final verification. Native lifecycle starts after
initial journal/link admission and includes boot through keyless verification and
teardown. Relay process CPU and nested worker/VMM CPU remain separate; neither
timing instrumented coverage nor estimated energy/send-only time is admissible.

Retain every byte/CPU/time/RSS regression. Three small trials and one fixed large
trial on shared Wick do not isolate cause or establish statistical speedup or
noninferiority. Successful transfer cells measure normal-path cost; synthetic
functional RED/GREEN separately establishes duplicate/late/replay behavior. A
valid output claim is bounded exact delivery plus the observed matched costs,
with each source, arm, size, trial and measurement domain explicit.

## Custody clarification required before execution

The unchanged host broker_objects engine removes its successful owned store
after exact readback and verified public-record emission, before later archive
collection. Clarify that exception in registration rather than claiming every
host store waits for archive/member verification. Native disks remain until
compact receipt archive and member checksums verify. Preserve failed cells and
all losses, delete only successful owned stores, report apparent file bytes
separately from freed allocation, and copy no payload/disk or new target to Mac.

## Prelaunch orchestration review

Reviewed scripts below; no execution, shell mutation or build was performed.
The registration now explicitly distinguishes host in-engine successful cleanup
from native archive-before-disk-cleanup custody, resolving the wording issue.

| Script | SHA-256 |
|---|---|
| `host-pairs.sh` | `6ea752f1275bb095bffe2998b19ecd6a321155f25fcba819aecdab0c6947572a` |
| `native-linux.sh` | `3a82f168f4ce11b192165cb815cadcd791f37c5ceb001edd62c74c928a6d20ad` |

No additional authority/custody blocker was found. Both scripts refuse existing
result paths and verify reviewed source/executable manifests. Host pins its C7
baseline, runs one fresh cell at a time in registered alternating order and
requires successful engine cleanup. Native pins protected duplex host/guest,
kernel and VMM; rejects an interpreter, existing VMMs and an existing shared link;
only removes its matching target/inode/root-owned symlink. Existing Rust resource
configuration is unchanged. Both native versions must peak at two slots/524,288
bytes under identical two-link/two-frame limits, with zero final usage/refusal/
fencing and matching two-hop totals. Native disks remain untouched by this script.

One launch precondition remains: host 22 cells times 120-second cell timeout
can exceed the registered 30-minute batch bound. Enforce an explicit cumulative
30-minute deadline on Wick around the whole host script (not merely an SSH/client
wait deadline), or add an equivalent remaining-batch check before execution.
Four native 300-second cells nominally fit in 20 minutes; retain the registered
outer batch bound as well. GNU timeout TERM is not proof of joined teardown.
Any timeout/nonzero status stops the batch and preserves evidence; inspect only
owned process identities and reconcile teardown before any further cell. Do not
claim successful cleanup or broadly kill other work. Source/binary verification
and final required tests must finish before performance launch.
