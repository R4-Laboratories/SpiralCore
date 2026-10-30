# Artifact member custody correction

This additive record corrects an unsupported custody claim. The shell loop used
for five `compressed_localized_32` archives, the earlier `cold_random` preflight
failure archive, and five then-unsealed `cold_random` archives omitted its input
redirection. It therefore visited zero members, wrote an empty member-hash manifest,
and still claimed 53 or 56 comparisons. The original receipts, empty manifests and
frozen packet checksums remain unchanged. Their claimed member comparisons are
explicitly superseded by this correction.

The authenticated API digest/size checks, ZIP integrity checks and exact member-name
comparisons ran independently and had passed. They did not establish the omitted
per-member comparison. The parent's original `SHA256SUMS` checks likewise proved
internal hash consistency, including the empty files; they did not prove that each
manifest contained its expected 53 or 56 entries. Future acceptance requires exact
entry counts and a qualified per-member comparison in addition to checksum checks.

The bounded repair uses the existing Bash, unzip and SHA-256 procedure with explicit
input redirection, a visited-member counter, a nonempty output requirement, and exact
expected/visited/manifest-line agreement. It also checks duplicate names and equality
of the archive, supplied and retained-file member sets. The clearly labeled minimal
historical reproducer returned success with zero visits (RED). Runtime qualification
then rejected empty lists, missing listed members, wrong expected counts, wrong file
content and missing retained files; a known two-member fixture passed (GREEN).

`correction.json` records the affected scope and the actual recheck of **613 members
across 11 archives**: ten archives with 56 members and one with 53. Each actual ZIP
member's SHA-256 matches the retained remote file, and each rechecked archive SHA-256
and size matches its original authenticated API identity. The 273 compact remote
files already retained in the six frozen packets were also compared byte for byte
with their local originals. The 45 earlier populated manifests each have exactly
56 lines; `earlier-manifest-counts.json` records that count audit without claiming
an additional raw-archive comparison for those earlier files.

`history/` preserves the original defective manifests, custody receipts and API
responses. `rechecks/` contains the actual member inventories, new comparison
receipts and explicitly corrected custody records. The cold case's first final
packet uses these corrected records; its earlier unsealed local drafts remain
preserved here. No frozen packet, ZIP, PCAP, measurement row, canonical plan, capture
accounting record or scientific analysis was rewritten. No performance measurement
was rerun because of this custody defect.

The checker is scoped to these retained archives whose digest, size and inventories
are already authenticated. It does not independently enforce a decompressed-output
time/byte cap on unzip and is not presented as a general hostile-archive verifier.
The separate independent review uses direct ZIP metadata enumeration and streamed
byte comparisons rather than this checker’s input-list/hash loop. Its source,
qualification and results are retained alongside the author evidence when complete.

No hosted artifact cleanup or new measurement is authorized by this packet alone.
Parent acceptance requires both rechecks and the final cold-case packet to pass.
