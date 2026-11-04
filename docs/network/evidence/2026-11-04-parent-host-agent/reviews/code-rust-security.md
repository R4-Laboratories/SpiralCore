# Original parent/helper bridge initial source review

CODE/Rust/security review,2026-11-04, source-only. Shipping profile has no
production constructor and native/aggregate qualification remains open.

Reviewed host_agent.rs351641f943d3ec4c2cd84190434d2c4cb0ee80d3e36a96a4807467e299b7da16,
host_agent_tests.rsffe5e184ade0a27461eb96c2e539b778476bb9ff57f7a1f9e8675359de53fd80,
installed_parent.rsb9cf1251cba9da9cd3fa0bf86800c4be7a0bb2fdcb5cf66faccb9a5fef2ca43c,
API contract3c3e77d45449ddf3e4679f7b7fa4b0821e6de6ec68200d31b3ada9a60040cc73.

HOLD before activation: RegistrationBinding::check_paths pins only the VM state
directory. Canonical vm_vsock_port_socket_at calls vm_socket_dir_at, which may
select a distinct shortened socket directory for a long state path (canonical
config.rs860–875/927). Replacing that separate socket parent can leave the
original state inode unchanged and pass this check while redirecting the broker
bind destination. Pin and recheck the actual canonical broker-parent directory
under trusted provisioning, or explicitly refuse that unsupported profile until
the hook exists. Required focused case: long state path selects short parent,
then independently replacing that parent must refuse before registration I/O.
String equality is insufficient for its filesystem identity. This finding was
sent to Root immediately; it does not expand the audit outside this bridge.

The remaining reviewed ownership shape is appropriately conservative for this
component: one private profile attempt; reserve before holder/spawn; weak parent
and anchor avoid an Arc cycle; actual helper owner survives typed start failure;
uncertain Drop keeps owner and reservation; positive helper shutdown still
retains credits through outstanding registrar aliases. Parent/process/filesystem
operations run outside registry/helper ledger locks. Complete canonical binding
includes original anchor identity and registration facts. CapabilityBinding is
only bounded service/verb plus its32-byte descriptor digest, so the16-entry
pre-copy gate supports this narrow copy path. No speculative schema-copy issue
is reported. Actual runtime bind_config/consumer wiring, current helper model,
failed-start controls and native cleanup remain qualification gates, not proof
provided by these source checks or the three refusal fixtures.

No build/test was run for this review, and no source was edited. Final review
must use the corrected exact source and actual executed evidence; this initial
record is not release or merge approval.

## Reviewed successor and actual component evidence

PASS for the frozen component checkpoint; no remaining source blocker in this
bounded bridge review. This is not shipping-profile, native VM or merge approval.
The prior HOLD remains attached to351641..., and its actual0PASS/1FAIL short-path
countercase is retained at host-agent-path-red-corrected/countercase.txt (the
replaced canonical broker parent passed). The successor host_agent.rs
SHA-256 a40115a87f0a8f63431ad2d349a073c2c6599d262d401041d7d221aa642861d5 pins BOTH
original state and canonical broker-parent descriptors. Candidate opens and
losing candidate drops run outside the pin mutex. Named owner/mode/dev/inode
checks use the retained winners, including canonical shortening.

Successor tests SHA-256
08007f4da75d80e08d33e996679872df623cca931c9325d6387ac8e74ff85f55.
installed_parent.rs and API hashes above are unchanged. The actual six qualified
source/manifest/lock pins were read from host-agent-path-green/source-before
and every source-after-clippy line reports OK. Build, parent-tests, explicit
positive-tests and Clippy exits are all0. The scoped parent run passes15 entries,
with one functional case explicitly ignored and124 unrelated filtered; that
SAME functional case is then explicitly executed1PASS/0ignored/139filtered.
It proves real original daemon/signer startup and positive shutdown, reservation
retention through shutdown and owner Drop while registrar alias survives, then
refund only after its LAST alias drops and original supervisor finish succeeds.
No VM or registration was issued by that functional case. Its0.05s is fixture
wall time, not a throughput/native lifecycle result. The final protected Root
object hash a8fe5a2a900d4041c9a229dcdb9d0c9b672b1e1d7061c4804ccf3298de2aae2a
matched afterward. The two original protected f708 helper hashes match their
before/after records. Scoped resolved lib/tests Clippy -Dclippy::all passes0.

Raw records read directly from the owned Wick path:
/var/tmp/spiralcore-resident-parent-20261103/publication-closure/validation/
{host-agent-path-red-corrected,host-agent-path-green,host-agent-positive}.
Fresh-home key ENOENT was a fixture prerequisite; canonical key initialization
was added to the fixture, with no product fallback. New Hostd/client bind_config,
current-registration/audit-target and native consumer seams are separate local
work and are not retroactively qualified by these f708 component records.
