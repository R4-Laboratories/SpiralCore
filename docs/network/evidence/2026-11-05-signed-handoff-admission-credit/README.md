# Signed portable handoff and admission credit lifetime

Both protected signed-image tests passed on Wick. Client selected the original portable runtime under the full pin and refused a changed signature pin. Hostd refused the missing admission pin and accepted the original one through Runtime verification. The complete source and signed-asset readbacks matched before and after execution.

A separate predecessor test exercised canonical signing under a synthetic 160 MiB workspace claim. It built successfully and failed after admission with 167,798,432 bytes still charged, including the resident slot. The successor reserves temporary and retained credit in the same tenant pool before invoking admission. Scratch releases after the original token checks, and retained credit stays in the original slot. All 28 selected installed-parent checks passed, including the new lifetime check and the saturation case where the first debit fits and the second refuses before signer creation.

The reservation fixture exercises real signing with a synthetic workspace quote. The signed-image tests exercise the portable handoff through Runtime verification. The first installed native transfer and original retirement still require the combined native fixture.
