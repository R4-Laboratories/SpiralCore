# Installed launch admission replay regression

The protected predecessor test returned the original canonical admission after starting a backend. Reusing that admission started the backend again. The focused test failed at that assertion.

The successor tracks the dispatch attempt on the original opaque admission. The local and client wrappers check the state before additional effects. Canonical Hostd marks the attempt immediately before calling the backend, following its final backend and validity-window check. The state stays consumed across failure and rollback.

The successor compiled on Wick and the protected Hostd binary ran 23 distinct checks across signed local boot, ordinary local boot, and backend/posture rechecks. All passed, including the same-token regression. The checks use Mock dispatch. They do not establish an installed native guest launch or qualify the Client wrapper on the successor. The composed activation run and the first native installed transfer and retirement remain separate gates.

The three-file MVM SDK delta is against the SHA-256 preimages in the source receipt. Apply it to the corresponding staged SDK. The file is an evidence artifact for the reviewed component, separate from an upstream submission. The recorded Hostd source hashes matched before and after protected execution.
