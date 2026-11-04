# Original Windows path refusal correction

Independent source review found two issues before native release acceptance:
`absolute()` could remove a parent traversal component before the existing
post-normalization check, and Endpoint retained an original relative root
while individual operations resolved it against the current working directory.
The latter could redirect operations while the owner lock still belonged to
the earlier root. These source findings were not native runtime reproductions.

The corrected preflight runs before normalization or filesystem I/O. It requires
an original Disk/VerbatimDisk prefix followed by RootDir. It bounds original
UTF-16 to 4096 units and rejects NUL, parent traversal, and any original segment
ending in a dot or space. Existing normalized prefix/component/length checks,
handle ACL/type/link checks and ancestor sharing remain in place.

An additional Windows-only semantic test requires refusal for relative,
drive-relative, root-relative, traversal and trailing-dot/space inputs, with
the owned initialized profile unchanged and no alias child created. It never
changes the process working directory. The test was written before this small
correction; its intent hashes are in `validation/path-review-test-intent.sha256`.
Native Windows execution of this test is **NOT_RUN** here, so this is not an
observed RED/GREEN runtime claim. The native Windows selected storage target
now contains six tests; Unix remains three.

The exact updated backend passed the same MSVC-target compiler-only strict
Clippy check, exit 0, in `validation/windows-path-review-clippy.txt`. That
adapter does not compile the Windows test module or full Endpoint dependency
graph. The earlier full-crate cross-check remains blocked by absent `lib.exe`.
Current seven-file pins are in `validation/candidate-source.sha256`; the earlier
candidate and all raw failures remain preserved separately. Native CI and final
independent review must qualify this corrected source.
