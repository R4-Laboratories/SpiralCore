# Resident-helper publication verification

Verdict: **PASS for this bounded source/evidence publication**. No new build, test execution, source audit or product edit was performed. This closes the requested packet verification, not production profile/native/WAN activation or the whole MVM goal. Literal quality-gate tooling was not invoked.

Exact source candidate `d10ca39984f2bc7bbff968d5f34fabff3c20fb2a`, tree `f6b0460457823b5005bae82545f9b56eb40150fe`, parent `56db5291bf53433340eaf7102041370ee48aa7cc`. Immutable commit changes exactly three resident/helper Rust files and the resident contract. The three product/test Git-blob hashes equal the independently reviewed successor: resident `8fac9672a1d923c24b6a8b069180e7704d75ae0feaec2d8557467746a2ef0250`, helper `d90529eb4843d4327bcdf6ea3d76b2fa6ff75e23b0ede3c7bbeae7afc611a110`, tests `add4cbaf300ff862538f9dc4759841d431d4765e84be0c3b30f3622c47af0eba`. The final contract hash `6a95d211a325fc43f5052ba11420b5a11e9e92c0e6b053f5623c482941359b4a` adds accurate executed-result wording and retains activation exclusions.

Verified the archive itself is 349,762 bytes, SHA-256 `744e1ce08bf0866b14f0e923fb84eff9036eac57e2fee521f3f188fb2e399349`. It contains exactly 69 regular files: 68 unique indexed bodies and SHA256SUMS. Every indexed member verifies; no unindexed body, absolute/traversing path, symlink or executable-format body is present. Contents are compact logs, commands, identities, source and metadata; synthetic homes/keys, protected executable bodies and targets are excluded. Raw remote custody is not recreated locally.

Independently compared all 131 entries in final-source-blobs.json to this candidate's Git blobs, verifying both recorded Git object IDs and SHA-256. The three supplied qualified-source bodies match those blobs byte-for-byte. All 131 final input hashes also match the final release and Clippy before-manifests. Each recorded source-before/source-after pair compares equal. Initial and final actual-helper binary-before/binary-after maps each compare equal. The final map protects three objects: the rebuilt successor test plus the same unchanged SDK daemon/signer objects. Final test hash is `df75241706670245e1a35d9327e9c012ada8af1e3d20c66dd4c1a27c574058e2`; helper hashes are `856f78044b5cd106e5f9ebc6ba45e959e4e5bf5301456080375596ec3dc06a59` and `422c1be10124ee5b751c43a717b4fc8bfed29f4686a09610830639a79834cd2b`. Recorded manifest equality is evidence of those original checked readbacks, not a claim that this publication review re-executed the objects.

The retained metadata resolves all 19 MVM packages from one immutable `bef4146c0de18b22ac392118acfd898a369cc64c` Git source, one adapter `7f638428522e70b36a82d68700a6374267ea7e40`, and one shared transfer/gateway-core source `ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`; no alternate path copy of those dependencies was found. This graph does not contain FlowMux, and no FlowMux qualification is inferred.

Receipt checks:

- Actual API probe contains exactly one E0599; original normal-library metadata before/after hashes agree. Earlier zero-selection control-script prerequisite is retained as a summary, not fabricated as a compiler failure.
- Initial locked release: exit 0, 22 passed / 2 ignored. Earlier explicit real-helper case: exit 0, 1 passed / 0 ignored. These retain their earlier source identities.
- History-barrier RED: build 0, test 101, 0 passed / 1 failed. Original resident plus strengthened successor test is retained.
- Final locked release: build 0 and normal serial parent execution 0, 22 passed / 0 failed / 2 ignored / 124 filtered. Separately selected exact canonical-helper case on the SAME successor protected test object: exit 0, 1 passed / 0 failed / 0 ignored / 147 filtered. Recorded command supplies fresh synthetic MVM_HOME and protected same-SDK daemon/signer paths. This is 22 ordinary entries plus one separately selected new helper case, not a sum of repeated runs. The older standalone helper case remains excluded.
- Final release/offline/locked Clippy `--lib --tests -- -D clippy::all`: exit 0; emitted compiler-message JSON has zero warnings and zero errors; warnings.json is empty. This is not relabeled `-D warnings`, and does not certify unrelated targets.

README/packet counts, pins, archive digest, source-match statement and overlapping-run explanation agree with the records. The copied code/security review is the exact prior `3dcd92ca...` record, including history and successor; its prior pending-Clippy statement remains historical and is superseded here only by the independently read terminal receipt.

No VM is booted, no VM service is registered, and no object is transferred. Native backend retirement, telemetry ownership, endpoint integration, WAN/performance, accepted numeric model/capacity and FIPS/CUI activation are explicitly excluded. No publication of keys/homes/binaries was observed. No actionable publication blocker found.

## Packet body identities at review

- `README.md`: `9de5cd2e1dfa816b5ae1ee6270857ac00429fa07d005c556539e886cdffbfeee`
- `receipts.tar.gz`: `744e1ce08bf0866b14f0e923fb84eff9036eac57e2fee521f3f188fb2e399349`
- `packet.json`: `711eb28b157e228660ad1b4cbf0ff15a5266a252affcaf6e2701c0a3ee829b62`
- `source-verified.json`: `78c5b2f7859762fe0ab93bc398b250b42d240aa4e1988bf759eab944519aee15`
- `code-rust-security-review.md`: `3dcd92ca8cf032a0c989b1d5f1882b9911dc0455e9b6c477542772240b8a77b2`
