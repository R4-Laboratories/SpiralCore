# Authenticated mailbox runtime compatibility, format 1

Status: proposed for bounded implementation review. Owner: canonical MVM runtime
producer, verifier and admission. Tracking: [SpiralCore #268](https://github.com/R4-Laboratories/SpiralCore/issues/268),
under #43 / #42. This changes runtime compatibility metadata, not the SpiralCore
wire protocol, MVM mailbox envelope, authorization or receipt meanings.

## Problem and required result

The verified rc.3 runtime candidate at `ffd90f895d7080629eceb16095d1ca81c0a49b95`
accepts at most 2048 bytes of mailbox reader boot metadata. Contributor source
accepts 8192 bytes and adds directed-journal, object-pair and sender-ordinal
semantics. Native trial 12 generated 2553 bytes, reported control readiness,
then failed the source workload. Even a shorter binding with an unsupported
field is incompatible because reader decoding denies unknown fields.

The selected runtime must support the actual admitted mailbox operation and
exact minted boot encoding before identity-drive publication or VM startup.
Artifact authenticity and compatibility remain separate checks.

## Existing authority and additive declaration

Add optional `mailbox_reader_v1` to the existing per-architecture signed
`RuntimeAssembly`, using a closed format-1 declaration: nonzero maximum encoded
boot bytes and a closed set of `directed_journal_v1`, `object_pair_v1`, and
`sender_ordinal_v1` capabilities. Sender ordinal requires directed journal.
Unknown formats/features and inconsistent declarations refuse. A larger size
limit grants no semantic feature by itself.

The enclosing schema-2 manifest and all existing assembly fields retain their
meanings. Omitted metadata preserves existing legacy reader operations with a
2048-byte bound and grants none of the new extension profiles. Existing closed
older consumers refuse the added declaration rather than silently downgrade.
The supported host/runtime matrix must document that refusal.

Read the declaration only through existing verified release evidence and its
retained archive/architecture/artifact selection. Preserve it in the existing
verified snapshots. A deserialized declaration or mutable cache file is not
authority. Do not create another signer, version/hash allowlist or operator
capability override. Existing source and legacy runtime provenance remain
distinct from authenticated release provenance.

## Producer and admission

The canonical producer may emit the new declaration only for the captured
current guest build after verifying the selected compatibility/reader inputs
against the producer's compiled inputs and existing build fingerprints. Pure
assembly of arbitrary captured bytes must not label an old guest with newer
host constants. Keep generation in MVM; no new guest work enters mvm-images.

Derive required profiles from the actual admitted grant and boot binding:
retained directed incoming/outbound journal requires directed journal, the
object selector requires object pair, and outbound sender settlement requires
sender ordinal. Check binding-to-plan continuity. Refuse unsupported profiles
at the existing verified runtime launch gate. After minting the actual binding,
check its exact serialized length before writing the drive or persistent boot
record, endpoint startup or native execution. Carry verified provenance through
that operation; estimates or local host limits cannot substitute for the check.

All existing signed-plan, capability, lease, original owner, resource and audit
checks remain mandatory. A compatibility refusal creates no VM/helper and does
not report readiness. No ordinary, unsigned or opaque-traffic fallback follows.

## Qualification and delivery gates

Cover legacy baseline success, missing extension metadata, unsupported semantics
that fit under 2048 bytes, exact size boundary and one-byte overflow, wrong
architecture, unknown/malformed declarations, changed artifacts and untrusted
cache/caller declarations. Cover stale producer inputs and snapshot continuity.
A pre-dispatch regression must prove refusal before VM/helper creation. Execute
required shared-type gated compilation and the final workspace checks before
calling this feature qualified.

A compatible deployment additionally requires newly built guest binaries,
deterministic declarations, an intact archive/checksum and accepted canonical
release authentication, followed by actual feature-specific guest and paired
transfer/recovery tests. The old signed candidate stays immutable. A contributor
source build is useful build evidence but does not satisfy release authority,
WAN/platform acceptance or the complete upstream submission.
