# PR71 candidate run41740719870 — Phase1 diagnosis

Observed failing predicate: tools/gateway-network/runtime-object-netns.sh490–492
requires the shaped right netem qdisc to have qlen0. The archived cell
campaign/wan80-10-t1-combined-cpu has tc-left-accepted.txt=true and
**tc-right-accepted.txt=false**. The right measurement-after snapshot has qlen1,
backlog42, drops142. Independently evaluating every conjunct gives: exactly one
netem=true, qlen_zero=false, drops_nonnegative=true, limit256=true,
delay40ms=true, rate1250000bytes/s=true. The failed jq-e therefore returns1 before
line495 store allocation recording and before complete.json. This remains a
FAILED/incomplete cell; fidelity/capture success does not make it accepted.

The service evidence is not a decoder failure: client has20 successful operations,
server status=ok, reopened readback=true and expected_files_exact12. The original
exact-accepted.txt and capture-accepted.txt are true. Capture summary has33799
packets,33945287 IP bytes,32998915 UDP payload bytes,34418473 Ethernet bytes and
other_packets0. Captured33799=received33799, dropped0. Service stderr files are
empty. Shaper packet losses142 remain observed; they are allowed by this gate's
nonnegative-drops condition and are not the failing conjunct.

Data-flow boundary: capture command filters UDP port4433. Its drain snapshots
are33798,33799,33799 packets with captured=received and zero capture drops. It
therefore proves the filtered capture delivery stabilized, not that every packet
class in the whole qdisc drained. The qdisc then reports one42-byte queued packet.
A link-layer/control packet is a plausible explanation, but its identity is not
established by this UDP-only PCAP. No claim that it is definitely ARP or harmless
is made. The observed cause of the exit is certain; provenance of that residual
packet needs additional scoped evidence if it matters to the correction.

The archive source metadata identifies source2a9d2197d7c73fed46203d00440555665660ff13,
workflow8008497b1f86f1ad1649ddc26317af4372495901,
controlf1f30b8fb9890ea22af0b06c64be080a4c952128 and attempt1. Its source-files.sha256
pins the inspected script to9260eea950de8b258d97f45aa59e87a31956725e164ec1ff65189893a6a29657,
identical to current local script bytes. Original archive SHA-256:
8631d00332a9f2cc588a5ac3793e5b7e879178eeee6a0eb9179d56392497d5c8.

This is read-only Phase1 using the original archived members and independent jq
conjunct evaluation. No product/harness edits, measurement reruns, larger waits,
weakened qlen predicates, removal or acceptance relabeling were performed.
The retained source scope is local/shaped three-namespace networking, not real WAN.
Parent/root owns any orchestration correction and fresh required qualification.
