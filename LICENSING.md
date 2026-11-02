# License scope

On 2026-11-02 the project owner authorized MIT licensing for **original SpiralCore
code and new documentation**, preserving imported licenses and explicitly
excluding third-party research artifacts. The root [LICENSE](LICENSE) records
that grant. It does not assert ownership of every file in this repository.

## Original contributions

The grant covers original contributions to the active Rust transfer, gateway,
gateway-core and MVM adapter/witness implementations, original experiment tools,
and original project documentation and diagrams. It grants rights only in those
original contributions; third-party portions and dependencies retain their own
terms, even when located within these directories.

## Imported and separately governed material

| Material | Treatment |
|---|---|
| `crates/uor-r4-object-store/` | Retains the UOR Foundation's [MIT notice](crates/uor-r4-object-store/LICENSE) and pinned import provenance. |
| MVM source and retained integration patch bundle | Preserve the upstream [Apache-2.0 license](crates/spiralcore-mvm-live/mvm-patch/LICENSE-MVM), notices and [patch provenance](crates/spiralcore-mvm-live/mvm-patch/README.md). The root grant does not replace those terms. |
| Cargo dependencies and bundled executables | Dependencies retain their own licenses. Public evidence downloads do not relicense their components; consult pinned manifests, locks and upstream notices. |
| `references/`, `archive/`, historical `projects/spiralcore/`, `index.html`, exported research/model source and original supplied artifacts | Excluded from the new MIT grant unless an independent license applies. Preserve original attribution and provenance; public visibility does not imply permission to reuse. |
| Quoted reference source, embedded third-party content and imported historical documents anywhere in the repository | Retain their original terms; excluded from the new grant. |

The Rust runtime's original implementation is covered as an original SpiralCore
contribution. Its [reference provenance](PROVENANCE.md#coupled-shared-runtime-rust-port-2026-10-30)
does not relicense the supplied Python wrapper, saved cards or extracted reference
source. The new README and its diagrams do not relicense linked historical research.

The manifests in [PROVENANCE.md](PROVENANCE.md) distinguish imported material from
subsequent work. Unresolved third-party rights remain unresolved; this is not a
blanket license for those materials. Contributors must identify borrowed material
and preserve applicable notices.
