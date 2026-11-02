# Independent diagnostic code and arithmetic review

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no blocking findings.**

Scope: the frozen context-only helper, final `schema_context_profile.rs`, their
two retained doses, design/plan, and final result report. This was a bounded
cross-author Rust/code/security and arithmetic review. The reviewer ran no Cargo
builds, new timings or expanded audit. Production transfer sources and its
manifest/lockfile have no diff from baseline
`0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb`.

## Source and measurement boundaries

The original diagnostic uses the existing public Whole encoder for both target
and canonical SCVAL input, comparing separate contexts with one reused context.
Every pair checks full schema-frame equality against the original standalone
schema encoder and both arm outputs against each other, then independently
decodes Whole and schema frames to exact target bytes. SCVAL input is capped to
the existing 16 MiB Whole domain; this is not a production schema-domain change.

The final helper uses raw CCtx instances with all seven parameters identical to
unchanged `new_encoder`: level 3, window 25, LDM enabled, zero extra workers,
content size/checksum enabled, and dictionary ID disabled. It provides full
`compress_bound` destination capacity and enforces the existing context-size
allowance after compression. Each result must equal independent original Whole
and SchemaZstd frames and decode exactly before its successful observation is
emitted. No small-buffer error is used as a candidate-exclusion certificate.
No unsafe Rust, networking, secret material or production trust-boundary change
is introduced. Outputs use exclusive creation; this remains a local diagnostic
for trusted public fixture manifests, not a hostile-input service.

Per case, three fixture trials and six repetitions alternate the timed arm order
equally. Both fresh-context first use and all five later repetitions are retained.
Constructors and canonical intermediate preparation have separate timings. Input
loading, hashes, equality checks and decoding are outside compression windows;
pair CPU/wall contains the nested Whole/schema windows and must not be added to
them. JSON/report writes occur outside measured arm windows. Both arms coexist,
so per-arm context workspace is not process RSS. First use is not a cold-host or
cold-filesystem measurement.

The allocation follow-up changes context sharing **and** output-vector retention
together. Fresh output replacement/deallocation versus reused-vector clearing is
inside its encode timers; both have full output capacity. The intermediate is
prepared equally, so there is no measured intermediate-allocation saving. Unlike
the first dose's public wrappers, this dose places frame validation outside both
raw compression timers. Cross-dose subtraction cannot isolate an allocation
effect. The report expressly preserves these limitations.

## Independent arithmetic

Recomputed medians and matched ratios from the raw JSONL with independent
jq/JavaScript arithmetic, rather than trusting the helper's summaries. **Every
summary field matches** in both doses. Each has 144 unique pairs, 288 arm
observations, 24 inputs, 24 context-setup rows, 24 first-use pairs and exactly 72
shared-first pairs. All parity/decode flags are true. All 24 input, artifact,
canonical intermediate and reference-schema identities agree between doses.
The maximum intermediate is 15,142,482 bytes.

Repeated large opaque results (15 matched pairs per row):

| Diagnostic | Separate pair CPU median | Reused pair CPU median | Paired CPU reduction | Paired elapsed reduction |
|---|---:|---:|---:|---:|
| Context only | 51.042 ms | 51.105 ms | 0.368324% | 0.832724% |
| Context + output vectors | 49.238 ms | 49.307 ms | 0.528201% | 0.533157% |

Independent medians may rise while the median of within-pair ratios falls; the
report labels the distinct calculations correctly. Repeated schema-stage CPU
reductions are 0.680419% and 0.897550%, respectively. Small opaque pair reductions
are 1.477200% and 3.000300%. Retained losses include the context-only large fitting
pair at +0.011143% CPU/+0.312885% elapsed, and joint-variant large generated,
residual and fitting CPU at +0.454545%, +0.074211% and +0.224509%.

These observations support publishing the negative and leaving production
unchanged. They do not establish a material removal of unrelated-object CPU
overhead, an isolated allocation benefit, statistical significance, WAN speed,
durable-transfer speed, MVM qualification, physical energy or a production RSS
improvement. The report retains the preceding campaign's unrelated-object loss
to Whole-zstd and explicitly attributes prior mixed-session savings to prior work.

## Reviewed identities

| Item | SHA-256 |
|---|---|
| Original context helper / `context-source.rs.txt` | `64a06adf9e56483674cd6879dcedd1f29bc74cce88a6629b83282847ddb7011f` |
| Original context executable | `03afd5455414fe0e6469c826921000fb82c108d00491c3916dc707445d0ae8ec` |
| Final allocation helper | `75f28ab696ed51560d1f6db963d37477ca403ea8046eacc5d57ebb34cf35d314` |
| Allocation executable | `04c9cb66f3df69f943682580761034aa4b579d7b8be22499d91ad306bac6ffbd` |
| `context-profile.jsonl` | `edcd690f8233ef3df7500a2bb3095b143e8447803a98cf56935b763eb2e7ee0c` |
| `context-profile.summary.json` | `c6ec43c70c6be717bb540d06535b28893ee8bb204a5b13c3a31e063971d4e084` |
| `allocation-profile.jsonl` | `b4c8cad73b345e39c7ef9da230e6fc46227d5ea77355a214f0dc6b7564ff1895` |
| `allocation-profile.summary.json` | `23e4ee46bb31a43cc8f4834a97e950f4ebb580789d6aad2c955a545a608b3861` |
| `docs/network/schema-context-cpu-results-2026-11-02.md` | `15f1b2ca490bbfd0e1e7dbfc2821a0a1784cfe7f44e927f0c0b576eaf5be4b69` |

Executable/helper identities distinguish each uncommitted diagnostic addition
from the unchanged production baseline recorded in registration. Both run exits
are zero. The recorded focused strict-Clippy exit is zero; the reviewer did not
rerun it. Final report tables, ratios, counts and stated limits match this review;
no numeric correction is required.
