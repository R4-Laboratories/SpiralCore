# Independent CPU-conscious measurement and infrastructure review

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no blocking findings.**
Scope: the new fixture profile, existing campaign workflow/harness adaptation,
registration, analyzer and analysis method. No builds, new traffic or expanded
audit were performed. This is a prelaunch source/measurement gate, not evidence
of a speedup or of a completed campaign.

## Same-source controls and execution

All three arms use one frozen current `spiralcore-gateway-object` executable.
The pinned `9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b` checkout is a source-only
control: unchanged Whole/prefix/dictionary sources and both lockfiles are compared
before building. It is not a separately timed baseline. Per-session receipts bind
the service hash to the common binary. Builds finish before traffic; source and
binary identities are checked again. No historical timings are pooled.

The workflow retains exact action pins, read-only repository permissions,
same-repository PR-head checkout, no persisted checkout credentials, first-attempt
restriction, bounded job/campaign timeouts and failure artifact retention. No
credentials or new host routing/firewall changes are introduced. Existing owned
namespace topology, private key/config temporary directory, unprivileged service
execution, calibration, capture drainage and cleanup boundaries remain in place.
Actual source/artifact custody and host-route results must be checked after the
run; source review alone does not establish them.

## Fixtures, dose and honest byte tradeoffs

The new explicit registration schema selects exactly Combined, CombinedCpu and
Whole. Three arm-order rotations, two link profiles and two sequential size
groups give 18 sessions. Eight sends per group give 288 deliveries; both Combined
policies pay 24 checkpoint and 24 schema operations, for 336 total operations and
2,289,396,384 requested bytes. Twelve unique files are reopened and compared per
session. Existing six-send fixtures and the original 48-byte schema remain intact.

The two added controls are deterministic, registered before timing, and never
train a new schema. Their 256-byte records use a ten-digit value plus 222 hex
characters; the exact tier lengths and bounded no-LF tails reconcile. The fit
control retains the learned key. Drift retains one useful fitted record, then
changes to an equal-length untrained key and independent values. Neither test nor
campaign gate demands that SchemaZstd win; positive fit only keeps it eligible.

Before/after blanket byte equality is correctly removed for this policy. Only
the eight generated/opaque-edit/exact-repeat mechanism rows per paired session
retain mandatory identity/mode/path/framing equality. All other operation byte
differences remain observable. Analyzer opportunity rows join the same profile,
trial, group and label, check object identity/length where present, and compute
signed CPU-conscious-minus-exhaustive logical bytes through checked i128/i64
arithmetic. Positive, negative and zero differences are preserved. Group paid
cumulative and full-session opportunity costs are also retained.

Policy skips require the exact separate record shape, zero useful count and no
invented cost/bound or simultaneous schema evaluated/proven entry. Exhaustive,
fit, drift and repeat cases cannot claim policy skips in the analyzer. At least
one skip must actually occur. Existing per-transfer receiver CPU joins, actual
bidirectional channel deltas, both setup charges, publication/readback checks and
three-distinct-trial medians remain authoritative. Aggregate capture bytes are
not assigned to individual objects, and nested CPU is not added to process CPU.

## Retained checks

Inspected fixture GREEN 6/6, analyzer GREEN 12/12, both targeted strict-Clippy
exits zero, ShellCheck/static workflow exits zero and pure infrastructure result:
12 general negatives, four binding negatives, four capture negatives, five parity
negatives and the invariant-only positive/negative. The latter allows unrelated
byte differences while rejecting lost mandatory mechanism savings. No reviewer
rerun was needed.

All four historical analyses were reproduced byte-identically by the author,
with receipts retained: Runtime `05482731...66801`, Combined `edaf981c...36e17`,
candidate CPU `69b6efd3...efef2`, and unrelated CPU `a97182d4...e9f02`. Distinct
legacy registration schemas/pins retain their old interpretation. The newly
registered campaign remains NOT_RUN at this review and makes no Internet-WAN,
energy, transparent TUN, MVM or compliance qualification claim.

## Reviewed hashes

| File | SHA-256 |
|---|---|
| `.github/workflows/spiralcore-candidate-cpu.yml` | `8e37a6a3711adf10e43150aa7bbe616eaa82ada07c36ec156623250f2a8caaec` |
| `crates/spiralcore-gateway/examples/object_fixtures.rs` | `e18000b7d3e96c7eb5548d9873023dd04cee4a0c1ecc9d9e5ebdc387adbdf7f3` |
| `tools/gateway-network/runtime-object-netns.sh` | `9260eea950de8b258d97f45aa59e87a31956725e164ec1ff65189893a6a29657` |
| `tools/gateway-network/runtime-object-infra-check.sh` | `a385119fd8dd8f37a95e943fd95bd3254e867f744be2a28496d5bce578d98975` |
| `tools/gateway-network/analyze-runtime-objects.rs` | `ab94c4661307846c23e06d5e2d93e599260578a74a4e3f8a2c8a4c4b3124eae0` |
| `network-registration.json` | `3f3094270866741de37229d6cac09deef3acdb76437e8d1f9e9d4665384cf667` |
| `analysis-method.md` | `8ed8f160b543798d8b09005ce98bd932d5651d6f39c2edb465419771b2f68b47` |

The last two paths are relative to this evidence directory. Registration was
reviewed with `launch_ready:false`; a separately recorded authorized launch-state
transition does not change the reviewed dose or measurement design.
