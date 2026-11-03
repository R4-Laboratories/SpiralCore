# Holder handoff: focused CODE/RUST and count-only INFRA

**PASS — no blocking finding in the additive export delta.** Reviewed
SpiralCore88f4ea56eb3b7409d09af77faef916f3d9f157d8 against merged e71f4080;
contract843c6c95 precedes test5d2cc3d8 and implementation88f4ea56. This reuses
the published owner review; it does not re-audit the old component or MVM hook.
No product edits, builds or tests were performed by this reviewer.

The acquired generation is stored privately in HolderLease. prepare_handoff
freshly uses the existing canonical storage reader while that guard still owns
SH, refuses non-Active/different original generation, and never adopts a newer
record. That reader preserves existing key, directory/lock identity, marker,
record-size/checksum and canonical-encoding checks. The verified176-byte record
has a unique re-encoding, so SHA-256(record.encode()) is the full current record
digest, distinct from its existing domain checksum. Existing176 encoding itself
is unchanged.

File::try_clone duplicates the held directory and holder descriptions; it does
not reopen a pathname. Actual duplicate IDs are compared with pinned IDs.
Clone/validation errors return no handoff and do not release the original SH.
The non-Clone result owns Files; dropping the original guard leaves its duplicate
holder lock until final close. CLOEXEC is independently asserted by the new
positive test. Public owned fields support trusted local conversion, not guest
admission, arbitrary generation selection or a claim that the MVM consumer is
already wired.

The two new cases are focused and preserve old assertions: exact directory/
holder IDs, independently read full-state digest, descriptor flags, Busy after
original guard drop, then successful finish after handoff close; and fresh
generation/phase/checksum/root-replacement refusal with unchanged recorded bytes,
independent EX contention while the guard lives and EX success after final close.
The test-only valid-checksum changes are deliberate negative controls, not a
production alternative parser or authority bypass.

The workflow diff changes only four lifecycle counts9/10→11/12. Unit4, runners,
architecture expectations, read-only permissions, pinned actions, timeout,
serial policy, target scope and failure guards are unchanged. Expected hosted
aggregate becomes62 entries including four subprocess helpers; this is not a
claim that those four new hosted rows have executed. Harness README changes
likewise add only the two actual lifecycle cases.

Read local owner-handoff-macos output/exit:4 unit+12 lifecycle entries pass,
zero failed/ignored/filtered, one helper included; focused strict Clippy exits0.
Root reports Linux4+11 serial and strict Clippy PASS and a missing-method RED at
5d; those Linux/RED receipts had not yet been copied into this packet at this
review, so they are not asserted as independently read here. Source inspection
of5d confirms prepare_handoff is absent before implementation. Remaining exact-
source qualification/custody is separate from this source PASS; no new run is
requested by this review.

Reviewed SHA-256 pins:

```text
fc0b3dd44256da7bcfd7cdc793e77f0312ce421d365d6e295fda68ce17225279  crates/spiralcore-mvm-live/src/installed_owner.rs
b195539a984ba10d5e89c1955a33a367fdf9efa259c48a49f806e1ba50539541  crates/spiralcore-mvm-live/tests/installed-owner-harness/tests/lifecycle.rs
def209157be594f533dad9ad2edf431ccfaedf8ffabf59412e3a36f642932b7b  .github/workflows/spiralcore-installed-owner.yml
7da12708c64d7f8fb39918012f206f358833d3eb671026ce611934e197c42a28  docs/api/installed-host-owner-v1.md
```

Actual installed consumer conversion, worker and privileged VMM lifetime,
death/recovery/readmission and full B6a/B6b remain open. No native/Windows/WAN,
scale, energy or new transfer-performance claim follows. Only this review was
authored; existing published evidence is unchanged.
