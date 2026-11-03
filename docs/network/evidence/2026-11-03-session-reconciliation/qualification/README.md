# Hosted custody collection — PASS

Final qualification is verified for head
`b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea`, Linux run `41677859354` and
portable run `41677859307`. Both successful runs, every native row identity and
the original Linux checkout log identify merge `7c10e0f0`, whose entire tree
equals that head. [Final verification](portable/verification.md) retains all
seven original ZIP/API digests (195,352 bytes), all 236 inner checksums,
95 actual tests per native-client row (665), eight separate Unix storage tests,
and Linux 111 live plus 138 shared tests with all four zero-test scopes explicit.

`initial-failure/` retains the original Windows Server 2022 failure at `26dc93a1`,
its verified original ZIP and API digest, and the superseded successful Linux
run. The test-only writer-phase correction changes no production source or
deadline/partial-byte assertion. Those partial/canceled portable rows are not
final qualification. Final original ZIPs remain in `portable/archives/`;
only owned duplicate extractions were removed after checksum/source/count
checks. Native adapter-client rows do not exercise the new Windows live consumer,
which remains NOT_RUN under the documented profile/storage gate.
