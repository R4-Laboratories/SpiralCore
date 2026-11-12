# Two installed Firecracker guests start under volume registry contention

The original mailbox SDK sometimes refused one of two simultaneous launches before either guest could transfer data. Both starts access the same volume registry even when the application requests zero explicit mounts. The object-service integration had omitted the previously qualified five-second Unix launch lock. This run restored it to the SDK used by the caller and callee. [Earlier lock implementation](https://github.com/Casey-allard/mvm/commit/16eabe49c7981cf41a0a6dcb788cd1d86f8fbf62) · [Failure from the paired comparison](../native-object/empty-pacing.md) · [Integrated MVM source](https://github.com/Casey-allard/mvm/commit/23328db11e535baec8dd964cafc6ab2fa27fd74b)

## Launch and recovery

A separate Linux process exclusively locked the fresh home's registry inode before starting the original paired driver. It observed that exact driver executable with a descriptor open on the same inode, waited another 100 ms, and verified the executable and descriptor identities again before releasing the lock. Release occurred 691 ms after starting the launcher. This interval includes launcher setup. The original driver then admitted both participants, and each retired on its first stop with an unassisted exit code of zero. Firecracker and active MVM scope counts returned to zero. MVM's audit verifier accepted 17 entries, two sessions and two seals. [Overlap record](contended-launch.txt) · [Observer source](hold_registry.rs) · [Test and retirement output](test-results.txt)

The unchanged launch implementation at `af88b36fb` returned an immediate busy error in the regression. Against the joined source, all 66 Linux tests selected by `volume::` passed. The service waits only on kernel-reported contention, holds the same opened regular file, rechecks its identity against the registry name and reads the catalog under the lock. The test also confirms that an invalid catalog left by the previous writer remains intact and prevents admission. The lock path for other volume operations retains its existing behavior.

## Exact transfer on the admitted pair

The sender used the fixed-cadence raw Object-v1 mapping. The recipient reused one original worker and byte-compared all 4,096 bytes on both operations. The driver counted host/guest application exchange separately from the two port-5257 links and from the sealed helper channels. Both traffic columns include requests and replies. Each sealed byte counts once across the two endpoints. [Complete phase counters and readbacks](native.json)

| Operation | Object-v1 bytes | Sealed helper bytes | Guest port bytes | A / B workload CPU | Coordinator CPU | Elapsed through both guest results |
|---|---:|---:|---:|---:|---:|---:|
| Cold, 4 Calls | 4,658 | 64,002 | 754,532 | 10.131 / 28.891 ms | 27.029 ms | 8.750149 s |
| Exact repeat, 1 Call | 190 | 14,282 | 165,340 | 2.605 / 7.782 ms | 7.275 ms | 2.336933 s |

The full driver lifetime was 16.845099 s and its process used 8.354015 s of CPU. Guest workload CPU covers the two applications. The workload, driver and full external process-tree measurements have different scopes. This single same-host pair establishes contended startup, exact transfer and retirement. WAN latency, fleet capacity, a matched zstd control and Apple installed object execution require separate runs.

## Reproduction

Use MVM host source `23328db11e535baec8dd964cafc6ab2fa27fd74b`, selected guest-agent source `c9d9fe87b27cd2e5fc64d2b1d9c3a31b522b2097` and the raw 4 KiB workload/driver at SpiralCore `615d8ff98429b2339a069dcd0735c3df1a40af25`. The guest binaries were release builds. The driver and helpers used dev builds under Rust 1.97.1. All 4,530 files of the joined MVM source archive matched their Git blob identities. [Hashed binaries, counters and logs](source-results.sha256) · [Native paired setup](../../../../../../tools/ari-own-mailbox-witness/paired-native.md)

Build the Linux host helpers with feature `durable-guest-io-witness` and the driver with `installed-object-pair`. Prepare a fresh private run directory with `home/volumes`, `evidence`, a link at `install` to the selected host binaries and a `run.sh` that launches the validated `paired-journal installed-object-codec raw` command using the two sealed image paths and digests. Set the private MVM home and admitted Firecracker, endpoint, helper, runtime and kernel paths as in the paired setup. Compile the observer with `rustc --edition 2024 hold_registry.rs -o hold_registry` and run `./hold_registry /absolute/private/run-dir` on the Linux test host. A missing overlap or unsuccessful driver exit fails the observer.
