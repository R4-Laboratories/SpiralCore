# Independent final evidence review

Scope: the public sustained-broker report, new README summary and their compact
records. Product code is unchanged on the documentation branch. This review
used existing records and source; no build, benchmark or broad audit was run.

Verdict: **APPROVE the reported measurements and limitations**, with one requested
publication clarification: label the README's 62.51/107.27 MB figures as logical
mailbox bytes. Across the two actual mailbox hops the accounted vsock stream
totals are 125,013,918/214,536,774 bytes, respectively. The report already labels
its table as logical; neither quantity includes virtio/workload-exit overhead or
establishes WAN performance.

Checks independently performed:

- The seven native summary rows total **62 publication operations and 44 keyless
  reboot readbacks**. Every row reports successful exact reopening. All five
  two-guest rows report successful keyless recovery. Stage-one physical stream
  accounting equals logical; stage-two accounting equals twice logical. These
  are repeated publication/readback operations, not a claim of 62 distinct objects.
- Native table byte counts, application process CPU, sender session time and
  complete lifecycle match their raw summary values. The small two-guest Combined
  lifecycle loss remains explicit: 1193317 versus 1162816 microseconds for zstd.
  Guest application CPU is separate from relay/worker/partial observed VMM CPU;
  overlapping or nested clock intervals are not added into a host-total claim.
- The five successful local object rows match the reported bytes, CPU, verified
  completion and RSS. Higher Combined memory is retained. The same eight targets,
  two additional Combined setup publications and cache-aware baselines are explicit.
- The original large shared-budget RawMixed failure remains unavailable beyond
  its Full/exit/empty-result evidence. Its later changed-recorder global owner
  snapshot is not represented as per-dispatch attribution or as a successful
  complete raw comparison. Native separate-budget success does not erase it.
- Mac recovery last waiting checkpoint is 301103 ms, with tracker/index entries
  2458 to 0 and three persistent FIFO scopes. The final wait field also includes
  the subsequent resume interval. Two resumed operations have three exact
  comparisons. Lifecycle values match the record; unavailable recovery setup
  timing is acknowledged separately.
- Mac steady8 remains FAIL with 974 complete operations, 1463 comparisons, four
  incomplete admissions, zero mismatches and zero recorded quota refusals. The
  ambiguous Unauthorized and aggregate observations after failure are not
  promoted into a unique OS scheduling cause or a repaired Mac gate.
- Linux summary totals independently agree: 13684 completed operations, 20537
  comparisons and 324300612 framed-envelope bytes, with two explicitly incomplete
  hot-peer requests. Hot workers complete 1638 operations before their recorded
  Sender/Control/Identities refusal. Healthy scheduled peers complete 163/41 each.
  Worst-worker percentiles, sampled FDs/RSS, pre-drop state and post-join releases
  remain distinguished. The short service-retention profile does not qualify
  production default retention or arbitrary churn.
- Final native-platform records total **504 portable passes plus eight Unix
  storage passes = 512** across seven native client rows, with zero failed,
  ignored or filtered selected tests. The Linux execution excerpt supports 47
  release component/example tests and 112 shared conformance tests. These
  overlapping validation scopes are not pooled as independent experiments or
  interpreted as guest/WAN/Windows durable-gateway acceptance.
- Adapter coverage percentages match the declared compiled production line
  denominator: 4483/4828 = 92.85%; changed 167/168 = 99.40%. Whole-repository, branch
  coverage and the partially selected live scope are not implied.

The native 74-member archive/hash verification and seven successful payload-store
cleanup receipts remain root-owned custody evidence. This review checked their
summary/cleanup claims but did not rerun the full archive verification. All
broader platform, aggregate fleet, canonical WAN and certification gates remain
explicitly open. No further numerical or interpretation blocker was identified.
