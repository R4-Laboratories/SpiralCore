# Additive artifact publication API after immutable component30

The parent approved this additive continuation after publishing component30 as
SpiralCore a521ac73910ee9c7f421a166a639bdc62024e499. That source and its30-test
receipt remain immutable history. No MVM dependency may pretend this new API
exists in that revision; a new tested commit/pin is required before final use.

MappedStream gains seed_base_until(Vec<u8>, Instant), seed_schema_until(Instant)
and seed_checkpoint_until(Instant), each returning the existing core Stats.
They call the existing exact DuplexWriter *_until implementations under the
same original writer-lock/operation budget. An expired supplied Instant refuses
before invoked I/O; partial progress and interleaved replies do not refresh it.
The supplied future deadline must fit within this stream's configured bounded
operation timeout, maximum30s. Every operation checks current scope authority.
Existing convenience seed methods keep their current bounded timeout behavior.

Canonical factory bootstrap may perform at most one schema, one checkpoint and
one base publication, with existing shared object bytes/frames and Published
verification. Every step receives the SAME original setup deadline including
TCP connect, TLS, controller and codec preparation. A failure fences/cancels the
stream; no artifact failure can be treated as receiver readiness. Artifact
publications never appear as application bytes or replace a logical rolling base
except the existing explicit seed_base operation. Warm/Runtime/Combined remain
existing core modes, not new mapping codecs. Byte, deadline, failure and paired
real-socket artifact/half-close assertions remain mandatory.
