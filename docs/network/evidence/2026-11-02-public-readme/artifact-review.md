# Public evidence archive review

The owner requested public GitHub access to the evidence behind the network and
MVM results. This scoped credential review covers the four exact source archives
below. Final public delivery uses the executable-free subsets described in
`redistribution-notices.md`; that separate packaging decision preserves all
remaining original file bytes. This review does not authorize changing the
visibility of the older 7.7 GB historical preservation dataset.

| Archive | Inventory entries | SHA-256 |
|---|---:|---|
| `mvm-combined-9ceaeef.tar.gz` | 563 | `1bc2a967567a9c6bf55c7dc9e4767582836f9dbaccbdd995d286e5dd00f3963c` |
| `cpu-conscious-41465450384.zip` | 914 | `bdb1242f93d3f1acf4eb0335f3a5e808ceb6cf64d6902bf1643eeaca776d5e68` |
| `gateway-41361459962.zip` | 863 | `59238980889273cb9c069f396d8dd273229e81f12e218db5e9b0a1056e49e851` |
| `packet-selection-41364240626.zip` | 3,053 | `7c56b4a6abef97122513e74811de7d66a4954180fb8bbbf6bd1fb1619bcfb5db` |

All four hashes match the existing recorded custody identities. The archives
were enumerated and extracted into an owned review directory without running
their executables. No symlinks, private-key files, credential directories,
credential-bearing bootstrap/initramfs, receiver stores or disk images were found.
Text inspection for PEM private keys, recognizable provider access-token forms,
Bearer authorization material and serialized secret/password/token fields found
no credential value. MVM signing/encryption keyword matches were test names in
the validation log, not key material.

The three network archives contain nine DER files total. Each was parsed with
OpenSSL as a public X.509 certificate for `peer-a.test`, `peer-b.test` or
`SpiralCore-test-CA`; none is a private key. The source keeps private test keys,
service configuration and data stores in a separate temporary directory, and
copies only the public certificates into the evidence directory. The MVM
orchestrator passes credential-bearing bootstrap/initramfs bytes through a
private pipe and memory; those files are absent from the archive.

There are 18 object-gateway, 12 original TUN and 32 packet-selection capture files.
Every PCAP global header specifies a 96-byte snapshot length. The corresponding
source captures only UDP port 4433 on the isolated namespace underlay. These
snapshots retain packet metadata and may include a short encrypted QUIC prefix;
they should not be described as containing no payload bytes whatsoever. The
campaigns use public deterministic fixtures and isolated test addresses, not
captured ordinary user browsing traffic.

The archives retain reproducibility metadata, including timestamps, local source
paths/user names, hosted-runner details, public test certificates and synthetic
network addresses. These are part of the requested experiment evidence. No
blocking credential exposure was identified within this scope, and no credential
redaction is required. Original archive bytes and hashes remain unchanged; the
separately named public subsets omit frozen executables for the notice reason
recorded above. This is a bounded release review, not
a claim that pattern scanning proves the absence of every possible secret.

The old historical WAN preservation archive remains outside this review: its
raw network metadata and broad directory snapshots require separate examination.
Its already public compact results and source-linked reports remain available.
