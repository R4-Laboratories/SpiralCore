# Development source checkpoints: 2026-11-04

The original pure/core snapshot is `ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`.
Its adjacent core-checkpoint.sha256 describes that frozen snapshot, including
its original then-stale canonical-host pin; it is not a hash inventory of later
integration commits. It remains useful provenance, not an accepted release.

The follow-up integration branch aligns Adapter/Access to MVM8b833aee and the
same immutable TransferCA source. Live now uses those exact Git sources and a
refreshed lockfile. The selected graph contains one Core/Contract/Transfer/
FlowMux identity without local path patches. Seven installed-parent component
tests pass in an actual locked release build; see
[the precise graph receipt](../network/evidence/2026-11-04-immutable-graph/README.md).

The checkpoint also preserves Small/Ordered, TLS/Mio, completed-tail, access and
installed disk/parent work in progress. Qualification differs by component and
source. It is not a complete installed MVM service, native/WAN or fleet release.
Newer shared child/factory/helper work continues after the pinned MVM snapshot.
The live GitHub issue graph #42 and its children remains the roadmap authority.

Code checkpoints exclude local runtime/sessions, tooling state, credentials,
Cargo targets, VM disks and payload stores. Reviewed/tested component milestones
merge to main; development snapshots remain clearly labeled until their actual
integrated acceptance gates have executed evidence.
