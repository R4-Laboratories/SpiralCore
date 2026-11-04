# Weak installed-parent registry review

2026-11-04. **Scoped Code/Rust/Security source PASS; no blocking findings.**
The reviewer authored the ownership addendum, not this product implementation
or its regression test. No builds, tests or native launches were run here.

Reviewed product: MVM `crates/mvm-hostd/src/host_lifetime_parent.rs`, SHA-256
`886ea1530a5a55fd3464f622a3ac08f7154d092f67de3a118a53f33b55af01c1`.
Reviewed actual-ledger regression:
`crates/spiralcore-mvm-live/tests/parent-retirement-harness/registry.rs`, SHA-256
`d6d436741cc9d2d548b7e242f0baa7d98adc14a0af08feb2a68e4d269b0f802d`.
Comparison basis is the strong registry in public MVM
`77752e1967d20caf8bad710de864a0ccd967f9e6`. The client unsupported-backend
fixture's only relevant ownership delta retains a local `provider` Arc and
passes its clone to install, rather than passing its sole temporary reference.

The static now stores `OnceLock<Weak<dyn ParentAdmissionProvider>>`.
`install` downgrades the exact submitted allocation. `require_installed` returns
an owned upgraded Arc or refuses absent/expired registration. Arc upgrade is
race-safe: success retains that same provider for the caller; a simultaneous
last-owner drop can only produce success-with-retention or refusal. There is no
raw pointer, unsafe code, reconstructed provider or replacement allocation.
OnceLock remains set after expiry, so both same-provider and foreign-provider
reinstallation fail; expiry does not reopen admission.

The three production callers retain their authority boundaries. Client launch
preflight and Hostd plan preflight use availability only, not a permission
cached across later work. `bind_current` holds the upgraded Arc through its
synchronous actual-admission binding and permit return. If the resident drops
the provider after an earlier availability check, the fresh binding fails
closed. Existing original-plan/anchor and private-grant binding checks remain
unchanged; weak registration does not create authority or cancel a live permit.
The resident must still close admission before releasing its strong provider.
The registry itself is not a revocation mechanism for already-retained calls.

The regression uses an actual InstalledParentProvider and original managed
ledger. A scoped upgraded Arc keeps the real 4096-byte reservation alive after
the resident references drop, and global finish must refuse. Dropping that last
scoped Arc must expire the original Weak and reduce the actual metadata usage
to zero. Availability then refuses. A distinct original installation/provider
cannot reinstall into the consumed static; both original supervisors then
attempt their genuine final finish checks. This tests real bookkeeping and
holder exclusion, not a mocked counter or a caller Boolean. It runs in its own
integration-test process to isolate the intentional install-once state.

Root reports an actual public777 RED after the scoped call dropped, at
`registry retained the last original provider`; the initial 4096-byte retention
assertion passed. That report is kept distinct from this source review. There
is no successor GREEN or positive native retirement claim in this review.

Nonblocking API clarity: because `install` still accepts an Arc by value,
callers must retain another strong owner for the intended registration lifetime;
passing the sole temporary now expires immediately, fail-closed. The contract
explicitly states this requirement and the reviewed fixture was corrected. It
would also be useful to state it on the public install rustdoc. No registry
charge exemption, counter reset, uninstall/reset API or finite-lifetime restart
policy was introduced. Other factories and incomplete native owners still have
their independent real-drain obligations.

## Published successor comment delta

Reviewed immutable MVM `f708539f0c2317acf2e74f8f7833d6072a5abafe` against
777. Its two changed files are the reviewed weak registry and retained-provider
client fixture. Final registry SHA-256 is
`678f8336b19779298488471316eccf328cb63264c659d30fd38369cc37ac1a3a`.
Removing only the two added install rustdoc lines reproduces the originally
reviewed registry hash `886ea153…af01c1`. Thus this successor adds documentation
of caller strong ownership and permanent registry consumption, with no further
executable delta. The source PASS stands; the nonblocking clarity suggestion is
resolved. Public-graph GREEN and publication remain separately observed gates.
