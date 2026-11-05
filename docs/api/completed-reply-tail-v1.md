# Completed Reply tail extraction v1

Additive adapter API contract, Stage1 of the completed-tail design. Canonical
MVM envelopes, signed authority, ordering, strict poll/retry APIs and all limits
remain authoritative. Consumer recovery is qualified separately under
[completed-session-tail-v1](completed-session-tail-v1.md).

```rust
pub struct CompletedCallReply {
    pub payload: Vec<u8>,
    pub tail: CompletedReplyTail,
}
pub struct CompletedReplyTail { /* private, non-Clone */ }
impl LocalMailboxService {
    pub fn take_call_reply_with_tail(
        &mut self,
        caller: &MailboxBinding,
        replier: &MailboxBinding,
        call: &CallHandle,
        wire: &mut Option<Vec<u8>>,
        now: ServiceTime,
    ) -> Result<CompletedCallReply>;
}
impl CompletedReplyTail {
    pub fn wire(&self) -> &[u8];
    pub fn call(&self) -> &CallHandle;
    pub fn validate(
        &self, caller: &MailboxBinding, replier: &MailboxBinding, now: ServiceTime,
    ) -> Result<()>;
}
```

The caller owns wire before extraction and can recover that same Vec on refusal.
On success it moves into the opaque tail; payload moves out exactly once. No
retained plaintext copy, wire clone, new arrival, identity, sequence, active Call,
independent quota or send permission is created. No permit borrows are stored.
Read-only wire access conveys no current authorization; canonical admission and
send validation remain required for every future use. The tail cannot produce
an acceptance fact, RemoteComplete proof or application result a second time.

## Original ownership and authorization

Run the existing clock/expiry/FIFO drain before selection. Require exact original
service/Call handle, both current bindings and generations, the live mailbox
owner, unconsumed Replied(Some), original Call and Reply Unix/owner-monotonic
deadlines and no service shutdown. Acquire independent policy locks in the
existing immutable binding-ID order, retaining them through extraction. Do not
consume queued FIFO work early or turn an unknown gap into completion.

At successful queued application-Reply admission, retain only a32-byte digest of
the exact authenticated encoded input and its length alongside the existing
accepted record. Calls/control traffic need no new wire digest. Canonical drain
records that Reply key on its matched PendingCall. Extraction compares wire
length/digest to that exact accepted Reply and its original incoming binding,
generation, owner and deadline. No new decryption/plaintext verification copy is
needed. Duplicate admission cannot replace the original digest or lineage.

The tail retains original Call handle, both binding identities/generations,
mailbox owner/live references and fixed deadlines/floor clock. validate rechecks
fresh binding authority, original generation, owner/liveness and original caps;
it grants no transport access and performs no expiry extension. Service Drop,
shutdown, takeover/revocation or original expiry make validation refuse. Payload
extraction remains a completed local observation, not a rollback on later refusal.

## Shared accounting transition

Source correction: existing reply_budget charges encoded Reply length W, not
plaintext length N. Admission sets queue bytes to encoded_len; canonical drain
moves that exact reservation into PendingCall. Require N<=W before transfer.
That original W charge covers its queued plaintext but cannot by itself cover
simultaneous queued plaintext and the new retained ciphertext allocation. Let C
be the supplied Vec's capacity, which may exceed its authenticated length W.

While the original W/one-message reservation is intact, atomically reserve an
ADDITIONAL actual C bytes and one queued message in the SAME HostBudget, Data
pool, authenticated incoming Reply scope and signed capability contribution.
Thus conservative overlap W+C covers N+C. The shared host/tenant/mailbox/sender,
Data/total byte/message and signed byte-contribution ceilings remain enforced.
Signed arrival/rate ceilings remain unchanged. Additional
reservation claims zero arrivals/rate entries/identities/active Calls. No relay
frame reservation or release/reacquire gap is permitted.

On reserve failure return Full with original Call, unconsumed payload, wire and
old reservation unchanged. On success perform only infallible moves: consume
wire/payload, install the new C/one-message reservation in the tail, release old
W/one-message charge once. Ledger snapshots may observe conservative overlap;
they must never observe an uncharged retained wire. Reservation RAII owns Drop
release once even when the service or bindings are dropped first. Lease moves do
not duplicate charges; no Clone or uncharged into_wire escape is provided.

An expired/fenced tail still physically owns bytes, so its reservation remains
charged until explicit retirement/Drop. Service expiry cannot refund escaped
live bytes. The embedding must drop on fence/expiry/close; existing shared limits
bound abandoned leases by bytes/messages. This is process-shared service-budget
enforcement, not installed host-wide/C9 qualification.

## Errors and focused gates

Reuse ServiceError: Pending/FIFO-held => Backpressured; consumed result => Conflict;
missing handle => UnknownHandle; wrong handle/binding/owner/generation/wire =>
Unauthorized; expired => Expired; missing wire => State; oversize => Limit;
shutdown/terminal statuses retain existing typed equivalents. Clock maintenance
keeps existing precedence. Maintenance may expire canonical state; a Full refusal
does not consume state or replace identity. Strict poll_call stays byte-for-byte
in behavior and releases its reservation normally.

Required RED/GREEN tests use actual signed/admitted Call+Reply:

- W>N, original charge retained, overlap W+C capacity refusal BEFORE extraction;
  strict poll afterward still yields exact original payload once.
- Successful extraction moves the same wire allocation, returns payload once,
  leaves exactly C/one-message tail charge and releases exactly once on move/Drop.
- Overallocated authenticated wire has W fitting but C exceeding capacity:
  Full preserves the exact original allocation/pointer, payload and old W charge;
  no hidden clone/shrink/reallocation or relaxed ceilings are permitted.
- Shared host, tenant, mailbox, sender, Data and signed contribution ceilings
  each refuse with original unconsumed state; no second arrival/rate/Call claim.
- Wrong/stale handle, different authenticated Reply/wire, policy/owner takeover,
  deadline/clock, Pending/FIFO, consumed and shutdown refuse without substitution.
- Strict poll unchanged; tail after service Drop/expiry remains charged until
  Drop and validates as refused. No new source claim of automatic retry, closed
  stream/crash resume, completed-tail repair or >2048 session capacity.
