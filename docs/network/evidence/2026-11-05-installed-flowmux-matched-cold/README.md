# Installed FlowMux cold comparison across a private LAN

One signed Linux x86-64 Firecracker guest sent the same 11,600-byte
repetitive fixture to a native arm64 macOS peer in each arm. The Mac
sent an exact 11,600-byte echo using Whole in every arm. Both sides
independently verified the application bytes. The peer joined its
reader and removed its store. Wick verified original VM and endpoint
retirement and zero guest and staging reservations.

The isolated qualification build selects Raw, Whole or Combined on
the Wick installed endpoint at compile time. It leaves the original
production default and runtime policy interfaces intact. All three
runs use the same guest image and test payload, original SmallV1
admission, resource limits, signed destination and peer identity
rules, authenticated TCP/TLS, exact readback and lifecycle path.
Each run independently generates its own listener, TLS identity and
signed launch. The Mac verified that the two setup artifacts were
absent in Raw and Whole and reopened both exactly in Combined.
Whole uses the shared SmallV1 prepared zstd encoder used by
Combined's Whole candidate. The profile uses level 3, a 17-bit window,
the fast strategy, and checksum and content-size fields.

| Measurement | Raw | Whole zstd | Combined |
|---|---:|---:|---:|
| TLS bytes, installed endpoint to Mac | 14,148 | 2,652 | 4,321 |
| TLS bytes, Mac to installed endpoint | 2,581 | 2,581 | 3,869 |
| TLS bytes, both directions | 16,729 | 5,233 | 8,190 |
| Verified setup artifacts | 0 | 0 | 2 |
| Mac wall time after accept | 262.063 ms | 252.698 ms | 306.314 ms |
| Mac application-thread CPU | 7.218 ms | 7.561 ms | 7.964 ms |
| Mac process CPU | 13.339 ms | 16.392 ms | 24.764 ms |
| Wick witness process CPU | 977.973 ms | 962.006 ms | 976.434 ms |
| Wick supervised lifecycle | 2.035 s | 2.011 s | 2.045 s |

Combined used 51.0% fewer TLS bytes than Raw and 56.5% more than
Whole. Both Combined directions include the setup exchange and its
acknowledgments. The arm's configured policy does not establish
which candidate encoded the data blocks. TLS accounting includes
handshake and carrier framing, and excludes guest vsock and SSH.
Mac wall includes verification and peer cleanup. Wick witness CPU
covers its own process and is not the installed codec worker's CPU.

The original helper failed the Raw oracle after the peer reopened
an unexpected setup artifact and confirmed its exact content.
The original owner retired the guest and endpoint. The first
benchmark build failed on a missing ObjectMode import. The corrected
successor built all three pinned helpers and all native trials
exited zero at both ends. The SDK tree comparison shows one
benchmark-only source difference from the previous composed SDK.

Apply the peer delta to the
[previous cross-host fixture](../2026-11-05-installed-flowmux-crosshost/README.md).
Build each isolated Wick helper with
`SPIRALCORE_FLOWMUX_QUALIFICATION_ARM=raw|whole|combined` and the
existing locked SDK dependency overlay. The execution driver pins
the selected helper and the unchanged companion helpers. The
packet includes source deltas, driver hashes, sanitized records
and the exact results. Private orchestration remains outside this
packet.

This is one serial cold trial per arm on a private LAN. It records
the setup cost of the current installed path. A sustained series
with independently registered workload order, larger varied data,
actual codec-worker CPU and real WAN conditions remains open.
