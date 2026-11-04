# Installed telemetry disk ownership

Part of the approved installed-service and aggregate-budget work in #43/#49.
This contract retains canonical MVM telemetry policy and introduces no wire
format, guest permission, alternate telemetry path or deletion authority.

The installed endpoint projection's cache allowance is TOTAL disk capacity C.
The canonical collector requires R =
`mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP` (currently
8 MiB). Logical key/status/catalog sidecars require the canonical Core
`INSTALLED_TELEMETRY_SIDECAR_BYTES`, K=32 KiB. Disposable codec capacity must be
checked C - R - K. A profile with C < R + K
must refuse before changing issuance state, reserving endpoint resources, or
creating a disposable namespace. Spare aggregate quota cannot repair an
undersized original projection.

The actual retained owner must hold R + K in the SAME installation/tenant ledger,
pin the original canonical state directory and fixed records/key names, and
survive endpoint/VM retirement. The disposable inventory must receive C - R - K;
its retirement may refund that disposable allowance only. A retained owner
must prevent generic runtime cleanup from deleting its files. A stopped VM,
drained codec, successful transient cleanup, or `destroy_on_exit` alone is not
audit-record deletion authority. Global completion must remain Busy until an
actual durable handoff or canonically authorized retirement resolves that debt.

Alternatives considered: treating all C as disposable loses retained data;
charging another R + K in addition to C double counts and does not own the files.
Use the checked split and distinct retained owner. Durable catalog/restart
adoption must close before activating the installed native profile. No reset,
zero-usage exemption, or new resource budget is allowed.

The first implementation increment is the pre-effect capacity refusal. It
does not by itself establish retained-file ownership or repair the inventory
split. Its focused regression uses the actual parent preparation API and
asserts unchanged issuance, aggregate usage and installation directory names.
The later retained-owner component implements R + K and the checked split;
its [recorded checkpoint](../network/evidence/2026-11-04-retained-catalog/README.md)
does not establish child-producer coupling or restart adoption. All remaining
ownership/lifecycle increments are required for release.
