# SC-OPT-019 phase 2: exact record preflight and capped counting

This prospective amendment retains the [original measurement contract](measurement-contract.md),
fixtures, arm order, baseline revision, timing scopes, byte accounting, capacity
bounds and acceptance thresholds. It registers one changed candidate after the
first local campaign failed advancement. It does not authorize rerunning an
unchanged candidate to obtain a better timing result.

The first campaign at source `a128b3a` remains in `local/`: **700 exact transfers**,
full baseline/candidate frame and body parity, and 600-byte localized random
updates. Its [summary](local/summary.json) SHA-256 is
`6453a9b9d8d10726a0353589ee20c365ad54e4eabf7a6847970519cd8acaaf12`.
The replacement sender-CPU median paired ratio was 0.8872938354 at 1 MiB and
0.9599936918 at 16 MiB; the latter failed the required ≤0.95. Warm and cumulative
CPU gates passed at both sizes. No WAN campaign advances from that result.

The additional candidate changes are confined to schema encoding. In its
per-record loop, if a record exceeds `MAX_TEMPLATE_RECORD`, or its first byte
after the existing JSON-whitespace scan is not `{`, emit the identical literal
record immediately. Length is checked first. Learned and artifact-validated
templates have JSON-object skeletons, and the existing parser rejects those
inputs without consuming match work. Records that could begin a JSON object
retain the original UTF-8 validation, matching, budget and literal fallback.
The second change computes the existing capped record count with
`memchr_iter(...).count()` over fixed 64 KiB input blocks. Stop after a block if
the cumulative newline count exceeds `MAX_RECORDS`, returning `MAX_RECORDS+1`;
otherwise include the nonempty unterminated tail after the final block. This
must equal `records(target).take(MAX_RECORDS+1).count()` exactly, including
empty input, final LF, block boundaries and the record limit. It allocates no
offset list and leaves the actual record-emission iterator unchanged. The
bulk-count implementation is a prospective CPU improvement, not a speed claim.
The whole-literal decision, artifact, intermediate format, compression settings,
decoder and transport remain unchanged. The architect's proofs and exact-output
qualification precede the candidate freeze.

The retained combined stage diagnostic measured independent 16 MiB schema
encoding at 5.586 ms median and independent full selector preparation at
96.103 ms median. These overlapping calls are not additive pipeline components
or a measured bound on transfer improvement. Five of the ten registered large
replacement inputs—trials 3, 4, 6, 8 and 9—already exceed 65,536 records and
therefore take the earlier whole-literal branch. The prefix preflight cannot
remove matching work that those inputs already bypass; the capped-count change
can affect their count phase. Keep those trials in the same primary cohort; do
not select only benefiting inputs.

Freeze phase 2 source and executables before running the unchanged harness into
a **new phase-2 output directory**. Preserve the original `local/` files,
profiles, qualification and executable bindings. The source-derived baseline
still overlays `delta.rs`, `schema.rs`, `Cargo.toml` and `Cargo.lock` from
`56e15db5c77fe2ecdc03476d8e09664cd4213fd1`; all other measured source pins match
the candidate. The baseline and candidate are both persistent WarmSession arms,
alongside the same prepared Whole control. Every arm retains its actual paid
setup, generation, base seed, artifact publication when applicable and ten sends.

Run exactly ten local pairs per size: 700 transfers across ten bounded stores.
Retain all first-use observations, failures and controls, with exact stopped-store
readback and full winning frame/mode/body parity. At **each** size require:

- Target 2 replacement sender-CPU median paired ratio ≤0.95.
- Per-trial targets 1+3 sender-CPU sum median paired ratio ≤1.05, both at 600 bytes.
- Setup + actual schema publication + ten-target sender-CPU median paired ratio ≤1.05.

Report phase 1 and phase 2 independently; do not pool their observations or
describe phase 2 as isolated causal attribution of either new check. Only a passing
phase-2 local result can advance the previously registered six-pair, 420-transfer
WAN pilot. It remains a pilot, not a thirty-pair acceptance result. No new stage
profiling dose, harness framework or unregistered performance retry is added by
this amendment.
