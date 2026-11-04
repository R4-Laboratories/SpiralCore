# Native component workflow and publication review

Initial source-only review: **HOLD** for one workflow context error in
`.github/workflows/spiralcore-installed-supervisor.yml`, SHA-256
`fcb23d6d90af7026593a450b116d5c018fa71d805f0e942258e4fa11f0080ef7`.
`jobs.native-parent.env.CARGO_TARGET_DIR` references `runner.temp`, but the
[official context availability table](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability)
does not permit `runner` at job-level `env`. It permits that context at step
`env`. Move only this setting to the test step's `env`, matching the existing
installed-owner workflow. The reviewed initial candidate is retained as
`validation/installed-supervisor-workflow-first.yml`; no hosted failure or
actionlint execution is claimed here.

The remaining inspected design is sound: four native Linux/macOS architecture
component rows, native rustc-host check, exact PR-head checkout/assertion,
read-only token and no persisted credentials, immutable checkout/upload action
pins, Rust1.97.1, two Cargo jobs, dedicated runner-temporary target, release
locked all-target tests with serial process fixtures, before/after source
digests, ten-minute job bound, fail-fast disabled and always-uploaded compact
evidence. This workflow starts no VM or network fixture. macOS Intel is an
exact-file component role, not a native MVM backend qualification. Harness
dead-code warnings are retained; no blanket allowance or replacement of the
existing strict production Clippy gate is introduced. A positive eleven-test
resource assertion would additionally protect against accidental future
test-target omission; the inspected current harness includes the actual file.

The tiny public wrapper SHA-256
`3087c43422e28b3066fdd9c300b05a7ef4d967e476d31df127051f95fd550090`
includes the same eleven resource tests against the public crate under its
native-host cfg, without duplicating the ledger or enabling a Windows/native
MVM claim. Its actual root-executed Apple Silicon macOS result is retained:
eleven PASS, zero failed, exit0,8.24s compile/0.63s execution. Linux wrapper is
NOT_RUN. These results extend the prior source review without changing older
33Linux/32Mac standalone or143Linux product counts. No new build/test execution
was performed by this reviewer.

Final delta review: **PASS**, workflow SHA-256
`87f6ea2ce97ea1a26df703c56a89c3df8bf1e3702711e64ee155c3d8f9eac659`.
The root moved `CARGO_TARGET_DIR` exclusively to test-step `env`, resolving
the context-availability blocker. Added exact18-unit/3-lifecycle/11-resource
success-summary guards require zero failed, ignored and filtered entries;
the serial all-target execution, original native-death target behavior,
source checks and all other gates remain intact. Hosted execution is pending,
so this is source/schema review PASS rather than a CI result. The initial
HOLD and original candidate are preserved above. This review and the separate
code/Rust/security review are frozen for the root's publication branch.
