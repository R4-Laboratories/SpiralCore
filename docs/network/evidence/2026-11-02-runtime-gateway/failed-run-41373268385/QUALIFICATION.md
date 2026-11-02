# Incomplete first campaign: capture shutdown

Run `41373268385`, artifact `13110327774`, source
`507b0d0c851d274a9432313f72dbb26f17cbb6af`.
Original ZIP SHA-256:
`fac76ad7947569b07b76b92018eaac144a3fa01c5d111ec322a5f9a2253a37b9`.
The downloaded ZIP matches GitHub's digest and all 157 artifact-manifest entries
verified after extraction. Large frozen binaries and the PCAP are omitted from
this compact copy and retained in the original archived ZIP.

Linux tests, strict Clippy and release build passed. The first local raw session
completed 12 deliveries, eight distinct expected files reopened byte-for-byte,
successful publication receipts and equal opposite-direction channel counters.
It then failed the mandatory capture gate: 49,123 packets were written versus
49,130 received by the filter, with zero reported kernel drops. The seven-packet
tail is not silently discarded or treated as complete accounting. No shaped
session, runtime arm or warm arm was reached; this is not a qualified comparison.

The harness used a fixed short delay before stopping buffered tcpdump. The next
source revision will explicitly drain capture with bounded admission while
retaining captured==received and zero-drop requirements. This is a diagnosed
capture correction and a new source-pinned run, not an automatic retry or removal
of a losing result. The full incomplete witness remains reviewable.

Separately, quick TUN regression run `41373268329` succeeded on this source.
Old packet campaign run `41373268379` was deliberately canceled because editing
its path filters triggered its own previous research campaign. It is recorded as
canceled, not passed or failed; no packet implementation changed in this task.
