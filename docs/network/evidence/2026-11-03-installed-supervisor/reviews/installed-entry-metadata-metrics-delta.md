# Installed entry metadata/metrics delta

Disposition: **PASS for this narrow source delta**, retaining the earlier
installed-entry-security-initial.md limits and excluding the authored channel
from this reviewer's independent verdict. No execution or source edit.

installed_owner.rs SHA-256
`94006668298fbb172ae52e23105db51b57a44aa881ae6a84c556e0f85991100c`
only makes InstallationBinding and active_binding publicly available with an
explicit snapshot/noncredential contract. Canonical current() and original
Active Epoch/key/generation checks remain unchanged. No owner, Arc, Mutex,
descriptor or slot grant is exposed. Mutating an owned returned DTO grants no
activation authority; the actual worker channel still needs the private live
controller grant.

Host source pins remain the previously reviewed values:
host.rs `c53f8a66c3fa9ef80d98371a63430ced4f00080b9e2eb678356248be00d1f883`;
linux_host.rs `67bbace5f601eb31fd56f4989c99d315f7953fe1f1a00f68c12e1253f95a7df6`.
The installed wrapper's total process CPU/wall start before control/input and
replace the narrower inner total. Control/input subfields overlap these totals
and are not additive. Successful installed generation/role fields derive from
the retained context, with no nonce/initramfs/credential in the added metadata.
No normal-path performance benefit or full native lifecycle qualification is
inferred. Final exact-source CI and native recovery evidence remain separate.
