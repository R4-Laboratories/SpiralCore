# First installed addressed Firecracker launch

The Release-built guest programs and debug-profile installed host services were staged on Linux. The signed two-guest driver requested a **257-byte** transfer with independent destination readback. Both SDK launches stopped at workload-kernel admission. Neither guest booted. The driver collected no guest transfer counts, payload readback or transfer latency. [Recorded failure and image hashes](results.json)

| Observation | Result |
|---|---|
| Both workload kernels | The staged bytes matched the expected SHA-256. The copy omitted the donor’s `vmlinux.sha256` sidecar, which the canonical kernel resolver requires. MVM rejected the cache and evicted the kernel. |
| Timing | Launch setup took 2.244 s. The full unsuccessful lifecycle took 212.260 s, including a timeout waiting for the original coordinator to retire. |
| Recovery check | A fresh private installation received the kernel and the original sidecar, independently of the first installation’s machine state. MVM’s unmodified resolver accepted the cached kernel with digest verification enabled. |
| Root filesystems | The SDK materialized both directory inputs before its kernel check. The runtime replaced their OCI wrappers, and its injection removed the image-supplied entrypoint and runtime configuration. The next preflight detected eight mismatches in the ten-file input manifest and stopped before launch. |
| Installed tests | Seventeen selected Linux tests passed with the installation’s private umask. A separate run under `0002` reproduced the positive test fixture creating a group-writable directory, which the service correctly refused. [Tests, binaries and recorded scopes](host-tests.json) |

The prepared guest runtime took 812.73 s to build. Preparing the final static workload binaries took another 39.62 s. Those costs precede launch. The [earlier workload-input packet](../guest-images/README.md) records the directory contents **before** SDK materialization. The next run needs operator-admitted workload configuration that survives canonical materialization, verified inside each actual boot image. The first coordinator’s idle-retirement failure also remains under investigation. [MVM runtime injection](https://github.com/Casey-allard/mvm/blob/5ae2cdc4977e12a574a5baa2e4f0522c3f7a1ae7/crates/mvm-build/src/oci_runtime_inject.rs) · [Integration tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)
