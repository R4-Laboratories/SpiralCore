# Reliable object-channel implementation receipt

Implemented against design commit `7403a87` on base `420e511`. This is the
author's implementation/test receipt, not independent source approval.

The object profile uses `spiralcore-object/1`, standard mutual TLS 1.3 certificate
validation and exact peer-leaf pins through the existing transport. It exposes
one owned bidirectional stream, disables datagrams and unidirectional streams,
and configures 1 MiB connection/stream receive windows and a 4 MiB send window.
Packet transport retains its existing profile and limits.

`ObjectChannel` reuses `wan::encode_batch/decode_batch` with a four-byte LE length,
20 MiB maximum body and 512-frame maximum. Zero followed by stream FIN is orderly
end; the opposite direction acknowledges with zero and FIN. Finish waits for the
explicit acknowledgment, and the responder waits for transport delivery. Normal
EOF before zero, trailing data, malformed framing, deadline and cancellation
poison the channel. Dropping an in-progress future leaves a non-reusable state;
later calls or channel destruction close the connection rather than resuming at
an unknown frame boundary.

## Recorded execution

All commands used native Rust 1.97.1 Cargo, two jobs, no incremental compilation,
and the existing shared target. Tests used real local UDP/QUIC, public ephemeral
test certificates and synthetic bytes. No network campaign or performance claim
is derived from their wall times.

| Receipt | Observed result |
|---|---|
| `transport-red.*` | Exit 101, only missing `bind_objects`/`connect_objects` APIs |
| `transport-green-01.*` | Exit 101, unrelated service schema-cap usize/u64 mismatch |
| `transport-green-02.*` | Exit 0, initial exact two-batch exchange passes |
| `transport-boundaries-01.*` | Exit 0, all eight object-channel tests pass |
| `packet-transport-regression.*` | Exit 0, all five existing packet-transport tests pass |

The known service type error was corrected at the root's explicit request to the
16 MiB `client::MAX_BATCHED_OBJECT` cap after verifying the build-error-resolver
role. No other service code was edited by this author.

The positive test sends two 1,201,212-byte SCWBAT01 bodies in each direction on
the same stream, exceeding its flow-control window while preserving exact bytes.
Other tests cover packet/object ALPN separation, wrong pins on both sides,
oversized/undersized lengths, truncated prefix and payload, malformed batch and
sequence gaps, fragmented writes, partial-prefix timeout, explicit cancellation,
dropped futures, empty/too-many outbound frames, missing end acknowledgment,
data after zero end and immediate client destruction after successful finish.

Logical counters distinguish batch bodies, four-byte prefixes and completed
batches. Failed partial I/O is not fully counted there; Quinn UDP counters and
independent capture account for network traffic. Quinn snapshots exclude later
close/drain packets and UDP/IP/link headers. Existing object clients retain
responsibility for response correlation and terminal-prefix reply semantics;
the channel does not incorrectly require equal request/response batch counts.

## Reviewed-source handoff identities

These hashes identify the source handed off for independent review; this author
does not independently approve them.

| File | SHA-256 |
|---|---|
| `src/transport.rs` | `8062eaca7e99b3a81f0d244a661d4d953a51a38911b4795ead1f94278db2f6ad` |
| `src/object_channel.rs` | `8730387643fe8e09c1cc5cf4ccff95448a5f448c5f8c16b0938ad480bb635041` |
| `tests/object_channel.rs` | `1b9760359c1aa4d4fe06e1be384d3426a75966afe5397e75663c202cc98f8ab3` |

## Independent-review correction

`mvm_boundary` identified that the public object-profile send path should refuse
datagrams explicitly even when a raw authenticated object peer advertises support.
The root authorized a focused regression and a three-line profile guard, matching
the existing receive-side guard. `object-datagram-guard-red.*` records the actual
behavioral failure; `object-datagram-guard-green.*` records all nine object-channel
and five packet-transport tests passing after the correction. No other production
transport change was made in that correction.

Final `transport.rs` SHA-256:
`144786cdcbe4cf103a41c6879e926c4e860a01af6dda27aa5b4f4e7fa990c19a`.
Final `tests/object_channel.rs` SHA-256:
`2de551b255a9496e24ff32bce880e1b8601f222b2e3009eda47b99a72a5b97b1`.
The object-channel implementation hash above is unchanged. These final files
were released back to the independent reviewer for closure.
