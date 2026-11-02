# Encrypted canonical receipt component qualification

Date: 2026-11-02. Scope: the proposed receipt-control mapping and bounded source
tracker, tested with canonical MVM crypto/transport APIs. This is component
correctness evidence, not WAN, production mailbox, platform parity, performance
or security-certification qualification.

The component was developed in the isolated `codex/spiralcore-mvm-network`
worktree. The containing checkout HEAD at this test checkpoint was
`3e9226403f65652f7bf47e207a594b451344343b`; the new component files were uncommitted
and are pinned by their exact SHA-256 below. The final delivery records their
containing committed source independently.

| File | SHA-256 |
|---|---|
| `crates/spiralcore-mvm-adapter/src/receipt.rs` | `d5faf9945cb591218c098459ef812e1fc935ac03f976c84cc7cd8cbb8a112b9a` |
| `crates/spiralcore-mvm-adapter/tests/receipt.rs` | `41e76d608e7fd84fde0ca38bf0a54ed78d498eb913b818e641327d3f4b550bba` |

The final component command was executed on macOS ARM64 using Rust 1.97.1:

```sh
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
/Users/casey.allard/.cargo/bin/cargo +1.97.1 test \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --features live --offline --test receipt \
  --config /Users/casey.allard/spiralcore/local/mvm-service-20261102/patch.toml
```

Observed result: **15 passed, 0 failed, 0 ignored**. No instrumented or test-run
elapsed time is represented as transfer performance. The prepared MVM package
uses base `6ae461a54d85a1b70477c1efb97ea4a33ef3b331` with the retained
service patch, as recorded in that prepared package's `provenance.json`. Its
parent historical patch used `3104a12c5b071a6851323fa8115d2afaf3f0bb8b`; that
parent is not the dependency identity for this final component checkpoint.
The published equivalent is fork commit `5766754fbaae59dbee548f968716a514d3a796c1`;
the release owner separately reruns the locked default-fork build without a
local path override. Full package provenance is recorded by the native-build owner.

Passing tests:

1. `canonical_receipt_codec_preserves_existing_signature_and_rejects_malleable_input`
2. `generic_application_admission_cannot_acknowledge_reserved_receipt_control`
3. `ordinary_reply_has_destination_receipt_without_reply_of_reply`
4. `full_receipt_consumer_queue_can_retry_after_consumption_without_false_duplicate`
5. `prepared_receipt_refuses_backward_clock_and_policy_change_before_transport`
6. `integrated_admission_owns_receipt_credit_before_accepting_and_retries_same_message`
7. `encrypted_receipts_match_attempts_and_do_not_recurse_or_lose_positive_facts`
8. `delayed_negative_same_attempt_cannot_republish_consumed_positive_receipt`
9. `application_reply_and_admission_are_independent_and_late_replies_cannot_complete`
10. `receipt_tracker_rejects_wrong_signature_commitment_causation_and_policy_generation`
11. `attempts_and_shared_identity_quotas_fail_closed_and_release_owned_capacity`
12. `receipt_replay_and_prepared_resend_obey_signed_control_rate_without_ack_loops`
13. `receipt_return_reserves_before_admission_and_sends_only_canonical_encrypted_cast`

14. `receipt_return_rejects_undersized_reverse_grants_before_application_admission`
15. `integrated_receipt_admission_rejects_a_different_budget_owner_before_queue_mutation`

Independent security review identified two further admission hazards. The new
negative tests first failed with 13 existing tests passing: an undersized
reverse grant was accepted at configuration time, and a return configuration
could use a different aggregate budget owner than its application service.
The fixed return constructor pins the exact 4096/16384 profile; integrated
admission checks shared owner identity before mutation. Both regressions now
pass, and the budget mismatch case proves the request remains newly admissible
after rejection.

RED was observed before the receipt module, tracker, application-reply observer,
return configuration and integrated admission APIs existed. Additional failing
regressions exposed backwards monotonic time acceptance, the generic admission
path acknowledging reserved receipt control, duplicate control bypassing signed
rate limits, and an old negative observation allowing a consumed positive
receipt to reappear. Those cases subsequently passed. The return carrier also
hit and corrected the canonical ordering-scope 64-byte bound; receipt scopes
are now unique 64-character message-ID hex with sequence one.

Strict Clippy passed after these review fixes using the same environment and
config with `cargo +1.97.1 clippy --features live --offline --lib --test receipt
-- -D warnings`. This covers the library and receipt test target, not every
release target or platform. The release owner reruns full integration checks
after all source is frozen. `git diff --check` passed for this checkpoint.

The component emits only canonical encrypted Cast envelopes through the
canonical authorized `PeerTransport` gate. It defines no raw transport ACK or
replacement receipt signature. The tracker returns local observations and
never sends receipt-of-receipt traffic. Sender-visible signed acceptance and
application Reply are separate facts. Default and raised native resource
profiles, exact byte readbacks, actual bidirectional traffic, CPU and lifecycle
measurements belong to the enclosing service campaign, not these unit tests.

Independent interoperability review added `tests/service_epoch.rs` and observed
two failing cases for a canonical Call addressed to B at epoch 8 and Reply
addressed to A at epoch 7. The receipt matcher removed only the incorrect
equality across these distinct destination epochs; current reverse-binding
verification and original-message signed receipt epoch matching remain.
Rerunning the command above with `--test service_epoch` in addition to
`--test receipt` passed **15 receipt tests and 2 epoch interoperability tests**.
The reviewer owns the independent epoch test source and its final identity.
