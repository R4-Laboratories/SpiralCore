# Original mailbox helper recovery on macOS

Two actual mailbox helper executables recovered the original sealed message and destination-signed receipt after both helpers were collected and replaced. B returned all 257 application bytes exactly and accepted the signed reader ACK. The fixture did not remove the occupied service sockets for the controller. This is one Apple Silicon macOS host-process trial: guest streams, signing identities and live process handles standing in for the VMMs are fixtures. No VM was booted and no WAN was used. [Source revision](https://github.com/Casey-allard/mvm/commit/c18489d4486c1ebae4df39454a50a6a026fc7f6b) · [Machine-readable measurements and identities](darwin-helper-reconnect.json) · [Complete synthetic fixture output](darwin-helper-reconnect.txt).

| Observed boundary | Result |
| --- | ---: |
| Exact application payload | 257 B |
| Each of three sealed A → B relay turns | 5,542 B |
| Each B → A signed verdict | 1,139 B |
| Each guest submission | 325 B |
| Submission through verified V2 receipt before replacement | 25.621 ms |
| Submission through verified V2 receipt after replacement | 40.010 ms |
| B Receive request / response | 356 / 5,666 B |
| B signed ACK request / response | 2,208 / 2,212 B |
| First connection through B ACK, including recovery | 363.813 ms |
| Successor A unassisted exit | 209.033 ms |
| Successor B unassisted exit | 25.565 ms |

All three canonical envelopes and signed verdicts compared equal. The first return was deliberately lost to the fixture guest. The two receipt timings start at their respective retry submissions; they are not cold-transfer timings. Inner libtest elapsed time was 1.49 s; the enclosing process reported 1.53 s for the same trial. CPU, peak memory, complete port-link traffic and a zstd comparison were not measured. These byte counts describe individual protocol exchanges, not compression savings.

## Original ownership and platform mechanism

Linux can retain a socket inode using `O_PATH`; the corresponding macOS open attempt returned errno 102. The macOS implementation instead retains the original private parent directory descriptor and arms a kernel directory watch before identifying the socket. It authenticates the actual held child on both sides of capture. An observed namespace or attribute mutation makes the watch permanently unsuitable for occupied-path retirement, even if the original inode is restored.

Before interrupting a helper, the controller rechecks the watch and original admission. After collecting the original child, it requires the unchanged original socket, a kernel connection refusal, and another watch check before removing the stale leaf relative to the held directory descriptor. A vacant leaf needs no deletion. Changed or uncertain state refuses recovery while retaining custody. This operates under the existing trusted-operator and exclusive-writer assumptions; it does not establish an atomic unlink guarantee against a malicious same-UID namespace mutator.

Nine focused macOS reconnect tests passed. They cover elapsed deadlines leaving the original child alive, collected original-child status, substituted paths, replacement followed by restoration, unchanged refused sockets, and untrusted parent refusal. The paired-process test passed once, including unchanged receipts, current-revocation refusal and original successor shutdown. Strict macOS Hostd library Clippy and Linux x86-64 Hostd test-target cross-compilation passed on this source. Cross-compilation is not Linux runtime evidence.

## Remaining installed gate

The live witness must restart the held original helpers while both actual admitted guests remain alive, recover the original message and receipt, complete exact destination readback and ACK, and retire the successors through the original owners. That witness is separate from this process fixture and from the [2,049-operation native session](native-2049-checkpoint.md). This component revision also predates the subsequent rc.2 integration; its measurements must retain the original source identity. [Recovery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/195).
