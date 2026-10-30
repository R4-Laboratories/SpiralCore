# broad16-cold_structured: five-shard evidence custody

This packet preserves the completed `cold_structured` case at source
`9dce4c57d1f41cef7365b297cd690849afce34e7`: five successful first-attempt GitHub
runs, 30 paired trials across four arms, and **120 successful transfer receipts**.
There are no seed operations in these plans. Every object is 16,777,216 bytes.
Each shard's reopened verification compares 24 publications and 402,653,184 bytes
exactly, outside transfer timing: **2,013,265,920 bytes** across this case. Each
local plan matches its downloaded canonical plan byte for byte.

The fixture repeats a seed-derived 4 KiB pseudorandom template across the object.
The first eight bytes of each record contain the little-endian encoding of
`record_index mod 16`. This is structured repeated data, not an all-zero fixture.
This cold case sends the whole initial object; it does not apply the midpoint XOR
mutation used in the separate warm structured case. The receiver's recorded cold
graph-warming policy is disabled, so these results do not establish readiness of
an eagerly built graph cache. Cold sparse uses its Raw fallback and cold prefix
uses ordinary zstd, as specified by the fixed benchmark arms.

`aggregate.json` contains the author's analysis. `independent-review.json` reports
agreement across all 120 receipts and statistics; `independent-pcap.json` provides
separate raw packet-total replay. All five measured captures are complete under the
retained criteria. Each shard retains its complete outcomes without filtering.

Each `shard-N/remote/` retains original readable rows, plan, summary, capture
accounting/lifecycle, tcpdump counters, route/offload state, sender identities,
authenticated build-reuse proofs and fresh runner capture qualification. Its parent
retains receiver verification, publication inventories, cleanup and authenticated
run/artifact identities. The unchanged scientific checker and receiver identity files
are referenced in `../localized_32/common/`; all 25 shared files were compared with
their local originals and verified through `shared-evidence.sha256`.

Fresh authenticated GitHub API responses established the exact source, repository,
workflow, successful attempt one and one expected measurement artifact per run.
Each original ZIP's actual SHA-256 and byte count match its API identity. The
previously qualified member checker then compared every actual ZIP member with the
retained remote files. Its explicit visited-member and nonempty-manifest gates require
**56 visited members and 56 manifest lines per shard: 280 of each in total**.
`member-count-acceptance.json`, each `member-recheck.json`, and the custody receipts
bind the checker hash, actual archive identity, manifest and counts. The checker's
source, negative qualification and scope are retained in the
[custody correction packet](../../qualification/artifact-member-custody-correction/README.md).
This final case used the qualified checker directly; it did not use the earlier
defective loop. Independent scientific analysis is separate from this custody check.

Original ZIPs totaling **38,075,013 bytes** and four PCAP files per shard remain
local at the absolute paths in `local-raw-custody.json`. Their bytes are bound by the
complete member manifests. ZIPs, PCAPs, capability secrets, executables and diagnostic
stdout/stderr are omitted from Git. Six small qualification marker/PID files per
shard are also omitted from Git but retained in the complete local archive inventory.
Synthetic qualification rows remain distinct from the 120 measurement rows.

Each receiver inventory records **26 files: 24 `.object` publications, the profile
marker and the empty owner lock**. Original checks reported all 26 files OK. These
completed synthetic caches were removed only after exact reopening and hashing.

`hosted-artifact-custody.json` selects only these five measured artifacts. Parent
review and committed preservation are required before their cleanup. Producer
artifact **12931876271** and the separately preserved setup-failure artifact are
excluded. Packaging performed no commit, push or hosted deletion, and did not
create a portable bundle or alter previous case packets.

Captured IP bytes cover the selected runner-to-Cloudflare TCP443 leg, not both legs
or physical-link bytes. Sender plus receiver-handler CPU measures computation, not
energy. Elapsed transfer time includes computation; fixture generation and reopened
verification remain separate. Preserve each analysis's scope and negative results.
