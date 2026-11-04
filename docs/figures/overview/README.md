# Engineering overview illustrations

The four README pictures were regenerated on2026-11-04 with the built-in
`image_gen.imagegen` tool, following the user's request for professional
technical illustrations. The exact final prompt set, generated-output filenames
and image SHA256 hashes are in [prompts-2026-11-04.json](prompts-2026-11-04.json).
No API key, external stock art or separate CLI generation was used. The previous
cartoon-style versions remain in Git history. Current PNGs are1672×941 pixels.

These are conceptual software cutaways rendered as ordinary computer systems.
They do not specify custom hardware, portray live program output, encode real
benchmark data, or certify the architecture. The README's text, canonical source
contracts and linked measurements are authoritative. Colored slabs represent
software state; their geometry is illustrative.

| Picture | Engineering meaning |
| --- | --- |
| `object-transfer.png` | Verified receiver state, selected recipe/residual, and exact reconstruction. The three bottom textures suggest ternary structure, discrete CA state and residuals; they are not an actual recorded trajectory, physical oscillator or missing-information proof. |
| `normal-network.png` | Linux TUN attaches atL3; an authenticated QUIC carrier transports inner IP packets over ordinary UDP/IP/NICs between cooperating gateways. Opaque TLS stays opaque. |
| `mvm-mailbox.png` | The highlighted path is the mailbox-object path: encoding before canonical mailbox encryption, ciphertext over vsock and host relay, verification at the authorized recipient. Adjacent FlowMux and owner-access queues have distinct authorization/lifecycle. The picture does not claim all vsock services carry mailbox ciphertext, that the entire host is cryptographically blind, or that the pending integration is release-qualified. |
| `osi-model.png` | OSI5–7 indicate application/session/representation responsibilities as a reference mapping. TUN tapsL3. The normal NIC/link/IP stack remains intact. vsock is a local guest–host socket, not an assigned OSI layer. |

## Inspection and provenance

Root inspected all four tool outputs for text, layer numbering, encapsulation
order, endpoint/relay separation and scope. The seven OSI labels are complete
and ordered7→1. The packet cutaway places inner IP inside QUIC inside UDP/IP.
The MVM plate labels integration in progress and highlights only the mailbox
lane. All project assets are stored here; README rendering needs no private
filesystem, image-generation service or external artifact host. No raster was
resized, retouched or synthesized through a different tool after generation.

| File | SHA256 |
| --- | --- |
| `object-transfer.png` | `81b7cae5ec17709cbf39e14c33d6890da419906c3d5d65545414f66925e6bc4c` |
| `osi-model.png` | `e49c15b9e78b054da833fb32334eb4ae2ade7fb29fe575535043a82dd7fac962` |
| `normal-network.png` | `e42a769f02c59964c723d85eb65e7f6f669bb381eda93cdae9e0bec23742a957` |
| `mvm-mailbox.png` | `33d09551787c8f212a1cb26c6dc1abea0e4848cb96e9cdaf9e3c58fde36cbd87` |
