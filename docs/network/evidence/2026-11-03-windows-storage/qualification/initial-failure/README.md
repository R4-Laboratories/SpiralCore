# Retained initial native failure

Run `41740719867`, PR head `2a9d2197d7c73fed46203d00440555665660ff13`, tested GitHub merge commit `8008497b1f86f1ad1649ddc26317af4372495901`. Original row identity files and `tested-merge.json` retain this distinction. No passing Windows storage runtime result came from this attempt.

All three Windows rows failed compiling the canonical adapter dependency closure: `spiralcore-transfer/src/lib.rs` forbids unsafe code, and its new Win32 filesystem module used 37 unsafe blocks. The earlier isolated module typecheck did not include that enclosing crate policy. The complete native crate build exposed the integration defect. Original compiler output remains in each ZIP's `canonical-list.txt`.

The macOS 26 Intel row compiled and passed the adapter scopes, then ran the three private-object-storage tests: one passed and two failed on initial `Endpoint::open` with `Storage`. The fixture used PID and `SystemTime::as_nanos` for names. A bounded Rust probe on macOS 27 ARM reproduced thousands of duplicate names across three threads. That mechanism is observed locally; collision of the specific CI paths is inferred because the failed tests did not log their paths. A counter suffix and exact equal-timestamp uniqueness assertion correct the fixture without changing ownership refusals.

| Original archive | Artifact ID | API SHA-256 verified |
|---|---:|---|
| `archives/windows2022-x64.zip` | 13281825571 | `6be2f6a76edf45f7b50adf2dd4ed3a9048172455e05e1881352ec72f161e7b58` |
| `archives/windows2025-x64.zip` | 13281262783 | `b86719719630d1d4f0be08ae939013f9511e7f625bdb74305ab4ab351fe5b884` |
| `archives/windows11-arm64.zip` | 13281576257 | `eba68dbc50920f9626cac68c42f4de8ff419d3732d7e224bc1283eef33ecb0d5` |
| `archives/macos26-intel.zip` | 13282445691 | `a8dd46de200c343474346351ff9d0abfa795e116947b9d15c6776dbfb26c391e` |

All 52 inner hashes verified: five per Windows ZIP and 37 in the Intel ZIP. Only derivative extraction directories were removed; original archives and API metadata remain. Checksum manifests use CRLF. `checksum-parser-note.txt` records the initial parser refusal and normalization of derivative manifest line endings only. Original archive and data-file bytes were preserved.
