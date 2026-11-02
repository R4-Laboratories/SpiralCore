# Output preservation and operation correlation repair

Root-authorized bounded correction after independent review. The reproducer
confirmed that the previous error-report rename replaced an existing credential
or input at `stats_path`, admitted normalized aliases and replaced a hardlink
alias. `service-output-red.*` records those three behavioral failures. A separate
`service-correlation-red.*` records missing operation correlation IDs.

The service now validates normalized output, staging and lease paths against each
other, configured inputs/credentials and the store tree. It reserves both fresh
sidecar leases before any readiness or error report can be written. Existing
files, symlinks, hardlinks, staging files or leases cause refusal without writing
another report. An existing configuration file is protected by the same rule
that forbids replacing any pre-existing output target.

Initial publication atomically hard-links a synced, exclusively created staging
file to an absent output name; it cannot replace an existing entry. Subsequent
readiness updates require the original output inode and held lease, then publish
complete JSON atomically. The error path uses the same owned stats writer. Cleanup
removes only matching owned staging/lease inodes, and leaves published evidence.
Output parents must already exist. Stale reservations after a crash fail closed;
this implementation does not delete or recover another invocation's files.
As with the configured credentials/store, parent directories are trusted local
filesystem state; this does not claim immunity to an adversarial same-UID process
racing path replacement between metadata checks and rename.

The service also records `transfer_id` on each client operation and receiver batch.
Receiver IDs come from an already validated nonempty SCWBAT01 batch, whose frames
share the transfer ID. The existing integration test requires the operation and
receiver-batch ID sets to agree. This adds receipt metadata only, without changing
wire payloads or moving measured interval boundaries.

`service-output-green-01.*` records all seven service tests passing: preservation,
normalized/store/staging collisions, symlink/hardlink preservation, fresh owned
error output and cleanup, configuration bounds, real runtime/warm exact reopen,
scope/epoch rejection, and operation correlation.

Source released for independent review:

- `src/object_service.rs`: `ff5b5104561cc2bb846ecd18bda76b06af31ec8fa7f7c130d59d7d1d9a041b23`
- `tests/object_service.rs`: `2de75804206429db0dd13c9385241e426d4d56bbbd533027522d24b58ca4b1b1`

This is the repair author's receipt. Independent closure is assigned to
`mvm_boundary`; the earlier reviewer does not self-approve this correction.
