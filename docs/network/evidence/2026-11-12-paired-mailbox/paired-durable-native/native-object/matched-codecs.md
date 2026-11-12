# Raw and whole-object zstd over the installed mailbox

Two fresh Linux Firecracker installations transferred the same 4,096-byte fixture through the same signed Call, Reply and Receive profile. Each installation used the same two role images, original MVM runtime and host helper binaries, with one vCPU and 256 MiB per guest. The witness ran Raw first and fixed whole-object zstd second. In each arm, B returned every byte on both the cold transfer and the same-worker repeat. [Observed per-direction counts, build identities and original-file digests](matched-codecs.json)

| Measured boundary | Raw cold | Whole zstd cold | Raw repeat | Whole zstd repeat |
|---|---:|---:|---:|---:|
| Original application Calls | 4 | 4 | 1 | 1 |
| Data payload in the original Calls | 4,096 B | 4,110 B | 0 B | 0 B |
| Object-v1 requests / replies | 4,376 / 282 B | 4,392 / 282 B | 102 / 88 B | 102 / 88 B |
| Unique sealed helper traffic, both directions | 63,801 B | 64,055 B | 14,197 B | 14,274 B |
| A port 5257, host read / write | 66,272 / 382,361 B | 56,879 / 317,737 B | 15,717 / 99,086 B | 13,734 / 85,597 B |
| B port 5257, host read / write | 95,342 / 357,613 B | 88,360 / 309,427 B | 20,518 / 49,723 B | 20,486 / 49,864 B |
| Both guest links, reads plus writes | 901,588 B | 772,403 B | 185,044 B | 169,681 B |
| A / B application CPU | 9.458 / 26.828 ms | 8.781 / 29.877 ms | 2.415 / 6.702 ms | 2.514 / 8.597 ms |
| Coordinator process CPU | 36.309 ms | 30.295 ms | 8.610 ms | 12.230 ms |
| Elapsed through both verified guest results | 11.404 s | 9.153 s | 2.889 s | 2.450 s |

Raw used two literal fragments of 4,091 and 5 bytes. The SmallV1 encoder produced one checksummed zstd frame of 4,110 bytes, which occupied two fragments of 4,090 and 20 bytes. B observed codec 0 or 9 respectively on the actual Offer and both Data Calls. Fixed zstd increased the encoded payload by 14 bytes on this fixture. Its two one-byte fragment-format tags explain the additional two bytes in Object-v1 requests. A constructed the profile encoder once per invocation and encoded the fixture before issuing its Offer, including on the warm lookup. Cold encoder construction took 215,865 ns wall and 31,700 ns process CPU. Warm construction took 14,788 ns wall and 15,969 ns CPU. These subset costs are included in A's workload accounting. [Leased sender](https://github.com/R4-Laboratories/SpiralCore/blob/615d8ff98429b2339a069dcd0735c3df1a40af25/crates/spiralcore-transfer/src/client.rs#L585)

Each arm used the same original-length and object-ID checks, a fresh enrolled pair, a single outstanding Call, a 4,152-byte Call cap and an 88-byte Reply cap. Each original submission required the V2 durable peer receipt, each application Reply identified its original Call, and the workload explicitly acknowledged the original leases. B checked authority and reread the stored object on the terminal Call. The host compared every returned byte, and verified the same original B boot and worker across the two phases of each installation. The warm Offer used the previously published object in that installation with one Call and zero Data fragments.

The sealed counters include envelopes, serialized credentials and signed receipts. Both independently observed ends matched on both channels. Cold sealed traffic rose by 254 bytes. Of that increase, 90 bytes were on A's sealed request writes, 32 on A's returned receipt reads, 110 on B's sealed Reply writes and 22 on B's returned receipt reads. This records the distribution across the four streams. It does not attribute individual bytes inside the authenticated records. The guest-facing links also carried authority checks, polling, submission, ACK and readback traffic. Their cold exchange counts varied from 108 and 87 on Raw A and B to 89 and 73 on zstd. Guest-link and sealed counts describe successive layers and cannot be added into a single network total.

| Whole installed run | Raw | Whole zstd |
|---|---:|---:|
| Setup through both verified SDK starts | 5.125 s | 4.729 s |
| Command wall | 19.87 s | 16.85 s |
| GNU time user / system CPU | 11.11 / 0.21 s | 10.33 / 0.20 s |
| GNU time maximum RSS | 83,952 KiB | 83,380 KiB |
| Original helper exit statuses | 0, 0 | 0, 0 |

All six meter sessions closed in both arms. No transfer phase recorded a read or write error. In each whole session A's guest-port meter recorded one write error outside the measured intervals. Both original helper stops succeeded unassisted on their first attempt, both installations verified 17 audit entries and two seals, and cleanup left no Firecracker process, active VM scope, retained definition or per-VM runtime directory. The shared host was also running an independent transfer test suite during the campaign. The single observation per arm, fixed order and different reader polling counts limit timing and CPU comparisons. WAN and energy were unmeasured.

## Reproduction

Build both guest programs from the same tested SpiralCore revision with `large-object-fixture,object-reader-v1` and seal the resulting role images once. Build the paired driver with `installed-object-pair`. Use the measured MVM host with `durable-guest-io-witness`, the recorded guest runtime, and the same operator settings for two freshly provisioned homes. Run one installation at a time with the same role images and digests.

```text
paired-journal installed-object-codec raw A.ext4 A_SHA256 B.ext4 B_SHA256
paired-journal installed-object-codec zstd-whole-v1 A.ext4 A_SHA256 B.ext4 B_SHA256
```

The measured driver and both images used SpiralCore `615d8ff9` with the fixed leased sender at `06e041da`. The publication branch also incorporates the subsequent correction to the statistics for other fitted codecs. The source and binary pins for the actual runs are preserved in the measurements. [SpiralCore source](https://github.com/R4-Laboratories/SpiralCore/commit/615d8ff98429b2339a069dcd0735c3df1a40af25) · [MVM host](https://github.com/Casey-allard/mvm/commit/30ca64feba564f8a3a3f62ee96c746c51176567c) · [MVM guest runtime](https://github.com/Casey-allard/mvm/commit/c9d9fe87b27cd2e5fc64d2b1d9c3a31b522b2097)
