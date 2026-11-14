# Shared parent for the installed telemetry collector

The collector-only endpoint can now draw its execution and retained-record
credits from SpiralCore's original shared host/tenant budget. This is the parent
component needed by the persistent journal SDK successor; **complete native
shutdown is still unqualified**. The existing failed paired retirement remains
in [the native report](../2026-11-13-paired-sdk/native-retirement.md).

`InstalledParentProvider::configure_telemetry_only` selects canonical endpoint
profile 1 before the first grant. The profile admits four workers, one stream,
bounded metadata/stacks and 8 MiB + 32 KiB of retained-record space. It creates no
codec store and grants no plaintext, codec, carrier or staging allowance. This
resource subdivision does not establish host-blind telemetry: the collector
still processes telemetry under the canonical redaction policy.

Selection validates the actual signed deny-all policy, empty secrets/ingress and
original launch binding before parent registration effects. Cleanup requires the
original endpoint receipt and partition guard. Retention additionally checks the
actual VM reservation, pool, tenant, session and parent. Closing admission does
not authorize a refund or deletion. Object-TCP profile 2 keeps its original
store requirements; neither the wire protocol nor dependencies change.

## Executed Linux checks

The exact six source files and unchanged Cargo manifest/lock were SHA-256
compared with the qualified Wick snapshot, with no mismatches. Source and log
identities are in [parent-results.json](parent-results.json).

Using the declared MVM `7b22454077530e96146d68204b711de53e3d05a8`
dependencies, Rust 1.97.1 and `--locked --offline --no-default-features
--features canonical-host`:

| Command suffix | Result |
| --- | --- |
| `test --lib` | 166 passed, zero failures; test execution 14.66 s |
| `clippy --lib -- -D warnings` | Passed |
| `check --all-targets` | Passed; 36.66 s |
| `clippy --all-targets -- -D warnings` | Passed; 3.79 s |

The same production changes also passed 166 library tests in 15.36 s and strict
library Clippy with the contributor MVM `841fec80e` overlay. That overlay uses
the coherent SpiralCore `4ea98b9` dependency family; it does not qualify a
repository dependency update. One newer-SDK-only assertion was removed from
the private ledger test before the final declared-dependency run; that final
test file is the published one. The assertions for all actual ledger charges
remain.

The focused controls cover missing retained credit, unselected collector
refusal, shared-pool/tenant isolation, admission-closed retained ownership,
signed deny-all selection and rejection of unsupported policy before effects.
The initial unsupported-policy regression **failed** because selection had
been deferred too late; selection now uses the existing canonical signed-policy
projection before binding. The missing credit helper produced an expected
compile RED before implementation. An earlier overlay failed to construct
because its codec dependency lacked owned-store APIs; that is retained as a
dependency-construction failure, not a product test RED.

Code and security reviews independently found no blocking issue in this scope.
Builds reused Wick's shared target with two jobs, 200% CPU quota and 6 GiB memory
limit. No Mac build, payload corpus or additional guest image was created.

## Remaining gate

The persistent SDK must retain its original backend and launch anchor through
the journal stop transaction, deliver the positive endpoint retirement result
to this parent and seal retention before clearing journal state. Then the
actual two-guest transfer/shutdown must be rerun. These component checks do not
establish native retirement, WAN performance, fleet capacity or FIPS compliance.
