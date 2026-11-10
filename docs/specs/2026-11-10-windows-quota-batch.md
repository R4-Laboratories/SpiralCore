# Windows quota scans through owned directory handles

Status is an architecture decision for the PR #106 Windows performance repair. A handle-relative implementation is approved for focused implementation and native qualification. Performance improvement remains unproved.

The Windows Server 2022 run 42894601561, job 130104398191, completed the 129-block case in 5.78 seconds. The sustained case reached 2,626 and 2,619 blocks before its 240-second watchdog, with 64 publications, one attempt and 1,051,712 cache bytes per side and current scopes. Counts, deadline and authorization remain acceptance gates.

The first strict-ancestor/GUID-path prototype, source a310, failed its native gate. All Windows 129-block cases closed after two blocks at about 0.17 seconds with live scopes. Three of twenty portable endpoint tests failed during parallel endpoint opening. Pinning shared ancestors against writes is incompatible with independent sibling temporary-store creation. Retain this failed result. MSVC metadata compilation and Clippy from that prototype do not qualify its native behavior.

`Endpoint::disk_usage` scans publications at each admission. Windows `quota_file_len` previously called `open_read` for each file, repeating normalization, ancestor opens, volume checks, effective-SID lookup and security-descriptor work. Reduce repeated setup without making retained quota metadata authoritative.

## Decision and alternatives

Reject a cached quota index for this slice. External growth, replacement, ACL changes and disappearance would leave an index insufficient for admission. An authoritative index requires a separate enforced immutable-storage and reconciliation contract.

Replace the failed absolute-path batch with a handle-relative scan. Keep the public API as one Windows-native synchronous `quota_usage(root, scoped_namespace, max_files, namespace_filter, extension) -> Result<(u64, u64, usize)>`. Its trusted noncapturing namespace predicate preserves current canonical namespace selection. Return scoped bytes, aggregate bytes and publication count. The Windows transfer mapping delegates to it. Unix retains its current implementation.

A private `QuotaRoot` retains the original effective SID, selected volume, and original owned-root handle. It performs namespace opens relative to that root, publication opens relative to each namespace, and enumeration on the held directory handles. No path, identity, iterator, handle or content reader escapes. Drop the scan before mutation or the next operation.

## Acquisition and access

Accept only the existing validated drive-absolute local NTFS root input. Preserve existing normalization, original metadata ancestry checks and volume checks during initial root acquisition. Open only the owned root with `FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL`, share-read, and reparse refusal. Validate its held regular-directory identity, original volume and current-SID protected private ACL. Release traversal-only ancestor handles after acquisition. All subsequent operations use the retained owned root, so later drive-letter or ancestor changes cannot redirect this scan.

The older attribute-only ancestor handles provide metadata checks. Their sharing masks alone do not establish rename or deletion exclusion. List-directory access on the owned directory participates in Windows sharing enforcement. The new scope pins owned storage directories while permitting unrelated stores beneath shared ancestors to proceed. [Microsoft sharing algorithm](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/8c0e3f4f-0729-49f4-a14d-7f7add593819)

Use reviewed `windows-sys` native bindings for `NtCreateFile`, its object attributes, Unicode name and result types. Set the actual retained parent as `RootDirectory`. Require each supplied name to be exactly one validated normal basename, with no slash, backslash, colon, NUL, traversal, trailing dot/space, wildcard, device form or alternate stream. Bound UTF-16 length and checked byte conversion. Use `OBJ_DONT_REPARSE` and `FILE_OPEN_REPARSE_POINT` so reparse results refuse rather than redirect. Use `FILE_OPEN`, never create, overwrite or repair. Validate NTSTATUS and returned handle before transferring ownership, closing any returned failure handle correctly.

Namespace opens request list-directory, attributes and READ_CONTROL with directory-only synchronous flags and share-read. Publication opens preserve the existing generic-read and READ_CONTROL rights with share-read, non-directory synchronous flags and reparse refusal. Include SYNCHRONIZE where synchronous open options require it. Convert these internal native handles into one RAII owner each. Preserve held-file type, link-count, volume and private-ACL checks before reading length. Content reads retain their existing implementation.

Query the effective SID for each namespace and publication operation and compare it with the constructor SID. Refuse a change. Preserve process-token fallback solely for `ERROR_NO_TOKEN`, and preserve caller impersonation. Check the root and namespace private ACLs and directory kind during the scan. No process SID cache or impersonation reversal is permitted.

## Handle enumeration and bounds

Use `GetFileInformationByHandleEx` with `FileIdBothDirectoryRestartInfo` for the first request and `FileIdBothDirectoryInfo` thereafter on the original root or namespace handle. Use reviewed binding definitions for records, avoiding handwritten native structure layouts. Parse a fixed bounded aligned buffer with checked offsets, name byte lengths and record boundaries. Handle end-of-enumeration explicitly and refuse malformed records, unexpected status and unsupported identity formats.

Skip dot entries. Apply the current canonical namespace predicate to immediate directory names and the current object extension filter to publication names. Open each selected entry through the actual enumerated parent handle. Compare the returned file's 64-bit identity and volume with the enumerated identity and original volume. Refuse zero or unsupported identities rather than using them as proof. Directory/file/reparse mismatches refuse the operation. An identity mismatch or disappearing selected file is a storage error, never a zero-length entry.

Use one namespace handle and one publication handle at a time in addition to the root and fixed enumeration buffers. Avoid collecting records or filenames in vectors. Preserve checked sums, publication count and the caller's maximum. The scan observes live storage under the endpoint's existing writer ownership. It does not introduce an atomic filesystem snapshot or a stronger threat model for arbitrary privileged mutation.

## Qualification

Extend the two native Windows ACL and junction suites. Cover exact multi-file totals, broad/inherited ACL rejection on a later child, hardlinks, reparse children, incompatible writers, invalid handle kinds and controlled anonymous impersonation. Pinning the owned root and namespace must block their rename during the scan and permit it after release. Add root/namespace replacement and drive-alias change controls proving subsequent operations stay on original handles or refuse.

Add bounded enumeration parser cases for truncated headers, malformed next offsets, overlong or odd UTF-16 lengths, zero identity, identity mismatch and end-of-enumeration. Exercise native enumeration over more entries than one buffer. Prove parallel sibling store creation and endpoint opening succeed, directly regressing the failed strict-ancestor prototype. Record unavailable privileged negatives explicitly.

Reuse core quota tests for scoped and aggregate totals, growth between scans, exhaustion, overflow and publication count. Run all twenty endpoint cases together to exercise parallel creation. Preserve ordered retention and Small resource tests for exact publication, setup pins, retries and eviction.

Record six paired scans over 64 files using the old per-file path and new native batch, including exact totals, CPU and wall time. Run the supported native Windows primitive suites and seven-role component matrix. Repeat 129-block and sustained workloads with the original count, watchdog, authority and byte checks. Record measured speedup only after those native gates pass.

## Completed-open invariant

The selected local NTFS path requests synchronous nonalert opens with
SYNCHRONIZE. An observed STATUS_PENDING must not return or unwind output/input
storage still potentially referenced by I/O. This implementation fail-stops
with process abort before any such release. It offers no asynchronous-open or
timed-cancellation fallback. A child-process test injects the status only after
an actual completed call and checks the fail-stop marker and exit, then retires
the owned sandbox after that original child exits. Real pending handling emits
no potentially panicking diagnostics before abort.
