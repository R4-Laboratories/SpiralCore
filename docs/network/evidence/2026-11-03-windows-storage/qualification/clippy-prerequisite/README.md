# Runtime passes; Clippy installation prerequisite fails

At source c803cd4b, native Windows backend tests both pass (including real
junction refusal); the same runner's canonical client, outer storage and exact
publication/restart targets pass before them. The workflow job remains failed:
its newly required Clippy invocation finds no cargo-clippy.exe in the pinned
minimal1.97.1 toolchain. This is a workflow prerequisite failure, not a native
runtime or product test failure. Original excerpt is retained without changing
that result.

Independent session_debug Phase1/infra review confirms the install command lacked
--component clippy and the native log identifies the same exact missing pinned
component. Root adds that component to the existing install command; no test,
lint, source, platform or nonzero-count gate is removed. Fresh entire native
matrix qualification remains required before the job may be called passed.
