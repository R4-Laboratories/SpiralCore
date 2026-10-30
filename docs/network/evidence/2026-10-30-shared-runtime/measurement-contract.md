# Shared-runtime transfer measurement contract

Status: registered before acceptance measurements. Implementation and qualification
are separate from performance results. The initial campaign has not been run by
the harness author. The accepted [design](../../../specs/2026-10-30-shared-runtime-design.md)
defines `RustReplayV1`; this experiment does not claim Python card-byte compatibility.

## Workloads and paired comparisons

The executable is [examples/shared_runtime.rs](../../../../crates/spiralcore-transfer/examples/shared_runtime.rs).
Its `plan` command generates actual canonical traces from `State::seeded(width)`
with payload 1.0 and `Recipe { steps, output: Trace }`. It records checkpoint,
prediction, target, and recipe identities, actual lengths, source-file hashes,
source revision, and a unique public run identifier. There is no padding to a
requested headline size. Planning is untimed fixture qualification, not a trial.

The initial local dose consists of **two separate plans**, avoiding a Cartesian
product:

| Width | Steps | Cases | Trials per case | Methods |
| --- | --- | --- | --- | --- |
| 48 | 243 | generated, edited, unrelated | 10 | raw, zstd, adaptive, runtime |
| 4096 | 4096 | generated, edited, unrelated | 10 | raw, zstd, adaptive, runtime |

This is 60 paired inputs, 240 target transfers, and **240 separately recorded raw
checkpoint seeds: 480 transfer operations total**. Actual byte sizes come from
the generated plans; they must not be guessed or rounded to 16 MiB. If any actual
trace exceeds the profile/transfer limits, qualification fails before a campaign.
Changing this dose after inspecting measurements requires a new registration.

The supported cases are:

- `generated`: exact full-state trace returned by runtime replay.
- `edited`: one bit flipped in the middle byte of that trace.
- `append`: the same trace followed by 32 deterministic pseudorandom bytes.
- `truncate`: the same trace without its last 17 bytes.
- `unrelated`: deterministic pseudorandom bytes of exactly the prediction length.

Append and truncate are available for qualification and later registered runs;
they are not added silently to the initial dose. Modified targets are arbitrary
opaque byte objects, not necessarily valid runtime checkpoints or trace files.
The residual must preserve those bytes exactly. Random data uses a fixed
xorshift64 generator and a domain-separated width/steps/trial seed. Each method
gets identical input bytes. All methods start from an independently seeded,
fresh authorization scope. `warm_probe=false` prevents a previous target hit
from turning the comparison into an identity probe.

The raw and zstd methods call the existing batched transfer API. `adaptive`
compares the current raw, zstd, sparse, and whole-stream prefix candidates against
the checkpoint base. `runtime` evaluates those same candidates plus runtime
replay/residual and sends the smallest candidate under existing tie rules. All
candidate computation remains in its timer, including a losing runtime candidate.
The candidate costs and actual selected mode are retained. There is no forced
runtime-only arm and no hidden fallback after receiver failure.

This baseline set is the repository's configured transfer implementations. It
does not establish superiority over every zstd setting, an unprefixed
whole-object zstd implementation, rsync, or VCDIFF. The conventional adaptive arm
does include its existing whole-stream prefix encoding. Report that scope when
describing any benefit.

Method order rotates by `(trial + case_index) mod 4`. Trials run serially. No
measurement is discarded as a warmup or outlier. Do not run builds, tests,
profilers, or another measurement job concurrently with a timed campaign.

## Timing and byte scopes

Every paired input has a `generation` row: seeded checkpoint construction,
runtime replay, and target mutation, measured once before its four methods.
Identity hashing for evidence is outside that generation timer. The generated
bytes are checked against the plan before transfer.

Every `seed` and `target` operation records:

- `operation_wall_us`: input-ready to commit acknowledgement, including all
  candidate preparation, sender encoding, serialization, exchange, receiver work,
  and durable publication. Endpoint opening, JSONL writing, post-commit reopen
  verification, and cache cleanup are outside this timer.
- `operation_cpu_us`: the corresponding process CPU. In local mode this is
  already combined sender plus receiver CPU. In remote mode it is sender CPU.
- `receiver_cpu_us` and `receiver_wall_us`: receiver batch decoding, endpoint
  handling/publication, and reply encoding. Locally these are nested subsets.
  Remotely the existing server reports them; they omit server HTTP reading,
  network I/O, TLS termination, and relay CPU.
- `exchange_cpu_us` and `exchange_wall_us`: nested exchange-call scopes.
  Remote exchange wall includes serialization, request wait, server work, and
  response processing. **It is not pure byte-on-line transmission time.**
- `verification_cpu_us` and `verification_wall_us`: post-operation identity and,
  locally, exact reopened-object comparison. These are explicitly separate.
- Request and response body bytes, their sum, protocol frame bytes, batch count,
  selected candidate, and projected body cost. Successful adaptive/runtime cost
  must equal observed serialized body cost exactly.

`local` executes actual `wan::encode_batch`/`decode_batch` serialization and the
real endpoint in one process. Replies count the 16 bytes used by SCWREP01's two
timing fields. Its byte field is labeled **HTTP-body-equivalent**: no TCP, HTTP,
TLS, or WAN has occurred.

`remote` uses `wan::HttpClient` against the existing authenticated server. Its
body counts are actual HTTP request/response payload sizes, excluding HTTP
headers, TLS records, IP/TCP headers, retransmissions, and relay traffic. They
are not complete wire-byte measurements. A separate transport capture or
counter reconciliation is required before making that stronger claim.

Never add local receiver CPU or exchange time to local operation totals. For
separate remote processes, sender CPU plus reported receiver-handler CPU is a
useful measured subtotal, explicitly excluding the remaining host/relay work.
Generation can be added once to a method's transfer scope for an application-to-
ack model, but this is a composition of separately measured intervals, not a
second directly observed end-to-end clock. Generation is workload construction,
including prediction construction even for the unrelated control. It is not an
unavoidable cost of transferring an already existing file. Only apply this
composition when the modeled application actually creates its data this way.

## Fidelity, storage, and evidence

`ColdGraphPolicy::Disabled` is fixed for every arm. Checkpoints are transferred
through the ordinary raw/verified-object path; their cost is recorded before
the target. Seed cost must be shown separately, added for a cold-start total,
and amortized explicitly if a later run reuses the same checkpoint. The initial
campaign measures one target per seeded scope and does not silently amortize.

The raw and zstd arms receive a checkpoint only to keep measured setup equal;
their codecs do not need it. Retain those observed seed rows and report the
equal-setup totals, but use **runtime seed plus target versus raw/zstd target
only** for the practical cold-start comparison. The existing adaptive arm does
use the checkpoint for its sparse/prefix candidates, so its seed remains part
of its cold-start total. An unnecessary baseline seed must not inflate a claimed
runtime advantage.

Local success requires closing the endpoint, reopening its private store, and
comparing every recovered byte with the original target. Acknowledged object
identity and exact length must also agree. Only then is that owned arm's cache
removed; a `cleanup` receipt records the action. The enclosing cache root must
be new, is never reused, and remains as an empty directory after success. Failed
arms retain their cache. No pre-existing directory is removed.

Remote client records explicitly say
`commit-ack-only-pending-receiver-reopen`. After stopping the server, run `verify`
on that machine against its retained cache and the exact plan. It regenerates
the registered expected fixtures and performs an exact read/comparison in each
authorized scope. It records **every** seed/target verification, including
failures. This is an independent post-restart storage comparison, not an
independent implementation of the mathematical runtime. Runtime semantic parity
is qualified by the separate reference-vector tests.

The JSONL output is opened with `create_new`; existing evidence is never
overwritten. Each row is flushed and synchronized. Generation/operation starts
are recorded before work, so interruption leaves an identifiable unfinished
observation. Successful and failed operation rows retain available bytes and
timers. A preparation failure has unavailable metrics marked null rather than
zero. The driver stops after a transfer failure, preserving all preceding rows;
it never retries with a different method or discards the failing trial. A final
`complete` row is necessary for a complete campaign. Incomplete campaigns must
not be summarized as the registered full dose.

Run headers bind the plan file, executable, compiled source-file content,
operator-provided source revision, OS, and architecture. The remote receiver
executable hash is explicitly operator-supplied until corroborated by retained
server-side build evidence. The environment, compiler version/features, source
cleanliness, command, and independent review receipts must be retained by the
campaign owner before timing. Source hashes alone are not a clean-tree claim.

Remote plans are conservatively bounded to 768 MiB of potential cumulative
checkpoint/target objects and 1,800 transfer attempts. This protects the existing
1 GiB cache and 2,048-attempt profile from a full-matrix accumulation. It is a
planning bound, not a memory-use measurement. Split WAN work into explicit plans
and fresh server directories. The public run ID and every method/cell/trial enter
scope derivation; never reuse a run ID for another attempt at the same plan.

## Commands after source freeze and authorization to measure

Use an absolute Rust Cargo path if another tool named `cargo` shadows it. Build
the release example and `spiralcore-wan` with the qualified features first; retain
their hashes. The examples below assume the resulting executable path is stored
in `SHARED_RUNTIME_EXE` and the verified source revision in `SHARED_RUNTIME_REV`.
These are instructions, not records that the commands were run.

```sh
"$SHARED_RUNTIME_EXE" plan --out local-small-plan.json --source "$SHARED_RUNTIME_REV" --run-id shared-runtime-small-01 --transport local-mechanism --widths 48 --steps 243 --cases generated,edited,unrelated --trials 10
"$SHARED_RUNTIME_EXE" local --plan local-small-plan.json --out local-small-results.jsonl --cache NEW-small-cache
"$SHARED_RUNTIME_EXE" plan --out local-large-plan.json --source "$SHARED_RUNTIME_REV" --run-id shared-runtime-large-01 --transport local-mechanism --widths 4096 --steps 4096 --cases generated,edited,unrelated --trials 10
"$SHARED_RUNTIME_EXE" local --plan local-large-plan.json --out local-large-results.jsonl --cache NEW-large-cache
```

A remote shard is separately registered, for example one cell/case and one
trial. Its plan includes `allowed_scopes` in the existing server's schema v4.
Provide `SPIRALCORE_WAN_TOKEN` through the environment on sender and server;
never put it in arguments, receipts, or published output. The harness does not
print the token or authorization headers. HTTPS is required except explicit
numeric loopback HTTP used for qualification. This is the existing synthetic
benchmark transport, not end-to-end mailbox encryption or FIPS qualification.

```sh
"$SHARED_RUNTIME_EXE" plan --out remote-plan.json --source "$SHARED_RUNTIME_REV" --run-id shared-runtime-remote-01 --transport remote-http --widths 4096 --steps 4096 --cases generated --trials 1
# On the authorized receiver, start the existing spiralcore-wan server with
# --plan remote-plan.json --cache NEW-remote-cache; expose it through the
# separately approved HTTPS transport. Retain that server's executable hash.
"$SHARED_RUNTIME_EXE" remote --plan remote-plan.json --out remote-results.jsonl --url https://APPROVED-ORIGIN/ --receiver-exe-sha256 VERIFIED_RECEIVER_EXECUTABLE_SHA256
# After stopping that server, on its machine:
"$SHARED_RUNTIME_EXE" verify --plan remote-plan.json --cache NEW-remote-cache --out remote-receiver-verification.jsonl
```

No remote account, relay, server, paid resource, or WAN campaign is created by
this implementation task.

## Analysis and decision scope

Retain all raw observations. Report actual target sizes, seed and target bytes,
selected-mode counts, and per-case/per-size medians and nearest-rank p95 for
operation CPU/wall and nested exchange/receiver scopes. Preserve paired method
ratios and all negative controls. Use both raw and the existing adaptive method
as comparisons; do not attribute ordinary zstd/prefix gains to runtime replay.

A byte-saving result is not automatically a CPU or completion-time improvement.
Report each axis separately and give cold seed-plus-target totals. Do not infer
energy savings from CPU time alone. A bandwidth break-even calculation can use
observed byte savings and additional compute as an explicitly labeled model;
local timings do not prove WAN speed. A real WAN claim requires a separately
authorized, retained remote run with receiver verification and transport scope
clearly stated.

Qualification currently covers fixture edits, plan scope/size binding, output
preservation, selected-cost equality, exact local reopen, missing checkpoint
failure, and loopback HTTP acknowledgements followed by separate stopped-server
verification. Qualification outputs, including intermediate failures, belong in
`qualification/harness/`; the campaign owner performs the final frozen release
suite and independent source review before any acceptance timing.

## Registered remote WAN pilot extension

This extension is registered before remote execution. Its purpose is to qualify
the same Rust replay mechanism across the native sender/receiver architectures
and observe a small real relay transfer. **Three paired trials per workload are
a pilot, not the earlier 30-pair WAN acceptance program.** No measurement or
remote dispatch occurred while authoring this workflow.

The new [workflow](../../../../.github/workflows/spiralcore-shared-runtime.yml)
has one fixed dose: width 4096, steps 4096, cases `generated`, `edited`, and
`unrelated`, three trials, and the same four methods. This produces nine paired
inputs, 36 target transfers, and 36 separately recorded raw checkpoint seeds:
**36 fresh authorization scopes and 72 operation rows**. The existing
conservative remote plan bound admits these scopes at a maximum object charge
of 578.25 MiB; this is not measured resident memory. Actual generated lengths and
hashes must appear in the uploaded plan, with no size padding.

Execution requires all of the following:

- A push to `codex/spiralcore-shared-runtime`, with its exact commit matching the
  dedicated repository variable `SPIRALCORE_RUNTIME_SOURCE` and run attempt one.
  There is no manual-dispatch, pull-request, schedule, or main-branch trigger.
- Dedicated `SPIRALCORE_RUNTIME_URL`, `SPIRALCORE_RUNTIME_RUN`, and
  `SPIRALCORE_RUNTIME_RECEIVER_SHA256` variables. The origin must be HTTPS on one
  valid lowercase `*.trycloudflare.com` host, optionally ending in `/`, with no
  username, port, query, fragment, or additional path. The public run ID has at
  most 64 alphanumeric/hyphen characters and starts alphanumeric. Source and
  receiver hashes have exact hexadecimal lengths.
- The dedicated `SPIRALCORE_RUNTIME_TOKEN` secret, injected only into the final
  remote-transfer step as the harness's `SPIRALCORE_WAN_TOKEN` environment input.
  The existing WAN campaign's repository variables and secrets are not changed
  or referenced. No token value or authorization header enters the artifacts.
- A separately prepared receiver from the same source/profile, its corroborated
  executable hash, the same exact plan, and a fresh private cache. The parent
  coordinates the existing receiver and authorized tunnel; the workflow does
  not create an account, VM, paid service, or receiver process.

The job has read-only repository permissions, a separate concurrency group, no
automatic cancellation of an earlier pilot, and a **20-minute job limit**.
Checkout and artifact-upload actions are pinned to the repository's existing
reviewed commit SHAs. Each sender is freshly compiled on `ubuntu-24.04` with Rust
**1.97.1**, release/default features and the locked dependency graph. It rejects
compiler-wrapper/flag/target overrides, records the compiler, native target,
runner/CPU, feature tree, tracked sources, and executable hashes, and does not
reuse a previous build artifact or Cargo build cache.

Before the credential-bearing step, the workflow must pass Cargo check,
Clippy with warnings denied, the exact frozen canonical checkpoint/trace-vector
test on `x86_64-unknown-linux-gnu`, the full relevant release all-target suite,
and a fresh build. Source hashes are rechecked after building. It then generates
the actual plan and independently checks its fixed configuration, nine fixtures,
36 unique scopes, 72 operations, and bounded target lengths. A failed source,
vector, test, plan, or capability gate prevents the remote transfer.

The transfer is one serial invocation, with no explicit retry or campaign rerun.
It checks sender binaries before and after the run and checks that all 72
operation records succeeded with exactly one completion record. As in the local
contract, losing runtime-candidate CPU is retained. HTTP-body counts exclude
headers, TLS records, retransmissions, and relay/network framing; **this workflow
does not capture packet bytes and cannot establish a complete-path byte total**.
Timing includes the real remote request path, but nested exchange time still
does not isolate pure wire transmission.

Plan, JSONL observations, partial/error logs, test/build/vector evidence, source
and executable identities, and the sender executable are sealed and uploaded in
`always()` steps, with 30-day GitHub artifact retention. Hard runner loss can
prevent an upload; an absent or incomplete artifact is an incomplete pilot,
never an implicit success. The campaign owner must retain the downloaded raw
evidence durably in the project rather than relying on artifact expiry.

Sender completion remains **pending receiver verification**. After stopping the
receiver, run the harness `verify` command against its retained cache and the
same plan, retain all 72 verification results, and join them to sender scopes
and object identities. A mismatch, missing object, unsupported architecture,
interrupted run, or incomplete join prevents a fidelity-success claim. The
receiver executable hash supplied to the sender is operator metadata until
corroborated by the receiver's own build and verification receipts.

The pilot has no new numeric pass threshold for speed or CPU. Report all nine
paired inputs, actual sizes, selected modes, raw/adaptive comparisons, practical
cold-start comparison, seed cost, CPU scopes, observed elapsed time, and
unfavorable controls. Its three samples per cell cannot support the original
30-pair acceptance or a reliable tail-latency claim. No energy saving is inferred
without an energy measurement.

Workflow gate qualification is retained under `qualification/workflow/`.
`check-gates.sh` parses the actual YAML, checks trigger/source/attempt/credential
isolation and artifact-retention structure, checks every embedded shell block
with Bash and ShellCheck, and exercises valid configuration/authentication plus
17 rejecting cases. The initial missing-workflow RED and subsequent qualification
output are retained. This local check neither dispatches the workflow nor probes
the configured origin.
