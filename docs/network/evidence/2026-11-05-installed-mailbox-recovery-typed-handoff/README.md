# Installed mailbox Recovery V2 typed handoff

This candidate connects the exact signed recovery enrollment to the original
Core mailbox owner and issues a typed one-shot worker job. A V2 job cannot be
taken or executed through the legacy V1 API. Both the initial native claim and
later path check use the distinct default-deny V2 parent policy. The Hostd
changes share the signed asset definition and select the matching Core prepare
method. The early Hostd V2 dispatch refusal remains in place.

Apply the source patch over the preceding installed listener identity candidate
and the pinned MVM source it documents. The four postimage SHA-256 digests and
the Linux release test result are in [validation.txt](validation.txt).
The independent Code, Rust, and Security review matched all four source pins.
The focused Core suite passed 14 tests after the typed API regression failed
to build with the three expected missing-interface diagnostics. The fixture
acceptor supplies a precreated socket. Its assertions cover typed routing,
enrollment continuity, and one-time issuance. Actual old-carrier EOF,
same-VM reconnect, Stop ordering and original deadline enforcement belong
to the Runtime replacement acceptor.

Hostd wiring is included for source review. Its separate test run was still
in progress when this checkpoint was cut. No guest was launched by this test.
It yields no WAN, throughput, or cryptographic qualification evidence.
