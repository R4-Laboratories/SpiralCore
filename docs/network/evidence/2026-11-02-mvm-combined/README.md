# SC-MVM-004 evidence and reproduction

Measured implementation `9ceaeef5070c87fd17b8519a1e8630b7f7888fd6` carries the
current Combined core through actual one/two guest MVM/vsock boundaries.
[Result and limitations](../../mvm-combined-results-2026-11-02.md).

- `registration.json`, `schedule.json`: fixed source/binary/kernel identities,
  same bandwidth profile and all 36 sessions registered before execution.
- `measurement-contract.md`, `build-and-validation.md`: exact comparison,
  timing/accounting boundaries, build tools and observed validation.
- `analysis.json`: source-bound analyzer output, all rows and paired medians.
- `supplemental-summary.json`: mechanism counts, application-byte parity,
  CPU/setup reconciliation, lifecycle, environment and losses.
- `compact-receipts.tar.gz`: all original campaign files, including result JSON,
  guest/host events, stdout/stderr, verification, load/process snapshots, cleanup,
  identities and per-session hashes. Contains no binaries, payload stores or keys.
- `review-*.md`, `review-*.json`, `review-*.txt`: independent scoped core/security,
  harness and arithmetic reviews. The original RED and GREEN receipts remain.
- `source-files.sha256`: measured standalone crate and dependency file identities.
  Paths are relative to the SpiralCore checkout at the measured revision.

All 336 publications reopen exactly; 168 also pass separate keyless guest reboot
readback. The campaign contains 288 target deliveries plus 48 setup publications.
All 96 cross-arm target groups agree by length and identity. Injected socket
interruption/reconnect evidence is distinct from native reboot evidence.

## Reproduce analysis without running VMs

Use a new extraction directory; the compact archive expands to the public
`campaign/` directory, not a build tree:

```sh
mkdir NEW_DIRECTORY
tar -xzf docs/network/evidence/2026-11-02-mvm-combined/compact-receipts.tar.gz -C NEW_DIRECTORY
bash crates/spiralcore-mvm-live/scripts/analyze-combined.sh NEW_DIRECTORY/campaign > NEW_ANALYSIS.json
cmp NEW_ANALYSIS.json docs/network/evidence/2026-11-02-mvm-combined/analysis.json
```

The parent performed this fresh extraction/reanalysis successfully. Independent
review also reproduced identical analysis and verified 36 receipt manifests,
288 hashed session files and eight registered executable/kernel/runner entries.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Compact original receipts | 1,335,542 | `72122856fede6bc3be6eb27e8312830583a42915b6f06eda4a4835ecc5e20b50` |
| Full raw campaign, frozen binaries and qualification | 3,816,619 | `1bc2a967567a9c6bf55c7dc9e4767582836f9dbaccbdd995d286e5dd00f3963c` |

Analysis SHA-256:
`f2ddf26bc322fa085581b0885323c6e2dfdd39e8a2fea5eb973a7a06d0196e9d`.
Signed host SHA-256:
`d5d67d96cacc3d0e4aeb8939ded15f98d10ec5c34204fa12b49c5cd7ffed6feb`.
Static guest SHA-256:
`55bd28a31ae80fa1b05ffb5d4c1073c7ed2f3a88db3867dd8dfeb89da6eab11c`.

The full archive is retained in the existing private dataset at
[immutable Hugging Face revision](https://huggingface.co/datasets/caseyallard/spiralcore-experiment-artifacts/blob/33ca33bb6e9db26e73fafff02924e9e8ee8cea75/2026-11-02/mvm-combined-9ceaeef.tar.gz).
A forced fresh download from that exact revision matched its size, SHA-256 and
every archive byte with `cmp`. Access requires the dataset's existing permission;
the Git compact archive is sufficient for public numeric review.

All 36 disposable receiver cache/ext4 stores were deleted only after successful
exact acceptance. Raw observations remain in both archives. Builds reused the
existing shared target and kernel; no extra persistent VM image was introduced.

These are three-trial descriptive local HVF/vsock results, not an Internet WAN,
production mailbox, transparent TUN stream, energy or FIPS/CUI qualification.
