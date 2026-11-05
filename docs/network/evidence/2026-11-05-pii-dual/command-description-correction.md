# Append-only command-description correction

2026-11-05 UTC. The original hashed packet and command.txt files are preserved unchanged. Independent review ab83da7c5b8cb49a9d7a7b5ca56cabc6a1c548d66d31a20733dba54493505bf2 identified descriptive command strings which do not completely reproduce the executed script's environment/configuration.

- run-aux command.txt describes the inherited pii-capacity configuration path and omits CARGO_PROFILE_RELEASE_LTO=false. The actual retained run-aux.sh executes the dual-root audit-flow-intermediate.toml and sets CARGO_PROFILE_RELEASE_LTO=false. Both old Aux60 RED and new Aux60 GREEN use that same release/LTO-off setting.
- run-core command.txt omits the explicit LTO-off environment and leaves a root placeholder. Use the retained executed script, source maps and actual results for execution context.

This correction changes no source, test result, archive member, historical receipt or qualification scope. It does not establish the final coherent SDK graph. No rerun was needed to correct these descriptive strings.
