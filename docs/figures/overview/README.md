# Overview illustrations

Four project illustrations generated with the built-in imagegen tool on 2026-11-02, then visually reviewed and copied into this repository. They illustrate software concepts, not a hardware product, literal state dump, measured compression ratio or certification. The README captions and linked contracts are authoritative for implementation details. Original SpiralCore documentation assets are included in the project's MIT scope; imported artwork is not used.

## OSI reference illustration

`osi-model.png` was generated with the same built-in tool and reviewed as a functional OSI mapping, not as a claim of separate Layer 5/6 wire protocols. It distinguishes reliable object streams, QUIC packet datagrams, inner/outer IP and the local MVM virtio-vsock path. The README includes the complete mapping in accessible text.

### Generation prompt

Use case: scientific-educational. Asset type: technically accurate professional networking-engineer illustration for the SpiralCore README, landscape 1792x1024 or 1536x1024. Create a polished premium technical reference plate, combining a precise seven-layer OSI stack with subtle dimensional materials and crisp editorial typography. Match the earlier visual family: warm off-white background, brushed aluminum edges, deep navy text, restrained turquoise for SpiralCore integration points, amber for encoded traffic. More technical and typographic than a product cutaway, with excellent hierarchy, readable labels, no clutter or sci-fi decoration. Title exactly "Where SpiralCore fits". Small subtitle exactly "OSI reference and implemented protocol paths".

Layout: left third is a seven-layer stacked model, large numbered bands in correct top-down order, exact text:
"7  Application"
"6  Presentation"
"5  Session"
"4  Transport"
"3  Network"
"2  Data link"
"1  Physical".
Highlight Application and Network in turquoise; keep Presentation and Session as pale neutral bands with a bracket adjacent to layers 5–7 labeled exactly "Application-level functions". Next to that bracket show three short entries exactly "Objects / mailboxes", "Codecs / reconstruction", "Transfer state / ACKs". Do NOT portray them as separate standardized OSI protocols. At layer 4 annotate "QUIC over UDP". At layer 3 annotate "Linux TUN: IP packets". At layers 2 and 1 a restrained shared bracket labeled "Ordinary NIC and link".

Right two thirds: TWO clearly separated precise protocol-path panels with gentle isometric slab depth, compact engineering typography.
Upper panel title "Implemented network paths". It has two clean columns with top-to-bottom stack slabs:
Column A heading "Object service": slab "SpiralCore object protocol"; slab "Reliable QUIC streams"; slab "UDP / IP"; slab "NIC / link".
Column B heading "Packet tunnel": slab "Inner TCP / UDP / IP"; slab "TUN + raw/zstd framing"; slab "QUIC datagrams"; slab "Outer UDP / IP"; slab "NIC / link".
Thin vertical directional connectors are acceptable here, but retain premium dimensional engineering illustration, not generic flowchart. Show a fine boundary around inner packet stack to distinguish inner versus outer encapsulation. Do not show warm-cache or CA codec in the packet column.

Lower inset title "MVM integration", with two small guest silhouettes separated by a host-relay silhouette. Label within guests "Mailbox + object codec"; label connections "virtio-vsock"; label relay "Keyless host relay". A concise inset footer exactly "Guest–host transport; separate from the IP stack". Indicate sealed envelope glyphs in transit and guest boundary silhouettes. Do NOT route this inset through the above QUIC stack; current native witness is local vsock.

Bottom note in small but readable text exactly "Functional OSI mapping; the implementation follows the protocol paths shown."
No benchmark numbers, hardware requirements, quantum imagery, blockchain, fake certification, lock badges on unencrypted layers, or claims of universal compression. Be exact about every label, correct seven-layer ordering and distinction between inner IP packets and outer UDP/IP transport. Prioritize engineering accuracy and professional legibility over decorative complexity.


- `object-transfer.png`: shared-state object transfer, with a targeted correction to use three children in the ternary symbol.
- `normal-network.png`: paired TUN gateways on ordinary computers and NICs; raw/zstd packet selection.
- `mvm-mailbox.png`: local two-guest host, encrypted envelopes and a keyless relay.

## Generation prompts

### object-transfer

Use case: scientific-educational. Asset type: professional open-source networking project README editorial illustration, landscape 1536x1024. Create a sophisticated high-quality three-dimensional cutaway illustration, NOT a flowchart, no boxes connected by diagram arrows, no stock cyber imagery. Precision industrial-design visualization, white porcelain and brushed aluminum machines, restrained dark navy, turquoise verified data, warm amber changed data, warm off-white studio background, subtle shadows, crisp edges, generous whitespace and large legible features. Scene: two cooperating endpoint workstations on opposite sides of a network link. Both expose a beautifully organized local cache of matching turquoise geometric data tiles. At left, a large complete cubic mosaic object has a few amber changed tiles. Over the central thin network cable, only a tiny amber set of change tiles and one small recipe card travel; do not show the full cube travelling. At right, the same full cube is reconstructed with exactly the same turquoise and amber pattern and a discreet verification check emblem. Beneath each endpoint expose matching conceptual shared-state instruments: a tiny ternary branching structure, phase rings, and a striped cellular-automaton grid, visibly matching at both ends. These are software state visual metaphors, not quantum computing or physical synchronized clocks. Make the central link and endpoints visually coherent physical objects, high-end technical magazine quality. Text: only three short labels, exactly "Shared state", "Recipe + changes", "Exact reconstruction", clean restrained typography. Do not include benchmark numbers, promises of universal compression, blockchain, infinity signs or a hardware product brand. Two endpoints must be visibly equal peers. Communicate that the endpoint already has verified reusable information and only missing information crosses the link.

### normal-network

Use case: scientific-educational. Asset type: professional open-source networking README editorial illustration, landscape 1536x1024. High-quality three-dimensional cutaway product illustration, NOT a flowchart or boxes and arrows. Consistent precision industrial-design style: white porcelain and brushed aluminum computers, dark navy internals, turquoise data, amber accents, warm off-white studio backdrop, soft shadows, crisp detailed but uncluttered physical assembly. Show two ordinary computers connected by an Ethernet-style network cable. Each computer is an exploded cutaway revealing three nested software layers above its physical NIC: ordinary application window with small TCP / UDP label, translucent virtual network membrane labeled TUN, and compact SpiralCore packet-gateway assembly beside the NIC. On sending side, a stack of turquoise packet tiles is packed into fewer smaller transport parcels; on receiving side, matching original packet tiles are unpacked into the remote application. Central physical link carries sealed parcels representing authenticated QUIC; the NIC and cable still look like ordinary networking. Retain exact packet shapes/patterns on both endpoints. Text: only concise labels "TCP / UDP", "TUN", "Packet gateway", "Authenticated QUIC", "NIC"; no paragraph text. NO shared-cache cubes, cellular automata, speculative stream compression, universal router or arbitrary Internet destination symbols: this track is an independent raw/zstd packet tunnel between cooperating gateways. Make the image understandable through physical cutaway construction, not through a flat schematic. Cinematic studio lighting without neon or dramatic sci-fi. Balanced composition, legible on GitHub, professional technical publication quality.

### mvm-mailbox

Use case: scientific-educational. Asset type: professional open-source networking README editorial illustration, landscape 1536x1024. Polished high-quality three-dimensional architectural cutaway scene, NOT flowchart rectangles and arrows. Consistent visual language: precision white porcelain and brushed aluminum host chassis, dark navy software interior, turquoise known data, amber changes, warm off-white studio background and soft shadows. One physical host chassis contains two clearly separated translucent miniature virtual-machine chambers labeled "Sender MVM" and "Receiver MVM". Inside each guest chamber show a local cache of turquoise data tiles and a small encoding/reconstruction mechanism. Source guest has full data mosaic with a few amber tiles, receiving guest ends with identical mosaic and discreet verification check. Between chambers, outside both guest boundaries but on the host, is a modest keyless relay device labeled "Host relay". A narrow channel from each guest's mailbox port to the relay is labeled "vsock". Tiny sealed envelopes with lock emblems pass through host relay; encryption/decryption is visibly inside the guest chambers, never inside the relay. No keys anywhere near host relay; no exposed plaintext object traverses host. Include a quiet orchestrator control panel on host managing both guests, not processing plaintext. Text only "Sender MVM", "Receiver MVM", "vsock", "Host relay", "Encrypted mailbox". No invented performance numbers, WAN geography, certification badges, production-ready stamp, router claims or quantum imagery. This depicts the current local two-MVM experimental witness; show distinct guest trust boundaries, small host relay, reused state and exact reconstruction in an elegant coherent physical scene rather than a block diagram. High-end technical magazine quality, generous whitespace, large recognizable objects.

### Object illustration correction

Edit this README illustration, preserving the entire composition, two workstation bodies, matching data cubes, all text, colors, lighting and network recipe/change stream. Make only the small bottom shared-state instruments mathematically consistent: on BOTH workstations replace the branching symbol with exactly ONE turquoise root connected to exactly THREE child spheres on a single row, with the middle child amber. This is a ternary split, not a binary tree; no additional nodes or branches. The two branch symbols must be identical. Also make the two circular phase-ring patterns and two tiled cellular-automaton panels exactly match each other conceptually. Keep the polished industrial editorial illustration quality. Do not change any other part of the image or add text.

