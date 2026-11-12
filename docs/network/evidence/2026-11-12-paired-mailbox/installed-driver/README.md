# Installed driver for an addressed guest-to-guest Cast

Two independent persistent SDK launches on the same Linux host each reserve one original worker. The test controller uses A’s original mailbox authority for the source scope and B’s original authority for the cross-address Send and B’s Receive grant. The test controller prepares the public guest boot records and private helper roles together. Only its explicitly named pair may use the compact development profile. Each signed plan permits 65,536 bytes of host workspace for the two 32 KiB reservations within the shared 1 MiB coordinator budget. [Original admission and service](https://github.com/Casey-allard/mvm/tree/5ae2cdc4977e12a574a5baa2e4f0522c3f7a1ae7/crates/mvm-hostd/src/mailbox_process)

Before either guest starts, both original signed boot files must match the bytes prepared for the two roles. The SDK checks their file and directory identities and synchronizes the records. A missing file, a same-length replacement, a symbolic link, a hard link or expiry prevents publication. [Publication checks](https://github.com/Casey-allard/mvm/blob/5ae2cdc4977e12a574a5baa2e4f0522c3f7a1ae7/crates/mvm-hostd/src/mailbox_launch/cast_enrollment/publication.rs)

The driver supplies fresh binary input only to A. B receives its own fresh 32-byte challenge through an ordinary workload call. A returns its local message identifier. B must independently return every input byte followed by its challenge. The coordinator returns transient ingress acceptance and handles the signed credential and ciphertext. The driver records application CPU, host driver CPU, elapsed times and per-original guest stream counters. A destination journal receipt has its own acceptance gate. [Driver and reproduction](../../../../../tools/installed-mailbox-witness/CAST.md)

The Linux driver and original coordinator API typechecked together. Focused component controls cover profile admission and two-sided publication. This packet contains no two-guest transfer measurement. The real Firecracker acceptance run has a separate record when executed. [Component results](results.json)

Linux and macOS passed the installed SDK directory and signed-publication checks under both group-cooperative and private umasks. [Filesystem results](private-state.md)

The addressed witness accepts verified role image files and records coordinator deadline failures before waiting for the same original process. [Materialized image and custody results](materialized-driver.md)
