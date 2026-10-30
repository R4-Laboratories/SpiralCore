# Two-machine WAN experiment

This experiment connects a standard GitHub Actions Linux runner to a Mac receiver through a temporary Cloudflare Quick Tunnel. It uses public synthetic objects only. Standard hosted runners are free for public repositories; the workflow retains compact evidence for one day. Account artifact-storage quotas still apply. No paid VM is provisioned. See the [latest 16 MiB results](large-wan-results-2026-10-29.md), [earlier 1 MiB efficiency results](efficiency-results-2026-10-29.md), [earlier October 29 baseline](wan-results-2026-10-29.md) and retained compact evidence.

This is a real Internet relay path, not a shaped-loopback or direct-peer test:

```text
GitHub Linux sender -- HTTPS --> Cloudflare -- encrypted tunnel --> Mac loopback origin
```

Cloudflare terminates HTTPS and can see payloads and the temporary bearer capability. This adapter is not the future ciphertext-only MVM relay. The origin accepts only allowlisted scope IDs and bounded object-frame batches; it does not expose arbitrary files or commands. Neither this transport nor its dependencies establish FIPS validation.

## Build and prepare

For the separate compressed-schema comparison, use
`.github/workflows/spiralcore-schema-wan.yml` and the `schema_fitting` example.
A v4 plan can explicitly enable `codec_cache_enabled: true`; omission preserves
the previous default. This enables retained decoder state and compact admitted
request/cache observations on stderr without changing wire framing. Keep the
receiver supervisor alive throughout all sessions, confirm the actual HTTPS
origins respond before dispatch, and stop receivers before independent readback.
The [registered commands and evidence](evidence/2026-10-30-schema-wan/README.md)
and [measured result](schema-wan-results-2026-10-30.md) describe the three bounded
shards, source-derived baseline, persistent state, body accounting and CPU scope.

Use Rust 1.97.1, the compiler pinned in the test workflow:

```sh
cargo +1.97.1 build --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --bins
```

The binaries are `spiralcore-wan` (temporary receiver), `spiralcore-bench` (plan/run/verify/summarize), `spiralcore-capture` (packet accounting) and `spiralcore-capture-control` (Linux capture lifecycle and qualification). The `spiralcore` loopback CLI also supports explicit sparse sends; its frame exchanges remain sequential.

For every campaign, generate a new random 64-character lowercase hexadecimal run ID and a separate random 256-bit capability. Keep the capability in a private file/environment variable named `SPIRALCORE_WAN_TOKEN`; never put it in the URL, command line or a tracked file. Create a plan with `spiralcore-bench plan --run RUN_HEX --trials 1 --out plan.json`. Use one trial for connectivity and thirty for the paired campaign. Plan outputs are exclusive and never overwritten.

Start `spiralcore-wan serve --cache NEW_CACHE --plan plan.json --bind 127.0.0.1:0 --seconds 3600` with the capability environment set. It prints the bound address. Point a temporary Cloudflare Quick Tunnel at that loopback address. The origin refuses to reuse a populated cache. Retain previous caches for verification.

## Hosted workflow

`.github/workflows/spiralcore-wan.yml` runs on the feature branch only when its commit exactly matches `SPIRALCORE_WAN_SOURCE` and the run attempt is one. Configure these temporary repository variables:

- `SPIRALCORE_WAN_SOURCE`: the reviewed full source commit.
- `SPIRALCORE_WAN_URL`: the current HTTPS trycloudflare.com URL.
- `SPIRALCORE_WAN_RUN`: the same run ID used to generate the receiver plan.
- `SPIRALCORE_WAN_TRIALS`: 1 or 30, matching the receiver plan.
- `SPIRALCORE_WAN_SUITE`: `efficient-warm`, `efficient-prefix`, `large16-localized`, or a registered `broad16-*` case on `codex/spiralcore-efficient`; leave absent for the legacy WAN branch.
- `SPIRALCORE_WAN_SHARD`: `0..2` for `large16-localized` or `0..4` for `broad16-*`; both require thirty global trials. Leave absent for the other suites.
- `SPIRALCORE_WAN_QUALIFY_ONLY`: set to `1` for a source-pinned Linux build/test/capture-qualification job with no WAN transfer. URL, run ID, trial count, suite and shard variables must all be absent. Omit this variable for measured campaigns.

The separate Actions secret `SPIRALCORE_WAN_TOKEN` contains the capability. Configure it from the private local file, not a shell literal. The workflow executes only reviewed repository code, grants read-only repository permissions and pins external actions. Triggering another campaign requires a new run ID, capability, empty receiver cache and source pin. Automatic retries are deliberately excluded.

The workflow builds/tests the exact source, generates the same plan, pins the tunnel hostname to one verified IPv4 address on its disposable runner while retaining TLS hostname validation, disables/validates TSO/GSO/GRO and captures that dedicated HTTPS flow with 96-byte header snapshots. It records the route, resolver result, offload state and tcpdump drop statistics alongside the receipts.

The runner executes `spiralcore-bench run --plan plan.json --url HTTPS_URL --out rows.jsonl` and then `summarize`. Failed transfers are retained and fail the job. Uploaded request-body counters on failed operations mean attempted body bytes, not successfully transmitted link bytes.

## Workloads and timing

The default v1 thirty-trial plan contains 575 isolated tasks and 906 seed/transfer operations. It includes 1 MiB cold random/structured data, exact repeats, localized edits, scattered edits and prefix insertion, plus a separate 16 MiB raw/zstd/cached-repeat smoke case. The quota forecast includes approximately 920 MiB of object payload plus publication overhead, below the one-GiB cache ceiling.

The v2 efficiency suites use `plan --suite efficient-warm` or `--suite efficient-prefix`, with the same
run/trials/out options. At thirty trials, the warm suite contains 480 tasks/900 operations and 780 MiB
of retained payload; the prefix suite contains 120 tasks/240 operations and approximately 240 MiB.
Run each suite against a fresh receiver cache and capability. Combining both under one cache would
exceed the quota once publication allowances are included.

The v3 `large16-localized` suite uses `--trials 30 --shard 0|1|2`. Generate all three
plans with one campaign ID. Each shard contains ten consecutive global trials of
the same 16 MiB base with a 32-byte midpoint edit, comparing `raw_full`, `zstd_full`
and `sparse`. All six arm permutations occur five times across the thirty trials.
Each arm gets its own raw seed and isolated authorization scope; every update
target is initially absent. Each shard retains 960 MiB plus publication headers
and requires a separate fresh receiver cache and temporary capability. Verify all
sixty seed/update receipts after stopping that shard's receiver, before proceeding
to the next shard. Do not combine shards in one cache or mix their receipt files.

For this experiment the Mac receiver uses the locally qualified native
`hardware-sha256` feature. The Linux sender retains default Cargo features and
sha2's native CPU dispatch. Record the feature trees, compiler/target, exact source
and binary hashes at both ends. Raw/zstd attempt bounded optional DAG preparation;
sparse skips it. At this object size the random DAG exceeds the node cap, so equal
durable-byte readiness does not mean equal preparation work. Report this CPU
comparison boundary alongside the measurements. The suite changes no protocol,
cache, namespace, resident-memory or HTTP-batch limit.

After the larger campaign, remove `SPIRALCORE_WAN_SHARD` along with the other
temporary variables and capability. Retain each shard's plan and receipts.

The v4 `broad16-*` matrix compares raw, ordinary zstd, zstd prefix and sparse at
16 MiB-class lengths. It disables optional raw/zstd DAG warming equally at the
receiver. There are twelve registered cases and five fresh-cache shards per case,
with six global trials per shard. Warm cases seed each arm separately; cold cases
do not. Insertions/appends start with shorter bases so the target stays within
16 MiB. See the [exact registered schedule](../plans/2026-10-29-broader-wan-and-scaling.md#registered-broad16-workload-schedule)
for fixture definitions, resource bounds, ordering and cleanup policy. This is a
new comparison profile; do not merge its results with the eager-warming v3 rows.

New captures require the lifecycle receipt as well as the packet parser's existing
checks. The supervisor verifies readiness, counter stability and bounded shutdown,
and separately records intermediate samples and final counters. Quiet samples are
not proof of a drained network. Before new WAN measurements, the exact Linux build
must pass live normal, interrupted and continuing-traffic qualification cases.
See the [capture investigation and contract](capture-lifecycle-decision.md).

Efficiency arms compare raw, zstd, ordinary exact-object cache reuse, current full-fold DAG sends,
retained prepared DAG updates and sparse recipes as applicable. Prepared sender base construction is
inside seed timing; the same object survives into transfer, whose timing includes target preparation
and terminal cleanup. Sparse arms retain the original base bytes. These are one-update experiments:
successor promotion is not performed. Reported resident charges are conservative owned-state
accounting, not preparation peak RSS. The full-fold DAG control uses the current executable, not the
historical pre-optimization binary; historical comparisons need their own source-pinned evidence.

Each pair uses identical fixture bytes and a seeded counterbalanced arm order. Cold raw/zstd uploads batch Begin/Data/Finish so the baseline is not penalized by one network round trip per chunk. DAG known-absent targets use the same batching opportunity. Only intentional exact-repeat arms probe first. Whole-object cache hits are shared with ordinary content-addressed reuse and are not credited as DAG-specific savings. The larger smoke case does not qualify a random 16 MiB DAG under the current node limit.

Every operation reports:

- End-to-end wall time from ready in-memory inputs, before required base/codec preparation, through a verified durable Published reply.
- Sender process CPU, including codec and its HTTP/TLS work; base preparation is also shown as a component.
- Receiver handler process CPU and wall time for frame decoding, base reconstruction, endpoint work, durability and reply encoding.
- Fixture generation CPU/wall separately, since generating synthetic input is not part of normal transfer.
- Application-frame and HTTP-body bytes in both directions; seed costs separately and cumulatively.

Receiver CPU excludes the separate cloudflared process, Cloudflare processing and HTTP socket wait. CPU seconds from ARM and x86 quantify consumed process time, not equal instruction counts or energy. Do not add CPU time to end-to-end wall time: the latter already includes computation.

Packet accounting observes only the runner-to-relay connection. It includes captured IP/TCP/TLS/HTTP traffic, acknowledgments and retransmissions on that leg. It excludes Cloudflare's internal routing and the relay-to-Mac leg. Capture completeness requires matching the pinned peer, classified traffic, unambiguous observed-SYN ownership, zero reported kernel drops, matching captured/received/PCAP-record counts and sufficient per-operation payload coverage. Reused tuples with different initial sequences are quarantined; the analyzer does not yet prove separate TCP epochs. Header snapshot length is not packet byte length. An incomplete capture can still supply observed totals, but cannot qualify a complete per-transfer wire-byte comparison. Inspect `capture_complete` and its reasons separately from workflow success.

## Exact-byte qualification and cleanup

After downloading the run artifact, stop the owned receiver process to release its cache lock. Run:

```sh
spiralcore-bench verify --plan plan.json --rows rows.jsonl --cache CACHE
```

Verification checks receipt identities against the canonical plan, regenerates each public fixture, reopens the persisted cache and compares every byte and length. This independent qualification occurs outside transfer timing and avoids sending reconstructed objects back across the WAN just to verify them.

Stop only the campaign's own tunnel, receiver and capture processes. Delete the temporary Actions secret and all campaign variables, including the optional suite variable. Retain compact source-pinned results, raw drop/offload evidence and the receiver verification result. Keep failed trials and negative workloads in the report. Public runner execution alone is not proof of complete wire-byte savings; the remote receipts, complete capture and local exact-byte verification must agree for that claim. Successful protocol-body accounting and exact-byte reconstruction can be reported separately when their own checks pass.

Official service references: [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions), [Cloudflare Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).
