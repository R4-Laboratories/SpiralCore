# Retained telemetry capacity prerequisite — scoped review

Source PASS, 2026-11-04, for the first capacity prerequisite only. Reviewed the
actual two-file delta against immutable 7a080af Git blobs; no source edits,
builds, test execution or expanded retained-owner audit were performed.

Exact reviewed sources:

- endpoint.rs: 5bc34078cfc35eceb2dd7a4436e32af85961e36bd0a0a404632ccefaf982c937
- endpoint_tests.rs: 66a21335edce48680333a05b6f1643751054ef2c9d546bb14e67d0b63b2600dd

The added comparison uses the canonical MVM DEFAULT_RECORDS_BYTE_CAP (8,388,608
bytes), with no rounded literal, subtraction overflow, added quota or fallback
to spare host capacity. QualifiedEndpointProfile::claim is called before the
lifecycle changes NeverIssued to Preparing and before endpoint reservation or
disposable inventory preparation. Therefore an undersized TOTAL cache returns
without consuming issuance or publishing an uncertain owner. The new check is
read-only arithmetic under the existing lifecycle lock; it adds no filesystem
I/O, nested lock, allocation owner, callback or drop behavior there.

The regression calls the real parent preparation path with the existing private
policy fixture. It requires Err, NeverIssued, unchanged managed usage and equal
original installation directory entry names, followed by successful normal
unissued cleanup. This is policy/storage evidence rather than opaque-admission
or native execution proof. The positive fixture now explicitly funds R+65,536
and retains the original one-shot, acyclic-owner and unproven-resource quarantine
assertions, with expected usage updated to that TOTAL. No assertion was removed
or relaxed to admit the undersized case.

The specification and plan accurately call this a prerequisite: passing TOTAL C
unchanged to disposable inventory remains a known bug; C-R separation, retained
R ownership in the SAME ledger, actual canonical file/cleanup protection,
durable catalog/restart adoption, and native activation remain OPEN. There is
still no production profile constructor. This review does not approve retained
file deletion/refund, a larger host/tenant budget, a native result or completion
of the full MVM program.

Root reports an executed prior RED (release build0, regression101) and current
GREEN41545 is pending at review request time; this source verdict does not claim
independent receipt verification or a completed execution/Clippy gate. Those
qualification results must retain their actual source/binary identity before
publication. No blocking code, Rust-lifetime or security defect in this narrow
capacity check.
