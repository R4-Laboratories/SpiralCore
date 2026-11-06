# Installed object campaign on a Firecracker mailbox

A Linux x86-64 Firecracker guest ran the object campaign directly under
the booted MVM agent and its signed Vsock policy. The private portable
bundle carried the fixed Raw campaign plan and enrollment pins. The
host used the original resident broker and service. There was no guest
NIC. This test used one authenticated V2 carrier throughout.

The campaign completed 20 canonical Calls and seven publications
totaling 851 B of logical object data, including repeat cases. The
original owner retired the guest before the host independently reopened
and compared every publication. It then removed the exact private
store, verified its absence and released the store reservation. Guest
and staging charges returned to zero, and no test listener or
Firecracker process remained. The separately retained runtime stayed
charged.

| Measurement | Guest | Host |
|---|---:|---:|
| Bytes sent, including authentication and framing | 211,798 | 194,906 |
| Application frames sent | 40 | 40 |
| Campaign CPU after local authentication | 64.455 ms | 57.484 ms |
| Campaign wall time after local authentication | 145.043 ms | 99.573 ms |

The original guest lifecycle took 2.762 s, and the supervised run
took 2.977 s. Outer host peak RSS was 190,392 KiB, including fixture
preparation. The directional byte counts reconcile. The tiny fixture
exercises protocol behavior and does not establish compression savings.

The initial object run was refused before boot because startup and the
additional codec credit overlapped the 256 MiB tenant resident budget.
The successful run kept the 4 MiB cache and 4 MiB staging reservations
through readback and removal, then acquired the 4 MiB codec reservation
only after Runtime released startup workspace. The qualification-only
broker reserves 2 MiB of metadata and 128 identities per scope including
64 control identities. Existing host and tenant caps stayed unchanged.
Managed reservations and process RSS measure different scopes.

The result covers one short installed Dev witness. Object-carrier
replacement, sustained Calls across expiry, concurrent fairness,
WAN performance and production security qualification remain open.

[Prior authenticated carrier component](../2026-11-05-authenticated-campaign-carrier/README.md)
