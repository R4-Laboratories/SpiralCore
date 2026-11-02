# SC-MVM-004 implementation plan

Approved scope: current Combined selector through the existing real MVM witness,
with matched persistent object sessions and exact reboot readback.

1. In `crates/spiralcore-mvm-live/tests/campaign.rs`, add failing coverage for
   explicit arm parsing, identical payloads, Combined receipt mechanisms, and
   restart/readback. Risk: medium; no production edits precede observed RED.
2. In `src/config.rs` and `src/main.rs`, add explicit opt-in arm names, keeping
   legacy arm semantics. In `src/fixtures.rs`, add bounded canonical mixed
   fixtures matching the gateway's public generation recipe. Risk: low.
3. In `src/campaign.rs`, route Combined preparation, checkpoint/schema seeds and
   sends through the shared `ObjectSender`; retain selection receipts and actual
   mailbox counters. Depend on steps 1-2; risk: medium.
4. Run focused legacy/new campaign and authenticated interruption/reconnect tests.
   Check full exact bytes after endpoint reopen and absence of partial publication.
   Record RED/GREEN receipts. Depend on step 3; risk: medium.
5. Parent builds pinned host/guest binaries, records identities and runs one guest,
   then two guests, three paired rotated trials per size and arm at one explicit
   shared bandwidth profile. Readback reboots use no mailbox credentials. Depend
   on step 4; risk: medium, controlled by existing fresh-output and disk guards.
6. Retain compact evidence and source-linked comparison of bytes, sender/receiver
   CPU, elapsed time, setup, and limitations; independent focused code/security
   review before publication. Depend on step 5; risk: low.

No core compression algorithm changes or broad historical audits are required.
