# Bounded clock issuance: native failure and regression

Date: 2026-11-02. Tracker: #43. The first current-service Firecracker attempt
used production source from `8e13e45` (the later `40793fc` changed evidence only).
Its host executable SHA-256 was
`28c7a9fb4b1f8df5a73c62cddce90561b43b31138652115c48a80abb022656c3`;
guest SHA-256 was
`1e7eb3d044f974a02afa6a02483db58196acc4c751cecd099d1fff7131bd0eca`.
The failed run remains at Wick
`/var/tmp/spiralcore-mvm-service-20261102/native-pilot4`.

The VM booted, emitted `guest_ready` and `guest_connected` on mailbox port 5257,
then reported `guest_done.ok=false`, phase `sender_campaign`, with canonical
workload exit code 1. It is a failed native session, not a successful transfer.
The parent reported `Unavailable` and its worker reported an unsuccessful
workload exit. The owned VM was stopped and the temporary Firecracker link was
removed. No other lab process was stopped.

Observed guest UTC was `1793658536000`; host session UTC was `1793658536828`.
The guest clock restore has whole-second resolution. The new service used exact
producer timestamps rather than the earlier live path's bounded backdate.
An independent deterministic test reproduced strict canonical rejection of a
host-issued envelope at 1900 when the receiving guest time was 1100:
`MailboxError::IssuedInFuture`. A service Call was admitted on the host but the
host's encrypted receipt was rejected at the guest. This isolated a concrete
clock defect; successful native retry remains its separate execution gate.

The correction keeps `Exact` as the existing API default and introduces an
explicit `Bounded` issuance policy, limited to 5000 ms. Native provisioning opts
into that existing live clock bound for Calls, Replies and encrypted receipt
Casts. Timestamp construction is shared; no canonical verifier, crypto primitive
or envelope format changed. An immutable message's issued/expiry values are
chosen once and reused on retry.

Issuance fails if the full backdate precedes either signed grant or route
validity. Expiry is capped by the requested expiry, both signed validity ends,
and `issued + 30000`; expiry must remain after actual current time. The native
Call deadline is also clipped to that expiry. Receipt preflight happens before
application admission, and its signed observation time remains actual current
time. Receiver clocks are never shifted to accept future-issued values.

Independent `service_clock` validation passed **4/4**: strict future rejection,
a full four-envelope cycle with exact trailing-zero payload and an 800 ms
bound, refusal at 801 ms, expired-message refusal, immutable retry headers,
and young reverse-grant refusal before application admission. Strict focused
Clippy passed. The parent reran all adapter features/targets: **82 passed**,
zero failed, ignored or filtered tests. The new four-test target is required on
all seven native OS rows; earlier 46-test-per-row artifacts remain unchanged.

| Reviewed file | SHA-256 |
|---|---|
| `issuance.rs` | `cdcf2982e1a742592d28dabeb74ac20f9bf0436c0d9d7583e567e22edc11f7e3` |
| `receipt.rs` | `ad2585e37009e81196e98393ab640ccbebb3f72c4b5d559dc62efaab337b02a2` |
| `service_sender.rs` | `812ea62f6205d27f1a956a0baffcb7127883d6be6cb2e4b3192f0682474ed261` |
| Native `campaign/canonical.rs` | `303a036916a665ac1bb810d187cb9d70076de24383a4de4503c3769cc0bf1277` |
| Independent `service_clock.rs` | `f3f2cd2d7e22a43a029b481c683ac48e3f1aa7a6114bad72b35e5230b3788296` |
