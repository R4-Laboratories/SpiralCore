# Genuine signed FlowMux fixture export

The [hosted Linux export job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43506622776/job/132165881465) created and independently reopened a real **62,385,152-byte** workload package on source `f4b6d0dfea2036ebed9419a228ae5a9a1396080f`. This verifies the signed-export and independent-reopening path tracked by [issue 261](https://github.com/R4-Laboratories/SpiralCore/issues/261). The job did not boot a guest or transfer application data. Wick remained operator-paused.

The static guest came from the existing UID-checked program, built through the standalone locked manifest. Its nine loopback component tests passed. The normal producer and the qualification test compiled, and strict Clippy passed for the qualification target and all native guest targets.

## What executed

One identified test executable first invoked a producer built with the original `2c9c2633` main file overlaid on the candidate harness. The verified inputs reached that producer, which returned its specific unimplemented error. The test exited 101 with its one intended failure. The original file's SHA-256 and Git blob were checked before the overlay; the candidate source was restored and checked before its build. This control does not represent a full original checkout or a performance baseline.

The same test executable then invoked the implemented producer. The test provisioned a disposable publisher through MVM's existing key helper before invoking the producer; the producer itself still required an existing identity. A separate consumer pinned that public key and independently applied the supplied current signed revocation feed. It reopened the archive and checked the signed role, kernel and official rc.2 runtime through the canonical SDK. Both signed revocation members and emitted sidecars matched the supplied inputs. The consumer's full revocation checkpoint matched before and after boot-asset preparation.

All seven expected packet files had the required ownership and permissions. The signed environment retained the exact guest entrypoint, one CPU, 256 MiB and only the selected egress destination. The input static ELF's SHA-256 was checked before invocation. Repeating the producer against the occupied destination refused, preserving the original directory and member inodes, lengths and digests. The candidate test exited 0 with one passing test.

## Recorded setup costs and identities

| Candidate qualification measurement | Result |
| --- | ---: |
| Wall time | 26.54 s |
| User CPU | 23.74 s |
| System CPU | 0.98 s |
| User plus system CPU | 24.72 s |
| GNU time maximum RSS | 67,320 KiB |
| Generated archive bytes | 62,385,152 |
| Applied revocation publication | 2 |

GNU time wrapped the test and its producer children after compilation. The measurement includes private-home setup, official asset acquisition, image materialization, signing, archive/install verification, the no-overwrite control and teardown. Maximum RSS is the reported process maximum, not simultaneous aggregate memory for the process tree. The control's early refusal took 0.03 s; it performs different work and supplies no speed comparison.

The guest ELF SHA-256 is `748aa46cf43a53a7657840649041ea99b152b44fe12c39e259ec30e31e1aa12c`. The generated archive SHA-256 is `47c637dd5050b3f003c9fb34db9b5cf20d94e0ec38ebd95596e87bafac143987`. The qualification executable SHA-256 is `fbdfd2d4e7c72ac7dd21b9cdf7107a8ca157d686ea547ecd41c4b8f17657291f`; all three executable identities were checked before and after both invocations. Rust was 1.97.1 and the host SDK was MVM `31f8a562`. The official guest runtime retained its separate `0.28.0-rc.2` identity.

The 2,191-byte public report ZIP has SHA-256 `270a2554c312395113055727be1c849e72f74ff55dde518572e746910bd1c9a0`. Six named public files retain the source, executable identities, outcomes, timing and result. Private keys, the temporary workspace and the generated packet were not uploaded. [Exact reports and member hashes](genuine-fixture-export.json).

Application transfer bytes, transfer latency and WAN performance are **not run** here; asset-download traffic was not measured. The run checked the original/staged ELF digests and the signed root filesystem, without independently extracting and hashing the guest ELF from inside ext4. Actual guest execution, exact 5,804-byte echo, signed audit and original-owner retirement remain the next installed gate. This result establishes neither fleet capacity nor FIPS validation. [Reproduction commands](../../../../tools/installed-flowmux-witness/README.md#prepare-the-authenticated-inputs).
