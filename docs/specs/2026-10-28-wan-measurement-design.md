# Standalone WAN measurement

User authorization: implement the existing WAN plan now, before MVM; prioritize exact bytes and fewer transmitted bytes, and measure total time and compute time. No remote machine is available; use a free test service. This narrows the previously approved WAN design to a reproducible two-machine relay test.

## Decision

Use a standard GitHub Actions Linux runner as sender and the user's Mac as receiver. Both execute the same pinned Rust source. A temporary accountless Cloudflare Quick Tunnel terminates HTTPS and forwards only to a dedicated loopback HTTP origin. Cloudflare is a trusted relay for this public synthetic-data experiment; this is not end-to-end confidentiality through the relay, a direct-host benchmark, MVM qualification, or FIPS validation.

Alternatives: a temporary direct VPS provides cleaner direct-path measurements but needs a cloud account/payment; an SSH tunnel requires an existing reachable host. The selected free path exercises actual Internet links without provisioning a paid resource.

The origin exposes only authenticated object-frame batches. A random 256-bit capability comes from a private environment variable and a temporary repository Actions secret. It is never a URL parameter or logged. Authorization precedes parsing object payloads. The origin binds only loopback, uses a dedicated marked cache, and stops after a bounded campaign. No remote arbitrary-file or shell API exists. A client-supplied 32-byte scope is bound into a namespace under the one authorized experiment capability; it cannot escape the endpoint directory. Scope separates trial cache states.

## Binary HTTP contract

POST /v1/batch; Authorization: Bearer followed by 64 lowercase hexadecimal characters; X-Spiral-Scope: 64 lowercase hexadecimal characters; Content-Type: application/octet-stream. No redirects, HTTP content compression, transfer encoding or cookies. Client accepts HTTPS, or literal loopback HTTP for integration tests only.

Request body: magic SCWBAT01 (8), count u32 little endian, then count repetitions of frame length u32 and an existing SCMOBJ01 frame. At most 512 frames and 20 MiB body; each frame remains at most 64 KiB. Header budget 16 KiB and five seconds; total request I/O deadline 60 seconds. Server processes frames in order and returns replies up to the first Published or protocol Error. It refreshes the endpoint clock and checks the deadline between operations; synchronous endpoint computation is not forcibly preempted. Authentication errors have no endpoint side effects. All responses are no-store and close the loopback origin connection so an idle pooled connection cannot monopolize the serial receiver. Sender-to-relay TLS reuse remains the relay/client's responsibility.

Response body: magic SCWREP01 (8), receiver CPU microseconds u64, receiver endpoint wall microseconds u64, count u32, then length-prefixed SCMOBJ01 replies. Receiver CPU/wall cover frame decode, endpoint handle (including base reconstruction and durable commit), and reply encoding. They exclude socket wait and the cloudflared process. Client rejects missing/extra replies, identity/sequence mismatches, malformed envelopes and inconsistent telemetry. Body sizes include telemetry when counted as HTTP application payload.

Batching is an ordered HTTP transport optimization, not a change to object identity/commit semantics. Raw and zstd can upload all data with Begin/Data/Finish in one request. Warm DAG can send its proposed difference in one request; Need causes at most two batched repair rounds. A Begin cache hit is terminal, so later proposed frames are ignored, and the sender records the actual upload bytes rather than pretending unsent data was avoided. To preserve exact-repeat benefits, sender first sends Begin alone when expecting a warm cache; cold full-copy baselines use the single streaming batch. This policy is explicit in each measurement.

## Measurements

Start the end-to-end clock before required target/base preparation and stop only after a verified durable Published reply. Record sender process CPU, base preparation wall/CPU, receiver endpoint process CPU and wall, application frames and HTTP payload bytes in each direction. Sender HTTP/TLS CPU is included; receiver relay CPU is excluded and separately labeled. Do not add CPU seconds to wall seconds: wall already includes compute.

Capture only the test HTTPS flow on the remote runner with bounded header-only packet capture; report captured IP bytes in both directions, including TLS/HTTP overhead, handshakes, transport ACKs and retransmissions. This observes the runner-to-relay leg, not Cloudflare's internal network or the relay-to-Mac leg. If capture is unavailable or incomplete, mark link-byte evidence unavailable rather than substituting body bytes. Disable supported offload aggregation on the disposable runner before capture. Keep setup and qualification traffic separate.

Use deterministic public fixtures, unique scope IDs, seeded counterbalanced paired ordering and at least 30 paired trials for the primary 1 MiB workloads. Compare full-copy raw and zstd, ordinary exact-object cache reuse, and DAG differences with matching initial state where applicable. Report cold seed costs separately and cumulatively. Repeat hits common to raw/zstd/DAG are general content-cache savings, not DAG-specific advances. Prefix insertion and scattered edits must be reported even when negative. A 16 MiB cold/repeat smoke case does not qualify 16 MiB DAG: the current node limit may refuse it.

The receiver subsequently reopens the persisted cache and compares every recovered object byte-for-byte against the deterministic fixture described in the signed-in runner's receipts. This qualification check is outside the normal transfer timing and does not transfer the entire objects back over the WAN.

## Gates and cleanup

Use TDD for batching, framing/auth/limits, measurement accounting, deterministic workload generation and summary statistics. Run existing tests unchanged. Independent Rust/security review precedes publishing the feature branch and triggering the bounded free workflow. No merge to main is required. Pin action revisions, use read-only job permissions, no untrusted PR trigger, and a 60-minute timeout. Stop only owned origin/tunnel/capture processes, remove only this campaign's temporary secret/variable, and retain compact receipts with exact source/binary identities. Do not claim real WAN results until the hosted run and receiver equality checks succeed.

Every hosted run requires a fresh run ID and an empty receiver cache. The workflow runs only the explicitly pinned source revision and attempt one. Benchmark transfer IDs are fresh per execution; unexpected Begin cache hits in seed/cold/mutated-target phases fail qualification. Only intentional warm exact-repeat arms may accept that shortcut. This prevents a repeated job from turning cold cases into mislabeled cache hits. The runner pins the tunnel hostname to one verified IPv4 address without changing TLS hostname verification, disables and verifies TSO/GSO/GRO, and captures that numeric endpoint. Successful per-row IP upload/download counts must cover their respective HTTP body counts; drop statistics and unattributed setup bytes remain evidence.
