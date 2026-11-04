# Independent requested-allocation witness review

2026-11-04 — session_debug. Code/Rust/security **SOURCE PASS** for the five inputs below. No product edits, builds or measurement reruns. This review does not approve a total scanner budget, native allocator/RSS bound, FIPS claim or native activation.

| Input | SHA-256 |
| --- | --- |
| accounting.rs | 5a165c3489d9e00ebb87e4e854a9366a001eacb2595445229c390cedb4b5a6b1 |
| allocator.rs | aca123497037d2a744176f5756d346a03c2f53f2c8f9e78614e5c461344eda92 |
| allocation_smoke.rs | cf5e067a280f100c28b357979662476ffa0ab1c5989bb9d19fad63e42ddbc96f |
| probe.rs | cb5695db0f97ad6c730ccbc82072c26e96a80918aa5c6e76778207f145e77280 |
| PLAN.md | e5b25372f8707e83473c61efdd5a26ec5e3b2868ed2792f9d4a59b98f553c785 |

The four unsafe allocator methods forward the caller's valid pointer/layout/size unchanged exactly once to System. Successful allocations/reallocations alone update requested-byte counters; a failed realloc leaves the original allocation accounting intact. Checked atomic arithmetic latches faults without logging, allocating or unwinding in callbacks. The successful checked addition justifies the subsequent addition used for the peak. Realloc overlap includes old plus new requested sizes as a conservative API calculation, not a measured System internal overlap. The smoke exercises actual System allocation/reallocation, original and appended byte preservation, overlap accounting and return to baseline. Arithmetic tests separately exercise overflow/underflow fault latching.

Phase resets are appropriate only for the declared single measurement thread. This code is not a consistent concurrent snapshot/high-water protocol. Inputs and policies are prepared before measurement; one scanner constructs and searches on the same thread. Search observations retain some prior results; legacy output comparison happens outside the measured windows. Reporting emits only static labels, lengths, counters and timings. Exact masked output and rule-name vectors are compared to the canonical legacy constructor for each selected policy/case. No key or payload output was introduced.

Interpretation: search phase peak deltas exclude the baseline already occupied by earlier scanner caches and retained observation outputs. Reconstruct absolute peak as live_before + peak_above_baseline, then compare to the policy's original pre-constructor baseline. The final full-drop live equality is an observed output field, not an assertion in this revision; acceptance must check the recorded pair. A measured maximum over these synthetic inputs is not a universal search bound. The preregistered 325 ordered nonempty selections exhaust constructor selections only, with default/disabled effective-full-set semantics still requiring their separate tests. It does not exhaust search strings, platforms, native allocator overhead or RSS.

Root reports protected build/run exit0, 588 source inputs and binary before/after MATCH, full-five constructor peak1,423,560 requested bytes, conservative realloc-overlap1,694,711, and final live99,980 equal to the original baseline for both policies. These are Root-provided execution facts; this reviewer examined source only and did not independently reread raw execution receipts. They support accepting the witness mechanism, while the full requested-workspace profile remains subject to its source model, finite constructor extension, bounded search/caches and independent profile decision.
