# Reviewable component source identity

Uncommitted new standalone crate; no branch change, commit, push, VM operation
or MVM fork edit. These are component qualification receipts, not issue44 closure.
All source/test files in this task are within the lead's explicit owned scope.

Actual same-source release result:20 PASS,0 failed/ignored/filtered, across four
test executables (8 duplex,1 mapped stream,4 object-v1,7 baseline TCP). Library
and doc-test scopes each have0 tests. Strict all-target Clippy exited0 without
warnings. Logs/exit files are retained under validation/, including every earlier
failure. No full shared-core test count or branch/line coverage claim is made.

Source manifest SHA-256:
`b90a5458b686f53dddb31bd8ef523ec73ff40dd786dfcd656bec3902182e786a`
for `validation/source-component20-sha256.txt`. Its12 own crate entries were
checked against local bytes and all passed; the remaining entries identify the
read-only shared dependency snapshot copied to Wick. This does not qualify
unrelated current-worktree edits to those dependencies.

| Production source | SHA-256 |
| --- | --- |
| src/lib.rs | 69379c2eed48d4355305cb47844ed14295b6aad7c40294da715cc8aeb2fd48b1 |
| src/tcp.rs | 476aa58a911558c531f16302b9a6945f25f649e737edca0f27c734677e301569 |
| src/record.rs | 578c1e049a18c50ed3e5d2f0369fdcdc0985c1fc68aef416bad9f483a0ed46aa |
| src/object.rs | b10e842d94edef4fede8f61a06c046166fe4973a4252bfccf9c356aacb857a41 |
| src/duplex.rs | ef0d404da24708ba689ac777bed7903d614ad97946bed84d014bae6b9eb4cc62 |
| src/stream.rs | 157d4319adb132d36bda77acccb3bfdaaf13b839051b81fa686ce23764090624 |

Contract SHA-256:
`33c4821642ca048611ac240c255a3f618ebd1c0def1528fee6fe197de999c69a`.
Its initial form, shared-core amendments, duplex/direction and mapped-stream
semantics were written before their corresponding product steps. API absence
and semantic REDs are distinguished in the packet README.

Independent source/security review and final exact integrated-source release
qualification remain pending. The lead owns those gates and the canonical
MVM hook approval. Actual signed-policy selection, TLS peer authentication,
canonical rate/window/transform/audit/reset integration, UDP/ingress, supported
native backend/platform, matched CPU/lifecycle/wall/RSS/WAN and scale gates
remain open. The std TCP constructor verifies a tuple, never peer identity.
