# broad16-cold_random: five measured shards and separate setup history

This packet preserves the completed `cold_random` case at source
`9dce4c57d1f41cef7365b297cd690849afce34e7`: five successful first-attempt GitHub
runs, 30 paired trials across four arms, and **120 successful transfer receipts**.
There are no seed operations in these plans. Every object is 16,777,216 bytes of
deterministic seed-derived pseudorandom data. Each shard's reopened verification
compares 24 publications and 402,653,184 bytes exactly, outside transfer timing:
**2,013,265,920 bytes** across this case. Each local plan matches its downloaded
canonical plan byte for byte. The receiver uses the recorded disabled cold-graph
warming policy; this does not establish readiness of an eagerly built graph cache.

`aggregate.json` contains the author's analysis. `independent-review.json` reports
agreement across all 120 receipts and statistics; `independent-pcap.json` provides
separate raw packet-total replay. All five measured captures are complete under the
retained criteria. Cold sparse uses its Raw fallback, and cold prefix uses ordinary
zstd; these named controls do not imply that a warm delta was sent without a base.

Before measured shard 0, run `40984573497` failed capture preflight before client
launch. Its failure packet remains separately preserved at
[`qualification/cold-random-preflight-attempt-1`](../../qualification/cold-random-preflight-attempt-1/README.md).
The parent's [path-mapping receipt](../../cleanup/cold_random_setup_attempt_1_preservation.json)
records the unchanged failed directory's move to
`local/broad-wan-20261029/campaign/cold_random/shard-0-attempt-1-preflight-failure`.
The fresh measured run is `40985302581`. The two shard 0 canonical plans were compared
again and are byte-identical; `setup-history.json` binds that check and the historical
packet/mapping. The failed setup contributes no measured row and is not erased or
represented as a successful transfer.

Each `shard-N/remote/` retains original readable rows, plan, summary, capture
accounting/lifecycle, tcpdump counters, route/offload state, sender identities,
authenticated build-reuse proofs and fresh runner capture qualification. Its parent
retains receiver verification, publication inventories, cleanup and authenticated
run/artifact identities. The unchanged checker and receiver identity files are
referenced in `../localized_32/common/`; all 25 shared files were compared with their
local originals and verified through `shared-evidence.sha256`.

Fresh authenticated GitHub API responses established the exact source, repository,
workflow, successful attempt one and one expected measurement artifact per run.
Each original ZIP's actual SHA-256 and byte count match its API identity. The five
archives contain 56 members each. All **280 members** were actually compared with
the retained remote files using the qualified correction checker, with independent
byte-comparison review retained in the [additive custody correction](../../qualification/artifact-member-custody-correction/README.md).
This packet uses the corrected 56-line manifests and custody records. The initial
unsealed empty manifests and unsupported claims remain preserved in that correction's
history; no previous frozen receipt was silently rewritten.

Original ZIPs totaling **70,600,224 bytes** and four PCAP files per shard remain local
at the absolute paths in `local-raw-custody.json`. Their bytes are bound by the
corrected complete member manifests. ZIPs, PCAPs, capability secrets, executables
and diagnostic stdout/stderr are omitted from Git. Six small qualification marker/PID
files per shard are also omitted from Git but retained locally in the complete archive
inventory. The synthetic qualification rows are distinct from the 120 measurement rows.

Each receiver inventory records **26 files: 24 `.object` publications, the profile
marker and the empty owner lock**. Original checks reported all 26 files OK. The
completed synthetic caches were removed only after exact reopening and hashing;
the separately preserved failed-setup cache is not part of that deletion.

`hosted-artifact-custody.json` selects only these five measured artifacts. Parent
review and committed preservation of this packet and the custody correction are
required before their cleanup. Producer artifact **12931876271** and the separately
preserved setup-failure artifact are excluded. Packaging performed no commit, push
or hosted deletion.

Captured IP bytes cover the selected runner-to-Cloudflare TCP443 leg, not both legs
or physical-link bytes. Sender plus receiver-handler CPU measures computation, not
energy. Elapsed transfer time includes computation; fixture generation and reopened
verification remain separate. Preserve each analysis's scope and negative results.
