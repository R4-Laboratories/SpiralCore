# Installed FlowMux startup on Linux Firecracker

[Trial 9](verified-echo.md) completed the signed one-guest invocation and verified the 5,804-byte echo in each direction. The eight earlier trials below preserve the startup and authorization failures.

The sixth attempt booted the signed guest into its agent on Wick. The agent refused host control because its kernel command line lacked the host signer’s public key. The installed key file existed and was 32 bytes. The Firecracker boot-source request lacked the corresponding token. The SDK returned a startup error, the cooperating peer timed out, and no echo completed in that trial. [Source identities, original log digests and measurements](results.json)

| Trial | Controller AS / device (MiB) | First observed refusal | Command wall (s) | User / system CPU (s) | Peak RSS (KiB) |
|---|---:|---|---:|---:|---:|
| 1 | 512 / 256 | The funded direct endpoint rejected the operator’s explicitly empty proxy table. | 0.11 | 0.01 / 0.01 | 30,432 |
| 2 | 512 / 256 | Original tenant helper startup failed. Its fresh home lacked the signer key directory. The controller did not expose the nested errno. | 0.09 | 0.01 / 0.01 | 32,548 |
| 3 | 512 / 256 | The tenant signer started. Reverification of the signed bundle returned an allocation error reading the tar entry body. | 21.63 | 20.17 / 1.02 | 152,204 |
| 4 | 768 / 256 | After signed-plan admission, guest runtime preparation returned ENOSPC. The error did not identify the failed write. | 29.36 | 27.15 / 1.33 | 269,872 |
| 5 | 768 / 512 | The original network endpoint wrote “endpoint component metadata credit exhausted” to stderr. The partition allowed 48 MiB. | 31.92 | 29.54 / 1.26 | 266,816 |
| 6 | 768 / 512 | Firecracker booted the guest. The guest agent rejected the control connection without its pinned host signer key. | 33.13 | 30.67 / 1.45 | 265,340 |

The physical backing was a private ext4 volume in a disposable mount namespace. The SDK, original helpers and peer ran unprivileged. Each installer issued a fresh original plan against the same signed 61,860,864-byte package. All six attempts used the same guest role, Firecracker binary, guest memory declaration and literal cooperating endpoint. The issuer changed in trial 3 to initialize the host key using the canonical MVM API. Trial 4 used the issuer built against the same MVM revision as the controller. Each peer reported listener readiness. Each namespace exited with status 1 and its loop attachment and host mount were absent on return.

The final metadata reservation was 128,057,344 bytes. The signed policy selected five scanner rules. The controller used the MVM collector and scanner queries and reserved the maximum supported bootstrap configuration before registration. The generated configuration’s actual node count was unavailable to the producer. Trial 6 recorded a 637,120 KiB virtual address-space high-water mark and a 265,340 KiB resident high-water mark. The table uses GNU time for the SDK command and its collected children. The issuer and cooperating peer were timed separately. These rows provide no application transfer time, CPU cost or network byte count.

After trial 6 the tenant helper reported a successful stop. The SDK refused the kernel retirement receipt because the generation was unbound to a process. Its resource ledger retained one guest, 256 MiB of guest RAM and 17 workers. No Firecracker remained. At that point the next gate required the registered signer key on the boot command line, binding the original process, and observing workload exit, echo and retirement.

Trial 7 reached original guest activation and SDK retirement. Its driver waited for a detached exit before sending the separate workload invocation. [Trial 7 measurements and audit boundary](activation-only.md)

[Trial 8](entrypoint-grant.md) follows the original signed two-verb plan through startup and retirement, including the missing guest-specific grant and the peer's accept deadline.
