# Persistent SDK ownership for paired Apple Silicon object mailboxes

The persistent SDK accepts the selected object service on HVF only when the request has the original pair reservation, the signed local plan, the registered application image and its frozen boot inputs. It keeps the original cold-launch handle before activation. Once both SDK starts reach the rendezvous, the host uses each retained live handle to bind its participant and start the two mailbox workers. Each worker acquires stop authority for its own VM, including a worker created before its peer reports a startup failure. [Contributor implementation](https://github.com/Casey-allard/mvm/commit/495835d3935cba9f5032ba62d69fa8b16f9589ca)

A failed activation can release its prepared owner immediately when the backend proves no child exists. If native rollback remains unresolved, the original reservation retains the handle for another stop attempt. After a worker starts, the SDK separately checks the original process exit and collects the helper before cleanup. Selected HVF boots use sealed application images without writable volume attachments. [Original worker and port 5257 controls](hvf-original-worker.md)

## Reproduction

Use the source revision for each row and Rust 1.97.1. The two Linux commands on the prior SDK commit exercise the unchanged Linux implementation. On macOS, the initial production SDK check exposed a crate-private native startup constructor compiled only on Linux or in tests. Enabling the existing constructor for macOS restored the normal persistent SDK entry point.

| Platform | MVM source | Cargo command after `cargo` | Outcome | Original log SHA-256 |
| --- | --- | --- | --- | --- |
| Linux x86-64 | `af88b36fb` | `check -p mvm-client --lib` | Type-check passed | `cd3aad0b92c60ed9372cd975670009b5746b012f6ff254a129dd2becf9a4b87b` |
| Linux x86-64 | `af88b36fb` | `test -p mvm-hostd --lib mailbox_launch::object_pair::tests::` | 8/8 passed in 0.08s | `d916b687b9e894c5d0024fd2d7de574e37e2d1f2d963784d53106a85c6a0771b` |
| macOS arm64 | `af88b36fb` | `check -p mvm-client --lib` | Constructor unavailable in production build | `9d135157c037c9deb07210174f11fa02bdab74c48c8e6983eb465e846286c5e4` |
| macOS arm64 | `495835d39` | `check -p mvm-client --lib` | Type-check passed | `ac7970060a2973f69f0cce8ee88b41cf243fae2b3fbb06b0d4cf32cfe26be1c3` |
| macOS arm64 | `495835d39` | `check -p mvm-hostd --bin mvm-mailbox --features durable-guest-io-witness` | Type-check passed | `3742c813f3b096e63972466d7a6c74fa7b08ba5dd467d834f8c567d7d8273696` |
| macOS arm64 | `495835d39` | `test -p mvm-hostd --lib mailbox_process::object_worker::hvf::` | 3/3 passed in 0.02s | `32e1d513ca6113b509e476c50b8ed29757259aba613321257f0abef3998e2191` |
| macOS arm64 | `495835d39` | `test -p mvm-vmm --lib hvf_stop::mailbox::` | 3/3 passed in 0.03s | `ac5b56f20d186253332c9cb64a732c870f1a87e5c5032a6679d60b2771fde670` |
| macOS arm64 | `495835d39` | `test -p mvm-hostd --lib mailbox_launch::object_pair::tests::` | 7/7 passed in 0.62s | `c39c7e2fcba6b6eb25a5fe288bd5a59e85d3e03b441ef4462cb0b6d294b9baf2` |

All 4,530 tracked Linux source files and modes matched contributor tree `ca7130499ab59ec0bfd5b1fc9f5c419a18ae8fd4`. The commands used `--locked`, one build job and the dev or test profile.

The SDK was type-checked on Apple Silicon and Linux x86-64. The test executables ran natively on their recorded hosts. The native-arm64 workload packs were signed and their eight image members were checked on reopen. The retained runtime came from a source build. The release verifier rejected that provenance before the selected Apple guest boot could proceed. Executing the installed pair requires a matching release runtime accepted by the original admission policy. [ARM64 image identities and release check](arm64-object-images.md) · [Signed packs and observed admission refusal](https://github.com/R4-Laboratories/SpiralCore/issues/202#issuecomment-6834424108)

No VM started in these SDK and worker controls. Application bytes, vsock or WAN traffic, and transfer CPU and elapsed time remain unmeasured for this Apple object-pair profile.
