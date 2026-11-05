# Installed mailbox recovery profile and signed enrollment

The MVM Hostd development candidate introduces a distinct Firecracker recovery
profile under the existing signed mailbox policy asset. It requires a unique
signed enrollment matching the independently selected commitment before Hostd
consumes the signed plan's nonce. A legacy selection, an absent selection, and
a foreign enrollment do not consume the nonce for the exact recovery plan.
A recovery plan without an enrollment is refused.

The corrected locked RED reached the expected missing recovery-profile variant.
The isolated green executable passed the existing V1 admission/dispatch
regression and the new exact enrollment regression. The code and test inputs
matched their recorded hashes. The first composition attempt used a dependency
selection incompatible with the retained Hostd source. The valid test reused
the earlier local FlowMux/transfer/storage overlay and retained the exact
original SDK lock without changes.

The candidate still refuses recovery dispatch at the initial installed gate.
It grants no listener, worker, or VM. The current SpiralCore provider needs an
explicit recovery refusal when compiling against the new profile, until the
original worker's replacement capability is wired. No native replacement,
release guest, WAN or security compliance result is established here.

The patch is relative to the retained-activation MVM SDK source used by the
installed qualification. It is a development checkpoint. An exact pinned
canonical authenticated-session test and the original route proof remain
separate from signed plan admission.
