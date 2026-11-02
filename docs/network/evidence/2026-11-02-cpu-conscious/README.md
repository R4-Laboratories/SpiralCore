# CPU-conscious selector: implementation and paired transfer evidence

The opt-in policy is implemented and measured. Exhaustive remains default.
All 288 logical deliveries reopen exactly; all 120 paired policy/exhaustive
operation byte differences are zero in this dose. Large unrelated sender CPU
improves, but the full shaped session does not become faster than Exhaustive.
See the [complete result and retained losses](../../cpu-conscious-results-2026-11-02.md).

## Source and execution

- Measured source: `30792f17d0a596c4344e4780ebd4b8f455e3bedb`.
- Implementation PR36 merged as `54d3cb843211c38f93174c0d04d04832d2382ad0`.
- Run: [41465450384](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41465450384), attempt 1, success.
- Exact execution-workflow commit: `d0db5fc1a13103f5720da8444424e6f38ff51f15`.
  Its workflow blob matches the reviewed source; the GitHub display title alone
  is not used as source or measurement evidence.
- All arms use the same service executable, SHA-256
  `89946f9bbbe80f4924e5dd6bd12ccede07f986c9597042ef7b5b1b381997e329`.
- Source-only control pin: `9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b`.
  Whole settings, encoder/probe sources and locks are identical. No historical
  binary or timing is used as an arm.

The registration fixes 18 sessions,336 operations,288 logical deliveries,
2,289,396,384 input bytes, two sizes and three paired trial rotations. Each
persistent receiver gets eight objects per size. Both Combined policies pay
checkpoint/schema publication. The original schema is retained, with fixed
fitting and independent mixed-drift controls. The latter contains one useful
template record and therefore retains schema evaluation.

## Readable receipts

- `network-registration.json`, `analysis-method.md`: pre-run dose, inputs,
  policy contract, timing boundaries and comparison rules.
- `core-checks.md`, `local-verification.md`, `infra-readiness.json` and their
  referenced outputs: RED/GREEN, exact encoding/frame parity, actual avoided
  compression, state/error preservation, gateway tests and historical replay.
- `review-core-gateway-independent.md`, `review-infra-independent.md`: narrow
  independent source/security and infrastructure/measurement review. The final
  arithmetic review is `review-analysis-independent.md`.
- `hosted-run.json`, `pr-checks.json`, `implementation-merge.json`: execution and
  delivery receipts, separate from scientific claims.
- `analysis.json`: all operation, paid cumulative and full-session rows,
  medians, matched ratios, signed byte opportunities, skips and proven pruning.
- `analysis-command.*`: source-bound analyzer invocation and successful exit.
- `verified-manifest.txt`, `verified-candidate-source.txt`,
  `verified-control-source.txt`, `verified-binaries.txt`: all 913 artifact entries,
  169 source entries,157 control entries and three binaries verified.

Validation output is retained as user-requested experimental evidence, including
expected REDs, the corrected test-fixture assumption and formatting correction.
Original trailing whitespace/newlines are not rewritten to satisfy source-style
checks. No unavailable `/quality-gate` command is claimed: the recorded independent
code/security reviews supplied that gate. No timing observation was discarded.

## Full archive and compact reproduction

The original GitHub artifact is retained in the existing private Hugging Face
dataset, including captures and executed binaries:

- GitHub artifact ID: `13153845111`.
- Dataset: `caseyallard/spiralcore-experiment-artifacts`.
- Path: `2026-11-02/cpu-conscious-41465450384.zip`.
- Immutable revision: `ba462609bc705928c91260090a8cc30cef377743`.
- Size: 37,840,360 bytes.
- ZIP SHA-256: `bdb1242f93d3f1acf4eb0335f3a5e808ceb6cf64d6902bf1643eeaca776d5e68`.

`artifact-storage.json` binds source, workflow and custody. A forced download to a
fresh directory from that immutable revision, with Xet disabled, matched size,
SHA-256 and every ZIP byte. All original manifest entries had verified before
archival. The archive is private; readers need access to retrieve full captures.

Git's `compact-receipts.tar.gz` contains 893 original public receipt files,
excluding binaries and PCAP files. It is 342,628 bytes, SHA-256
`a57fb382b8f7bddea342495e963faba4b1a69b1d83df8a6e9b9cc81f7612b101`.
Extract into a fresh owned directory, then run:

```sh
shasum -a 256 -c compact-files.sha256
cargo +1.97.1 run --release --locked \
  --manifest-path /absolute/SpiralCore/tools/gateway-network/runtime-analysis/Cargo.toml \
  --bin analyze_runtime_objects -- "$PWD/candidate-evidence/campaign" > analysis.json
```

Use the measured source or the results revision retaining the same analyzer.
Explicitly build through Cargo; do not rely on an unrelated cached executable.
Fresh extraction verified all 893 hashes and reproduced byte-identical analysis,
SHA-256 `42797d50a77329c0794e7e285bbdf7ec282eb4be6c06ae33ae08427bd5d5718a`.
The `compact-reopened-verification.txt` and `compact-analysis-command.txt`
receipts record this check. The compact manifest is authoritative for this
subset; the retained original artifact manifest also references excluded files.

These are real authenticated QUIC transfers across local or shaped namespaces on
one Linux host. No new two-host Internet WAN, physical-energy, transparent TUN
runtime-compression or current Combined MVM qualification is claimed. Per-object
CPU and wall overlap; packet bytes are measured at session scope. Retain every
negative and distinguish matched-ratio medians from ratios of separate medians.
