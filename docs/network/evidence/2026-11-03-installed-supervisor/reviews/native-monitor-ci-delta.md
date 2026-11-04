# Native monitor custody and storage CI delta

Disposition: **PASS for the reviewed source deltas**; execution/qualification
remains separate. Fresh infra-reviewer roster check passed; active infra role
instructions read. No build, test, signal, VM or source mutation by this review.

native_death.rs SHA-256
`6954574f70ff52ac0a1c432e11d26aed68dc080adc153b1802076cf2f12c6da6`:
the runner now captures the current backend's live parent before arming cleanup.
The /proc stat suffix's nth(1) is PPID field4. Its validator requires sudo
executable inode/root UID, the unique installation holder inode, owned run
launch-script argument and the current FC child relationship; birthtime and
validation are repeated around pidfd acquisition. All three guards are armed
only after this capture. FC positive exit is followed by monitor SIGCONT if
necessary and positive monitor exit before canonical callard retirement. No
numeric-PID signal fallback or lock unlock was added. Existing pre-capture
normal-cleanup/unknown-HOLD policy remains. This models the actual stopped sudo
SH custody observed in the retained failed gate, rather than bypassing Busy.

.github/workflows/mvm-mailbox-portable.yml SHA-256
`6a3492c3fb82dbd8ed696b3d5db840b1dc654c08e792c6c8488ba5257dd86baa`:
the new storage step requires exactly3 selected library tests on Unix and6 on
Windows, plus21/20 endpoint integration tests respectively. Enumeration must
succeed, passed count must equal listed, and failed/ignored/measured must be0.
Only the selected library scope permits unrelated filtered tests, whose count
is recorded. Endpoint filtered count must be0. This closes zero/ignored-scope
acceptance without claiming the selected library filter ran the full library.

The step uses the existing actual native1.97.1 host toolchain/matrix, locked
transfer manifest,2 build jobs,30minute job cap,3-row maximum parallelism,
read-only token, pinned action revisions and always-retained compact checksum
artifacts. It records the transfer lock hash and refuses lock mutation. No VM,
backend or host/network provisioning is added by this CI step. The scope receipt
explicitly excludes physical power-loss, native MVM backend and WAN claims.

The current manifest mvm_source is7d031c7f and matches the adapter manifest/lock;
the existing ae12 qualification records retain their exact historical revision
and state explicitly that their qualification applies to that pinned source.
They do not establish a PASS for the new source. Corrected Windows source
db97d858 and tests0cb2c6f7 remain source-approved; actual Windows storage runtime
is NOT_RUN until the new hosted rows execute and artifact/source custody passes.

This review excludes the channel implementation authored by this reviewer and
does not replace its independent lead review, final exact-source checks,
native recovery/readback evidence, nonroot observation authority, macOS death
qualification or the full B6a/B6b/WAN/scale delivery gates.
