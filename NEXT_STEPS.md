# CHRONOS remaining engineering and research work

## Implemented in the 0.3.2 line

- Textual named-event source language with forward references, source-to-JSON compilation, direct source execution, and CLI tests.
- Adaptive sequential/parallel scheduling with lazy worker-pool startup.
- Bounded deterministic compute-heavy operation with per-event and aggregate budgets.
- Incremental affected-cone recomputation for input overrides.
- Per-event hash-chained execution certificates and independent verification.
- Strict structural validation of saved/loaded run records and certificates.
- Bounded JSON serialization and bounded file reads.
- CLI option/positional validation and a source-freshness-aware release launcher, verified to rebuild after a Rust source timestamp change.
- Fixed-seed generated-DAG tests for execution/certificate agreement and incremental/full-recompute equivalence.
- Deterministic adversarial mutation tests: 8,192 source inputs and 8,192 JSON inputs.
- Atomic file output uses a sibling temporary file, flushes it, and renames it into place; overwrite and failure-preservation tests pass.
- 57 passing integration tests.

## Next engineering priorities

1. Add coverage-guided fuzzing for malformed JSON, oversized graphs, certificate entries, root hashes, and dependency graphs.
2. Add compact array-indexed graph storage and compare it fairly to the ordered-map reference.
3. Sweep adaptive thresholds and worker counts over heterogeneous task-cost distributions.
4. Add certificate streaming generation/verification for very large traces.
5. Consider signed run records only if certificate-origin authentication becomes a real requirement.
6. Add explicit cancellation/time limits and OS-level resource isolation before exposing execution to untrusted multi-tenant workloads.

## Next research priorities

1. Complete prior-art comparisons with DORADD, established DAG/dataflow systems, deterministic replay, provenance, and incremental computation.
2. Formalize and prove incremental recomputation correctness under a trusted-base assumption.
3. Formalize certificate soundness and clarify claims that depend on SHA-256 collision resistance.
4. Measure certificate overhead and incremental speedup over full recomputation.
5. Compare adaptive scheduling against fixed policies and mature external runtimes.
6. Identify a precise contribution beyond combining known techniques.

## Current limitations

- Finite in-memory programs and pure i64 operations only.
- One machine, CPU-based runtime.
- No GPU, distributed execution, external effects, transaction model, or general-purpose programming language.
- Adaptive cost estimates are hand-designed heuristics.
- Certificates are linear hash chains without digital signatures or Merkle inclusion proofs.
- Incremental recomputation requires a trusted base report.
- Input-size and work budgets are safeguards, not a full sandbox.
- Novelty and publishability remain unproven.
