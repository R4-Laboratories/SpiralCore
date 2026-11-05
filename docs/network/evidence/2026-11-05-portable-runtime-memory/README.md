# Signed portable Runtime startup memory

Each row is three fresh processes performing one verification of the same signed Firecracker bundle. Requested bytes are incremental above the allocator baseline. Process RSS is a separate observation.

| Runtime variant | Requested peak | Retained requested bytes | Process RSS |
|---|---:|---:|---:|
| Original workload verity baseline | 210,939,232 B | 104,743,792 B | 218,164–218,724 KiB |
| Borrowed archive and role ownership | 164,598,624 B | 104,743,792 B | 172,196–173,740 KiB |
| Reuse of parsed entry buffers | 168,562,390 B | 124,556,340 B | 178,636–180,124 KiB |
| Verified buffer compaction and chunked selected-file checks | 124,739,244 B | 104,743,792 B | 146,328–146,968 KiB |

The final conservative requested realloc-overlap term was 134,983,895 bytes. The selected-file comparison uses fixed 64 KiB stack scratch. The positive signed verity case, four existing Runtime cases, and selected-overlay mutation and boundary controls passed. The Core probe refused changes to the archive hash, publisher key pin and detached signature pin. The intermediate capacity regression is retained.

The first-guest test uses a separate managed startup reservation under the existing tenant cap. These measurements cover signed portable verification in isolation. The combined native process, guest transfer, original retirement, and production memory envelope remain separate.
