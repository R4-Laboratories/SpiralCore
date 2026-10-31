# SC-OPT-016 bounded-selection measurement contract

Registered under design/plan commit
`e90d9eadd46496c48fce4a2490b2449e9d2fe0ab`. Measurements await the implementation
commit, frozen executables, complete qualification and parent release. Reuse the
existing warm-object driver, receiver, timing, verification and summary code.

## Matched builds and roles

Use one common current harness with three explicit roles:
`baseline-selector`, `candidate-selector`, and `whole`. Baseline is a labelled
production overlay, not the complete historical checkout. Replace exactly these
files with their blobs from `4234591e25b4ca87db6a4670bdf407f48a581575`:

| File | Original SHA-256 |
|---|---|
| src/zstd_whole.rs | 0e0293bdf3ecfa0916bb9c0e611e1e968b4cb2f209cd150e461990b4551943be |
| src/schema_selection.rs | 884955623eed88c2665874fa0f921ecf713d34d053b55410b893c89d8013ebb7 |

Build this baseline with `SPIRALCORE_DISABLE_ZSTD_PROBE=1`. Restore the current
files for the candidate and build with the exact bundled-source guard enabled.
Both use the same Rust version, target, profile, features, Cargo manifest/lock,
build script, guard source and measurement code. No receiver or wire format
changes are part of this comparison. Root retains overlay/restore/source hashes
and compiler/dependency evidence. Use the existing target directory, explicit
target triple and `CARGO_INCREMENTAL=0`, with the 4 GiB disk floor.

The candidate executable generates canonical plans containing both source-pin
sets. Baseline pins differ only at the two registered files. Candidate and
baseline identities must report `cfg(spiralcore_bundled_zstd_probe)` true and
false respectively; the common harness does not call a new symbol absent from
the original source. Executable/platform/feature identities remain separate
from platform-independent source pins. Worker role identity is checked against
its own declared source profile. Canonical fixture, dose, order and namespace
checks do not require the baseline to masquerade as candidate source.

## Main experiment

Keep the SC-WAN-005/SC-OPT-015 deterministic xorshift64-v1 bases and exact object
identities at 1,048,576 and 16,777,216 bytes. Targets XOR 32 midpoint bytes with
0xa5. Each arm pays an actual Raw base seed in its own fresh authorized namespace,
then sends one absent target with `warm_probe=false`. DAG warming is disabled;
receiver codec-cache policy and quotas match across arms. Whole retains the
same level-3/window-25/LDM/checksum/content-size settings and Raw fallback.

Map the existing reproducibly shuffled six-permutation schedule to the three
new role names. Each permutation appears five times per size across 30 WAN
trials. WAN uses six five-trial shards: 180 sessions, 180 seeds, 180 updates,
360 transfers. Local acceptance uses the first ten trials in two five-trial
stores: 60 sessions, 120 transfers. Each store retains 510 MiB object payload;
its conservative 566.234375 MiB charge remains below 768 MiB and the store quota.
Stop and exactly verify each store before cleaning that owned cache.

Every selector session actually generates the frozen 512-record training corpus,
fits/checks the 169-byte artifact and constructs its context inside setup timing.
Thus both selector arms pay setup: 40 actual fits locally and 120 in the WAN run.
Whole pays its context construction; all arms pay their real base seed. This is
not a claim that the production constructor fits a schema, and no cross-session
fit amortization is substituted for measured setup.

Time each target from API preparation/encoding start through validated durable
ACK. Both selectors must preserve SparseV1, exactly 600 HTTP-body bytes, original
length and full output bytes. Clone their small winning request frames during
exchange, charging that common overhead to both selector arms; hash the complete
normalized request frames afterward, zeroing only transfer IDs. Require exact
normalized winner-frame hash/count parity. Receiver acknowledgments remain
validated by the client and final independently reopened byte comparisons.

Baseline Whole is an exact completed candidate at 1 MiB. Candidate Whole may be
an actual first-block rejection certificate; it is a lower bound, not a full
encoded size. Require its bound to exceed the 600-byte incumbent under tie rules
and not exceed the baseline's actual completed cost. The registered random
first block is expected to yield a 131,879-byte complete-body lower bound, but
all bounds remain explicitly typed. Inconclusive probes retain exact full-encode
receipts. At 16 MiB both paths already use the static 896-byte bound, providing
an unchanged-path control. All decisions are joined to the same target IDs,
actual Whole control, winning mode and body cost. Cold schema cost remains paid
in selection logic; these main fixtures must not introduce an artifact seed.

Local advancement requires all exactness/receipt checks and **median of the ten
paired 1 MiB candidate/baseline sender-CPU ratios <= 0.80**. Preserve every ratio
and negative observation. Do not rerun or retune the threshold. A failed CPU gate
is retained as a negative result and does not advance to WAN. This gate does not
promise a WAN timing result; actual completion and CPU are reported separately.

## Compressible/inconclusive control

Run ten interleaved old/new prepare-only pairs, with a separately fixed shuffled
five-AB/five-BA order. Use a zero-filled 1 MiB base and the same 32-byte midpoint
XOR edit. This supplies a roughly 600-byte Sparse incumbent while allowing a
smaller Whole winner, exercising the inconclusive first-block path plus original
full compression. Require an actual completed Whole winner, identical exact
selection receipts, and complete normalized winning request-frame parity.

Every sample records the same actual fit/context setup and Raw seed. The primary
control timer covers only `prepare_warm`. Consume the returned plan afterward
through the existing serialized in-process Channel, then reopen and compare
both objects exactly. Frame capture/hashing, sending, receiver work and reopened
verification are outside that preparation timer. Their combined in-process CPU
scope is recorded without adding nested receiver CPU again. The control is an
encoding-cost measurement, not a WAN or complete-transfer speed claim. Retain
all twenty samples and forty exact objects, including any fallback CPU loss.

## Metrics, failures and commands

Main sender-process CPU and receiver-handler CPU are disjoint and added once.
Exchange elapsed/CPU are nested scopes, not pure wire time. Report update and
actual setup-plus-seed-plus-update lifecycles separately, with median/p95, paired
ratios and wins. Body counts include request and response bodies but exclude
HTTP/TLS/TCP/IP, retransmissions and extra relay hops. Relay CPU and energy are
not measured. No builds, tests, profiling or uploads overlap timing.

Output creation is exclusive. Preserve partial workers, stderr/stdout, failures,
original raw rows, source/executable pins and exact verification. No warmup is
discarded. Historical warm-* and warmselect-* artifacts remain frozen-source
results; rebuilding those commands may now expose typed dynamic pruning rather
than their old exact Whole evaluation. No bound is relabelled as an exact cost.

```text
schema_fitting bounded-identity
schema_fitting bounded-plan RUN SHARD NEW_PLAN
schema_fitting bounded-paired PLAN BASE_EXE CAND_EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting bounded-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting bounded-summary PLAN0 RAW0 VERIFY0 ... PLAN5 RAW5 VERIFY5 NEW_JSON
schema_fitting bounded-local BASE_EXE CAND_EXE NEW_DIRECTORY RUN
schema_fitting bounded-local-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 NEW_JSON
schema_fitting bounded-control BASE_EXE CAND_EXE NEW_DIRECTORY
```

Set `SPIRALCORE_BOUNDED_SOURCE` to the committed common harness/candidate source.
Authentication remains `SPIRALCORE_WAN_TOKEN`, never printed. Verification also
requires `SPIRALCORE_BOUNDED_OBSERVATIONS` and
`SPIRALCORE_BOUNDED_RECEIVER_SHA256`. Worker commands are bounded implementation
helpers; parent owns the real WAN workflow and launch after local acceptance.
