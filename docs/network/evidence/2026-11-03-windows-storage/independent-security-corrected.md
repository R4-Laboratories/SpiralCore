# Windows storage P1/P2 correction

Disposition: **PASS for the source correction; native runtime gate remains
NOT_RUN.** Supersedes only the two source blockers in security-source-review.md.

Inspected windows.rs SHA-256
`db97d858103a0c4ca0cf4272691de9f730ff4e0c2d19113332cee54a8726a77a`
and tests.rs
`0cb2c6f7453aac92573d67570fe379cef9c5b191f5fd977d67a28d925d22d9ae`.
No build/test or source change performed by this review.

Lines145-152 now require the original Disk/VerbatimDisk prefix plus RootDir
before normalization or filesystem access, so stored endpoint roots cannot be
redirected by changing CWD. Lines153-170 bound original UTF-16 input and refuse
NUL, parent components and trailing dot/space aliases before absolute() can erase
their spelling. Original normalized path, component, volume, ancestor, ACL and
actual-handle checks remain. Both earlier source blockers are resolved.

The new Windows-only test checks relative/drive-relative/root-relative typed
refusal and traversal/trailing-alias refusal without creating resolved targets
or changing the profile. It avoids global chdir. Compiler-only success is not
execution of these tests. Require actual native positive-count storage and
endpoint targets before claiming supported Windows storage. No Windows MVM,
WAN or device power-loss qualification follows from this source pass.
