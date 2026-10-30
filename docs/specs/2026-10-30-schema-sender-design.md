# SC-OPT-013: reduce compressed-schema sender CPU without changing bytes

The user authorized this optimization after SC-OPT-012: preserve the compressed
schema's byte savings and exact reconstruction, retain one receiver across
successive transfers, and record before/after CPU and durable elapsed against
ordinary zstd. Baseline is `97b01a1fd9086b284b2e25a9807c1b8a145564a2`.

Measure the existing public schema encoding, compressed schema encoding and
object hashing stages before editing production. The likely avoidable work is
repeated allocation, parsing of fixed keys and SHA-256 layout construction for
records matching an already validated template. A prepared-layout match can
verify fixed fragments directly and parse only variable primitive tokens. Use
fixed-capacity borrowed value storage; a mismatch retains the existing general
parser and fingerprint lookup. The first template is a bounded initial fast
path, avoiding a linear scan of all templates on hostile or drifting data.

Alternatives are allocation-only cleanup (less repeated work removed), or
changing zstd settings/stream representation (different byte costs and decoder
qualification). This milestone selects a direct exact-layout path only if the
baseline profile confirms schema encoding is material. It does not change the
format, zstd parameters, whole-object identity, sender selection, receiver cache,
authorization or durable publication. Both prepared and free schema APIs share
the same exact stream writer. Fit and decode behavior remain unchanged.

The fast path must enforce the existing record size, UTF-8 and primitive lexical
rules, complete input coverage, slot bounds, matching-work charge and literal
fallback decision. Empty input, empty-object templates, duplicate keys, spelling,
whitespace, escaped strings, malformed and binary input remain byte-exact. No
unchecked slicing or received executable recipe is added. A pinned old encoder
is an immutable parity oracle; every optimized intermediate and zstd output must
match it, not merely decode to an equivalent JSON value.

Keep the investigation bounded: one baseline stage profile, one implementation,
focused regression/parity tests, one full qualification and one paired persistent
campaign. The performance goal is at least 10% lower sender CPU on matching
~1 MiB batches with identical bytes. Report regressions on every control and
total CPU/elapsed independently; never infer energy or WAN speed from local CPU.
Retain an unsuccessful candidate and its findings if it does not earn adoption.

Reuse the existing isolated worktree and shared build targets. Keep at least
4 GiB free, disable incremental builds and remove only successful owned session
stores. Preserve original runtime/saves and historical evidence. The user has
already authorized implementation and publication; routine implementation
choices within this scope need no further approval pause.

The matching proof is local: artifact admission already validates the first
template's flat JSON skeleton; exact fragments plus the existing deterministic
primitive parser establish the same token boundaries and layout as the general
parser. Complete record coverage is mandatory. Index zero is first in any
fingerprint bucket, so a verified match skips no earlier collision candidate.
Only a successful match charges `2 * literal_len`, matching the old lookup.
An over-budget match emits a literal and terminates future matching; an exact
budget fit may still encode the current record. Unprofitable matches retain the
same strict size comparison and their original work charge. A failed speculative
match leaves the counter unchanged and runs the old path, adding at most one
bounded record scan. Controls must expose that possible regression.
