# Resident portable startup reservation

This development checkpoint starts from the resident and retained-storage
branch at `c8ecf243` and pins MVM at
[`2bbb7c79`](https://github.com/Casey-allard/mvm/commit/2bbb7c795a55dc9767fd44c56cb4740f839db972).
The resident and its sibling mailbox adapter use the same MVM source. The
adapter takes the revocation changes already reviewed for SpiralCore main at
[`71c82810`](https://github.com/R4-Laboratories/SpiralCore/commit/71c82810).
The historical codec pins in this branch and in the MVM fork are retained.

The installer provides the full signed bundle pin, exact archive length, and
separate workspace and storage amounts. The same tenant ledger charges
archive bytes plus workspace as host memory, and archive bytes plus expanded,
scratch, snapshot, and cache allowance as storage. The exact archive length
controls MVM's archive reads. The additional amounts are installer estimates
and require separate workload and allocation qualification.

The reservation moves from held to active to complete on the original grant.
Attachment checks can inspect the held quote without changing it. Runtime
completion enables endpoint issuance. A portable preparation requires the
armed launch owner, and the resident stores that owner before polling the
canonical signed mapped launch. An unused-preparation receipt can release the
original grant after a pure refusal.

The existing native exit and endpoint receipts do not account for the
retained portable generation or shared image cache. The resident keeps the
startup charge and refuses final retirement until that disk has a verified
disposition. This component checkpoint does not boot a guest or qualify
startup peak memory, retained-storage recovery, platform parity, or WAN.

## Reproduce on Linux

```sh
umask 077
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features canonical-host,linux-host --lib installed_parent:: -- --test-threads=1
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --features live,conformance,hardware-sha256 --test canonical --test receipt_send --test service
```

On Wick, the locked resident run passed 25 tests. Two fixtures requiring
protected canonical helper binaries were skipped. The locked adapter run
passed 33 tests across the three targets. Cargo metadata resolved all MVM
packages to `2bbb7c79`. The resident test target emitted the two existing
unused-field warnings in retained recovery.

The baseline regression failed to compile because the quote and funding APIs
were absent. The final tests check the exact grant charge and phase transitions,
refusal of conversion without the armed owner, and retained disk accounting.
The mapped resident test also checks budget exhaustion and drives the canonical
Client's unsigned-request refusal to an unused-preparation refund. Both
fixtures run without native dispatch.
