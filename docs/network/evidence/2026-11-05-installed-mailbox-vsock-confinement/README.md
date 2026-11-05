# Firecracker mailbox guest confinement checkpoint

SpiralCore issue [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43) and
MVM issue [#4629](https://github.com/tinylabscom/mvm/issues/4629) track the
installed mailbox. This packet records the guest transport binding and the
first native launch attempt on Wick.

The original transport failed with `EPERM` under the exact MVM Vsock
seccomp program. The replacement uses an exclusively owned nonblocking socket
and finite `ppoll` waits. The shared transport keeps its existing framing,
absolute deadlines, cancellation and counters. Full close drops the owned
descriptor. The signed mailbox witness selects `Vsock`. Its FlowMux control
continues to select `Standard`. The guest checks that filtering is active and
that opening an IPv4 socket is denied before it reads the private fixture
bootstrap or dials the fixed mailbox destination.

Four component checks passed, including exact encrypted Call, Reply and Cast
with receipts, idle read expiry, backpressured write expiry and descriptor
close. The confinement fixture uses a connected Unix socket pair with the
actual guest filter installed in the calling thread. The static Linux x86_64
guest and composed host test binary were rebuilt. The source patch is relative
to the prior private installed-mailbox composition and applies in reverse to
the verified test source.

The first one-guest attempt ended at canonical managed-resource admission.
The 160 MiB startup workspace request did not fit alongside the other tenant
reservations. Firecracker did not start. The attempt is a negative admission
result and supplies no native payload, wire-byte or timing measurement. The
exit and runner wall time refer to the failed fixture setup.

The 14 MiB workload image reached Firecracker under a 133 MiB startup
reservation. The offline allocation probe measured 139,114,148 bytes for
attachment and 139,078,841 bytes for runtime verification at peak. The
24 MiB image measured 174,200,570 and 174,165,263 bytes. These are conservative
Rust allocation requests with realloc overlap, using the resident's compiled
dependencies. They do not measure RSS or whole-installation capacity.

The guest exited before exchange. A fresh diagnostic replay returned fixed
exit code 21 from the effective confinement check, before reading bootstrap
or dialing. Firecracker boot configuration contained `mvm.seccomp=vsock`.
The signed overlay's agent hash is recorded in the results. The original
stop returned the resident's Retired state before the test propagated the
guest error. The successful-exchange cleanup and ledger oracle was not reached.

The next gate is qualifying a guest runtime that enforces the signed tier,
then running the native exchange, exact readback and complete retirement
oracle. Production cancellation, scale, WAN and FIPS qualification remain
open. This packet has no native transfer measurement.
