# Changelog

## 0.3.2 - 2026-11-07

### Safer artifact saving
- Added a shared atomic-write helper that creates an exclusive sibling temporary file, flushes it, and renames it into place.
- Program, run-record, certificate, DOT, and incremental-result saves now use the helper.
- Added tests confirming existing artifacts can be replaced and failed replacement leaves the previous destination intact.

### Adversarial input testing
- Added deterministic mutation tests for the textual source parser and JSON program decoder.
- Exercised 8,192 mutated source inputs and 8,192 mutated JSON inputs without parser/decoder panics.
- Parsed mutated JSON values are passed through normal program validation when deserialization succeeds.
- These tests are reproducible regression tests, not coverage-guided fuzzing; coverage-guided fuzzing remains outstanding.

### Verification
- 57 integration tests pass.
- The full formatting, test, all-target check, Clippy, example, and release-build suite is re-run for this release.


## 0.3.1 — 2026-11-07

### Hardening
- Source parser enforces event-count and identifier-size limits before allocating the complete name/event tables.
- Source, program, run-record, and certificate readers enforce file byte caps during the read rather than relying only on a prior metadata check.
- Program names are limited to 4,096 UTF-8 bytes.
- Run records validate value coverage, event order, completion permutation, layer partition, and metric bounds before save/load/replay.
- Run-record execution and metric structures reject unknown JSON fields.
- Program and run-record JSON serialization is bounded during writing, avoiding oversized output-buffer allocation.
- Execution certificates validate event count, hash encoding, and predecessor-list size before save/load/verification.
- CLI rejects unknown flags, extra positional arguments, and repeated valued/boolean options.
- The PowerShell launcher rebuilds the release executable when Rust source or Cargo manifests are newer.
- The one-command quality script now includes the optimized release build.
- Added regression coverage for the hardening paths, including arithmetic overflow, divide-by-zero, and exact/over-limit aggregate work budgets.

### Verification
- 53 integration tests pass.
- Formatting, all-target compilation, Clippy with warnings denied, and optimized release build pass on the target Windows laptop.

## 0.3.0 — 2026-11-07

### Added
- Textual CHRONOS source language with named events, comments, forward references, source compilation, and direct source execution.
- Bounded HashRounds operation for deterministic compute-heavy scheduling experiments.
- Per-event and aggregate HashRounds budgets.
- Event-count and label-byte limits enforced by the public graph compiler as well as the JSON loader.
- Lazy creation of the persistent worker pool.
- Adaptive sequential/parallel policy with selected strategy recorded in run metrics.
- Incremental recomputation of changed inputs and their transitive descendants.
- Per-event execution certificates with predecessor inputs, outputs, and a SHA-256 hash chain.
- Independent certificate verification against the source program.
- CLI commands for certify, verify-cert, and recompute.
- Fixed-seed generated-DAG tests for reference/parallel/certificate agreement and incremental/full-recompute equivalence.
- Expanded benchmarks to compare sequential, forced-parallel, and adaptive execution.

### Compatibility
- Run-record format bumped to version 2 because execution metrics now include strategy.
- Program format remains version 1; HashRounds is an additional operation tag.

### Research integrity
- The adaptive policy is a heuristic, not a guarantee of speedup.
- HashRounds is a deterministic CPU workload, not a cryptographic primitive.
- Certificates are hash-chained traces, not signatures, Merkle proofs, or succinct proofs.
- Incremental recomputation trusts the supplied base values after structural validation.
- No new-theory or publication novelty claim is made.
