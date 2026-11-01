# Bounded packet compression selection

Starting source: `4bce7efa218bfd426116107af8c907254009ac1a`. The existing packet
encoder always attempts the shared whole-object zstd profile in `Zstd` mode,
then sends Raw when the compressed body is not smaller. That profile is actually
level 3, window log 25, long-distance matching enabled, zero worker threads,
content size and checksum enabled. The retained gateway specification already
says level 3. Parameters are prepared once, but compression, output allocation
and frame validation occur for every packet. The prior mixed local gateway
campaign saved bytes while increasing CPU and completion time; it did not isolate
per-flow wire bytes or prove a WAN speed gain.

## Decision and alternatives

Preserve `Raw` and `Zstd` behavior. First add an opt-in **FastZstd** profile using
level 1, window log 11, long-distance matching disabled, zero workers, content
size and checksum enabled, and dictionary-ID emission disabled. Reuse one CCtx
and a bounded output buffer per encoder. The existing decoder and `SCGPKT01`
wire format remain unchanged. Retain exact length, IP validation, SHA-256 digest,
fragment/reassembly, replay and authorization boundaries.

An opt-in **Adaptive** mode uses that same FastZstd candidate. It may skip some
compressions only after observed no-saving results for that flow. It is compared
directly with forced FastZstd; comparing it only with the heavier historical
profile would confound parameter tuning with selection. No mode is promoted as a
new default by this patch. If tuning alone supplies the useful gain, retain
Adaptive as experimental or leave it disabled in deployment.

Alternatives considered: a stateless entropy/repetition heuristic can miss newly
compressible content; a negative cache without a current-packet guard recovers
too slowly after changes; a proof based on the existing first-block zstd probe
does not apply to packets capped at 1280 bytes. A full second compression to pick
the smaller old/new result defeats the opaque CPU objective. We therefore use a
small measured profile plus conservative advisory flow state, with explicit
byte-regression tests rather than a universal compressibility claim.

## Interface

`PacketMode` adds `FastZstd` and `Adaptive`; existing variants are unchanged.
The service names are `raw`, `zstd`, `fast-zstd`, `adaptive`. Existing `zstd`
configuration continues selecting the historical profile. No packet wire codec
number is added: both new modes emit existing codec 0 or 1.

`PacketEncoder::counters() -> PacketEncoderCounters` returns copied public u64
fields: `packets`, `compression_attempts`, `compressed_packets`, `raw_packets`,
`adaptive_skips`, `reprobes`, `guard_probes`, `flow_evictions`. `packets` and output
mode counts advance only for successful encoding; attempts count actual compressor
calls, including a call that later fails. Other counters describe selector
decisions. Counters saturate and confer no authority. Exact framed bytes remain
the sum of returned fragment lengths, not an estimate derived from these counters.

The FastZstd context has a conservative 2 MiB workspace allowance and one output
buffer reserved to `compress_bound(MAX_IP_PACKET)`; context size is checked at
construction and after compression, and an oversized context is dropped. This
is separate from existing packet reassembly and transport queue budgets. The
historical encoder's allowances and behavior remain unchanged. The FastZstd
sender checks exact frame content size and single-frame length; the existing
receiver performs its full checked decode before exposing an IP packet.

## Adaptive state and decisions

State is private to one `PacketEncoder`/authenticated peer. It never changes
reconstruction or sends cache references. It holds at most 64 flow entries with
deterministic FIFO eviction; replacing an entry resets it to probing. A flow key
is IP version, exact source/destination addresses, TCP/UDP protocol and both
ports. Only unfragmented IPv4 TCP/UDP and IPv6 TCP/UDP without extension headers
are classified. TCP data offset and UDP length are checked before identifying
payload. Unsupported shapes, empty/control packets and payloads below 256 bytes
always attempt FastZstd. This parser is advisory and grants no routing permission.

1. New flows probe. Enter negative state only after two consecutive actual
   FastZstd results whose encoded body is not smaller than Raw.
2. For each eligible negative-state packet, scan the full payload at four-byte
   steps for repeated eight-byte words, with a fixed 128-slot direct-mapped table
   and exact word comparison. A repetition triggers compression immediately and
   resets the skip count (`guard_probes`). No byte samples from another flow are
   consulted. Hash collisions may miss a repetition; they cannot manufacture an
   exact word match.
3. Without a positive guard, skip at most seven consecutive packets, sending Raw.
   The next packet is a mandatory probe (`reprobes`). Any actual byte saving
   resets the negative streak and resumes compression for subsequent packets.
4. Failed encoding does not publish data or create receiver-cache readiness.
   State is advisory only; an expired/evicted history entry costs extra probing,
   never fidelity. Decoder behavior is identical in all four modes.

This selector **cannot guarantee byte equality with forced FastZstd**: entropy
coding, unaligned or long-period structure and deliberately changing content may
evade its repetition guard. Probe frequency bounds a consecutive skip run, not
the aggregate byte penalty for an arbitrary adversarial sequence. Retain vectors
that demonstrate those misses. Under no circumstances describe the heuristic as
an information-theoretic proof, TLS plaintext discovery or cache consensus.

## Evidence and promotion conditions

Freeze baseline source and executable identities before production edits. Compare
identical ordered IPv4/IPv6 packets through Raw, historical Zstd, FastZstd and
Adaptive, with fresh contexts between cases and prepared state reused within each
case. Include opaque, repeated patterned, new patterns, mixed flow interleaving,
same-flow opaque-to-pattern transitions and adversarial changed patterns.

Every decoded packet must match the original bytes and length. Record body and
framed bytes, selection counters, encoder/decoder CPU and wall times, and paid
setup separately. A compression-profile-only diagnostic holds allocation and
validation constant to isolate parameter effects. It is not network evidence.

Targets, not prior claims: preserve the full patterned byte saving; keep mixed
framed-byte excess at or below 1% versus forced FastZstd; seek at least 20% lower
opaque encoding CPU than historical Zstd. Also report Adaptive versus FastZstd
CPU and bytes, every transition miss, and any failed target. Passing local codec
targets alone does not establish total transfer speed. The subsequent registered
constrained-network campaign uses identical authentication, policy, traffic and
impairment settings across four arms and counts complete both-direction traffic.
No default change or broad performance claim follows before those results are
reviewed. Zero fidelity loss remains unconditional for successfully decoded
packets; UDP delivery remains a separate property.
