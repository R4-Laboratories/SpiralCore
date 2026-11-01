# Exact acknowledged repeats

Starting source: `a7a806e24c58999aa47eac1383fc718d8b1602bf`, branch
`codex/spiralcore-mvm-overhead`. This change owns only the transfer session/client
and focused tests. Existing before-host/guest executable identities are recorded
separately in `before-binaries.sha256`.

`WarmSession::send` currently calls `PreparedSelector::prepare_acknowledged` for
every object. The acknowledged base avoids hashing the base again, but even an
exact repeat still traverses delta selection, scores eligible codecs, constructs
a recipe, and calls `object_id(target)`. An existing cached exact-object ID is
therefore not sufficient to avoid repeat preparation. The Oct 31 actual-MVM pilot
records repeat losses against Whole, which already deduplicates identical objects.

The new branch first checks scope and resource admission as before, then compares
the complete input slice and length with the session-owned immutable acknowledged
base. Equality permits reuse of that base's verified ID. A private client helper
accepts the acknowledged-base type, not an arbitrary caller-supplied hash.

An exact acknowledged repeat sends the existing Raw Begin alone, even when the
generic `warm_probe` hint is false. Only a current authenticated Published reply
can complete it. If the receiver has evicted the object, its Ready reply continues
the same Raw offer with the known exact bytes and Finish. No changed offer,
automatic retry, new wire format, or bypass of receiver authorization is added.
Transport/identity/sequence failures continue to invalidate receiver readiness;
scope and epoch changes continue to discard scoped state before any exchange.

The returned selection receipt names Raw as the only evaluated path and reports
its full miss cost. It does not claim a minimum over codecs that were skipped.
Nonrepeat selection, sparse encoding and the existing 600-byte localized-change
fixture remain unchanged.

Tests are written before implementation. The focused RED requires Begin-only Raw
probing, zero data on a hit, same-offer Raw eviction recovery, exact stopped-store
bytes and current acknowledgement validation. An ignored native sender diagnostic
uses one warmup and three measured repeats for 1 MiB and 16 MiB. Its test-generated
acknowledgements isolate sender preparation; it is not authentication, WAN, MVM or
energy evidence. Target ownership copies occur outside both before/after timing.
The separate actual-MVM measurement owner controls timing windows; no build or CPU
profile may overlap those runs. All RED, GREEN and diagnostic output is retained.
