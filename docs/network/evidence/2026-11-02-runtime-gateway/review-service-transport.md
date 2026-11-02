# Narrow object-service and channel review

Reviewer: `/root/mvm_boundary`, code/security/Rust reviewer, 2026-11-02.
The active worktree roster `d1927d9e7ce51354` was freshly verified (13 agents,
14 skills); all three review roles and the subsequently needed debugger role
returned active. No conflicting in-progress file claim was present. The original
service was authored by `/root`; transport/channel by `/root/mvm_tests`. The
reviewer's own shared object facade is excluded from this independent review.

## Findings and bounded repairs

1. **Namespace configuration was not compared across authenticated peers.** A
   client with another scope or policy epoch could publish successfully into the
   server's configured namespace while reporting its different local namespace.
   This was configuration cross-wiring, not permission supplied by remote data.
   The behavioral RED in `service-scope-red.txt` reproduced acceptance. The
   authorized repair requires one initial SCWBAT01 singleton frame with kind 6,
   the locally derived namespace as transfer ID, sequence 0 and body `SCGNSP01`.
   The server compares the complete frame with its own expected value and echoes
   it; the client compares the echo. This is intercepted before Endpoint creation
   or codec preparation, never passed to the object engine, and grants no scope.
   The new wrong-scope and wrong-epoch cases both pass and create no store.
2. **Offered mode alone does not prove execution of that decoder.** The parent
   requested explicit publication-path attribution. The repair observes whether
   a successful Published reply corresponds to Begin, and exposes that path only
   after the existing client validates correlation and object identity. A new
   regression sends a duplicate RuntimeV1 offer without a preliminary probe:
   payload bytes are uploaded, but the server returns Published at Begin. It is
   correctly labeled `verified-cache-hit`, while fresh generated/residual objects
   are `verified-transfer`. Sent-byte counts are preserved. Missing path metadata
   also failed in the retained RED; final service tests pass 3/3 in
   `service-scope-green.txt` (exit 0).
3. **Object-profile sending lacked the explicit datagram guard present on
   receive.** A raw authenticated peer can advertise datagram support even under
   the object ALPN. The transport owner accepted a narrow outgoing profile guard
   and is retaining its own targeted RED/GREEN. Closure is pending below; no
   wider transport redesign was requested.

This reviewer implemented repairs 1 and 2 after the parent's explicit assignment.
Their independent closure therefore belongs to `/root/mvm_tests`; this receipt
does not self-approve those authored changes.

## Reviewed service boundaries

- Bounded config parsing rejects unknown fields, unsupported profiles/actions,
  inconsistent auxiliary inputs and excessive groups/operations. Bounded file
  reads include one extra byte to detect oversized data. Prediction recipes use
  the existing canonical decoder and bounded runtime, never received source code.
- TLS identity and exact leaf pins determine sender/recipient ordering. Receiver
  authority accepts only its locally configured complete Context; payloads cannot
  select a tenant, namespace or epoch. Policy changes require restart. The
  namespace-equality repair adds fail-closed configuration agreement.
- One group retains one explicit Raw/Whole/Warm/Runtime sender. Original candidate
  receipts retain their HTTP-body model and do not claim global or QUIC-wire
  optimality. Runtime preparation and auxiliary seed bytes are paid separately.
- A bounded blocking worker runs existing codecs/Endpoint logic; network work
  remains on Tokio. Durability and exact object identity are checked by Endpoint
  before Published. On end, the receiver closes and reopens its store, checks all
  committed IDs and optional local exact fixtures, then acknowledges finish.
  Errors never produce an unqualified successful completion.
- Operation timers include input read/hash, preparation/selection and validated
  publication response. JSON output is deferred until exchanges finish. Server
  batch timings cover Endpoint processing; stream framing/waits remain in whole
  process totals. Group/setup/finish/readback costs are separate, nested counters
  are not added into process CPU, and capture must include transport close.

## Reviewed transport boundaries

- Separate `spiralcore-object/1` ALPN, TLS 1.3 mutual chain validation, hostname
  checking and both exact peer pins remain enforced. 0-RTT/resumption are disabled.
  Object receive windows and send window are bounded; the profile permits one
  bidirectional stream, no unidirectional streams and no incoming datagrams.
- The owned channel reuses canonical SCWBAT01 encoding/decoding with a 20 MiB
  prefix cap and 512-frame limit. Malformed/truncated input is an error; zero is
  a distinct orderly-end record. Exact application replies remain validated by
  the existing object client.
- An in-progress future sets Busy before I/O. Cancellation, timeout or partial
  failure poisons and aborts the channel; a dropped future cannot later resume
  from an unknown frame boundary. Dropping an unfinished channel aborts it.
- Finish requires zero acknowledgment, stream FIN with no trailing bytes, and
  transport delivery acknowledgment. The retained immediate-client-drop test
  covers the closing race. These transport acknowledgments do not replace
  application publication or processing semantics.
- Channel counters count completed logical prefix/body operations and explicitly
  exclude incomplete I/O; Quinn counters/capture retain the independent network
  accounting. Previous packet-profile behavior is separate.

Observed existing receipts: `transport-boundaries-01.txt` has 8 passing channel
tests and `packet-transport-regression.txt` has 5 passing unchanged packet tests.
This review did not rerun either. Those tests cover peer/profile rejection,
bounded framing, partial reads, timeout/cancellation/dropped futures, explicit
finish and exact reused-stream batches.

## Snapshot identities and disposition

| File | SHA-256 |
|---|---|
| `src/object_service.rs` after authored repair | `52824d56092d6af79682d58d71f41341261bf31fdbe17c95f9a9f42393e48d21` |
| `src/bin/spiralcore-gateway-object.rs` | `a5725824cc139a2326ac5c023401b64789f3262d657cbc3a3f53cffefb579b19` |
| `tests/object_service.rs` after authored repair | `a915f7e524842a4689b93c06a0ca09d767023b8f069de104e4d9a6460267ee70` |
| `src/object_channel.rs` | `8730387643fe8e09c1cc5cf4ccff95448a5f448c5f8c16b0938ad480bb635041` |
| `src/transport.rs` before outgoing-guard repair | `8062eaca7e99b3a81f0d244a661d4d953a51a38911b4795ead1f94278db2f6ad` |

No other actionable source finding was identified. Final disposition awaits the
small outgoing guard and independent service-repair closure. Integrated final
checks, frozen execution identity and network measurements remain parent-owned;
this review does not establish WAN performance or transparent TUN runtime gains.

## Outgoing-profile guard closure

**Transport/channel review PASS.** The owner added the explicit Objects-profile
rejection at the start of `Peer::send_datagram`, before inspecting remotely
advertised datagram capacity or touching the send queue. I independently checked
the three-line change and the retained real-QUIC regression against an
authenticated object-ALPN peer advertising datagram support.
`object-datagram-guard-red.txt` records the expected failing assertion; the GREEN
receipt records 9 passing object-channel tests and 5 passing packet tests, exit 0.
No test or build was rerun by this reviewer.

Final transport SHA-256:
`144786cdcbe4cf103a41c6879e926c4e860a01af6dda27aa5b4f4e7fa990c19a`.
Final object-channel tests SHA-256:
`2de551b255a9496e24ff32bce880e1b8601f222b2e3009eda47b99a72a5b97b1`.
The object-channel source hash above is unchanged.

The service-repair reviewer separately reported the namespace and attribution
repair correct and raised an output-path preservation finding to the parent.
That separate finding and final service identity are not closed by this transport
approval; the parent and independent service reviewer own its disposition.

## Independent output-preservation and attribution closure

**PASS, narrow repair review.** The code-reviewer roster eligibility was checked
again. The transport owner authored this repair; this reviewer did not edit its
source or rerun its tests. Before networking or error output, `run` validates the
configuration and reserves both fresh output paths. Canonical parents, protected
input/store overlap checks, exclusive lease/staging creation, and first-publication
hard links reject existing files, dangling symlinks, aliases and stale artifacts.
Subsequent atomic updates check the inode this invocation owns. Failure reporting
can write only through that acquired lease. Existing config paths are protected
by the general refusal to replace any existing output. The trusted local output
directory remains an explicit boundary; this is not a claim of immunity to an
adversarial same-UID filesystem race.

The retained `service-output-red.txt` reproduces three preservation failures;
`service-correlation-red.txt` reproduces missing transfer attribution. The final
`service-output-green-01.txt` has seven passing tests (exit 0), including unchanged
credential/input bytes, hard/symlink preservation, normalized store/output and
staging/lease collisions, clean owned error output, scope rejection, and real
paired Runtime/Warm exact reopen. Transfer IDs now join client operations with
accepted receiver batches; the service integration test checks their ID sets.

Reviewed final hashes:

- `src/object_service.rs`: `ff5b5104561cc2bb846ecd18bda76b06af31ec8fa7f7c130d59d7d1d9a041b23`
- `tests/object_service.rs`: `2de75804206429db0dd13c9385241e426d4d56bbbd533027522d24b58ca4b1b1`

No additional actionable finding was identified in this bounded repair. Runtime
campaign completion and performance interpretation remain separate pending gates.
