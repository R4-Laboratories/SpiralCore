# Persistent journal SDK: original backend and retirement custody

Contributor MVM [`0835ef0ad`](https://github.com/Casey-allard/mvm/commit/0835ef0ad)
implements the opt-in installed persistent journal lifecycle. It retains the
actual `Arc<LocalBackend>` and prepared launch anchor through admission, startup,
rollback and stop. The ordinary persistent journal API remains available with
its existing behavior.

The installed route uses the existing final admission gates and signed
configuration/image/runtime checks. It consumes dispatch before starting the
original backend and does not start a second ordinary mailbox helper. It
requires the admitted Linux/Firecracker collector-only shape and a funded,
authenticated canonical bundle workspace. Raw historical rootfs fixtures do not
meet that stronger startup contract.

Stop validates the original name, anchor, backend, journal boot, directories,
marker and observed kernel identity. It stops through the retained backend,
collects the original mailbox Child and obtains the original `OwnedRetirement`.
The parent callback must seal the endpoint's retained records before journal
marker clearance. The SDK holds the consumed retirement receipt across callback
failure and remembers successful parent finalization for retry. Readiness and
accounting rollback use that same journal owner and record successful stop,
allowing later collection without attempting a second VM stop.

## Executed checks

All 12 committed changed files were SHA-256 compared with the exact qualified
Wick snapshot after formatting, with no mismatch. [Machine-readable source and
log identities](sdk-results.json) retain the passing checks and earlier failures.
Rust 1.97.1, `--locked --offline`:

| Command | Observed result |
| --- | --- |
| `test -p mvm-client --features test-support --test persistent_journal_host_lifetime` | 2 passed; signature compatibility, no workload boot |
| `test -p mvm-client --features test-support --lib launch::persistent::journal` | 5 passed, zero failures; 0.10 s |
| `test -p mvm-hostd --lib mailbox_process::original_journal` | 29 passed, zero failures, 3 existing ignored tests; 1.78 s |
| `clippy -p mvm-client -p mvm-hostd --features mvm-client/test-support --all-targets -- -D warnings` | Passed |

The final bounded build/test unit took 94.962 s wall and 133.028 s CPU, with
reported peak memory 3.6 GiB. These are compilation and component-validation
costs, **not transfer CPU or latency measurements**. It reused the shared Cargo
target with two jobs, 200% CPU quota and 6 GiB memory cap. The local commit hook
initially started a Mac build; that owned process tree was stopped and its
102 MiB target removed. Commit used the repository's documented
`MVM_SKIP_CLIPPY=1` option because the recorded Clippy check ran on Wick.

Independent review found and fixed the earlier rollback/collection mismatch;
the final authority and rollback delta passed security review. The initial
attempt to clone `AnyBackend`, a mistaken Mock status assertion, an invalid
hostd feature selection and the oversized private error are retained as failed
construction/validation stages. No lint was suppressed.

## Acceptance still open

The actual two-MVM transfer and installed retirement successor has not yet run.
Its fixture must install distinct signed role bundles, bind the executed images
to their authenticated root declarations, and preserve the existing exact
readback and byte meters. The previous 257-byte transfer still has a failed full
lifecycle result.

Converted startup failures before a journal helper exists retain original
custody conservatively; a positive no-helper recovery transaction is not yet
qualified. Parent callback receipt responsibilities, actual failure/retry,
orphan/crash recovery, workspace/BDD/platform checks and the full delivery
program remain open. Native/helper wait stages have internal bounds; the
caller deadline is not a hard deadline for every filesystem sync or parent
callback. No complete release, WAN, capacity, CUI or FIPS claim follows.
