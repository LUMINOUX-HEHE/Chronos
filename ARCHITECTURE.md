# CHRONOS Architecture 0.3.2

## System overview

CHRONOS is a single-machine Rust runtime for finite, deterministic, pure computation graphs. It combines a validated event-DAG representation, a sequential reference interpreter, a lazy persistent worker pool, a dynamic ready-queue scheduler, adaptive execution selection, incremental input-cone recomputation, and independently verifiable execution certificates.

This is a concrete systems implementation built from established computer-science ideas. It is not, by itself, a new theory of computation.

## Workspace layout

The project remains in one Cargo workspace and one repository. The chronos-core crate exposes both a Rust library and the chronos command-line executable.

Source modules:
- crates/chronos-core/src/graph.rs
- crates/chronos-core/src/runtime.rs
- crates/chronos-core/src/incremental.rs
- crates/chronos-core/src/certificate.rs
- crates/chronos-core/src/format.rs
- crates/chronos-core/src/atomic_file.rs
- crates/chronos-core/src/source.rs
- crates/chronos-core/src/bin/chronos.rs

## Graph model

The graph module defines EventId, Event, Operation, graph validation, predecessor/successor indexes, deterministic Kahn layers, canonical topological ordering, graph summaries, and causal ancestor/descendant queries.

Dependencies are derived from each operation and must reference known event IDs. The compiler rejects duplicate IDs, empty or oversized labels, graphs exceeding 100,000 events, missing dependencies, cycles, per-event work-limit violations, and aggregate work-budget violations.

The compiled graph stores:
- events in a BTreeMap;
- predecessor and successor indexes;
- deterministic Kahn layers;
- a canonical topological order;
- cached estimated work units;
- cached maximum layer width.

BTreeMap and BTreeSet provide deterministic iteration and tie-breaking. The design currently favors reproducibility and auditability over minimal data-structure overhead.

### Operation semantics

Operations are deterministic and pure:
- Input(i64)
- Add, Sub, Mul, Div
- HashRounds(input, rounds)

The arithmetic operations use checked signed 64-bit integer semantics. Division by zero and overflow are explicit errors.

HashRounds applies a fixed bounded 64-bit mixing transform. It is a synthetic CPU workload for scheduler experiments, not a cryptographic primitive. Each event is capped at 5,000,000 rounds, and the aggregate graph budget is capped at 25,000,000 rounds.

## Reference evaluator

CompiledGraph::evaluate() executes operations in canonical topological order. For a valid program that does not fail, this is the semantic reference against which parallel and adaptive evaluation are checked.

A successful ExecutionReport contains:
- event values indexed by ID;
- canonical topological order;
- deterministic Kahn layers.

Canonical order does not depend on worker completion order.

## Runtime

CausalEngine stores a configured worker count and an optional worker pool.

### Lazy persistent worker pool

- CausalEngine::new() validates the configured count but does not start threads.
- The pool is created only when a parallel run is requested.
- The same pool is reused for later runs on the same engine.
- Dropping the engine sends stop messages and joins worker threads.

The pool uses crossbeam channels for tasks and completions. Worker messages own the operation and a fixed two-slot predecessor-value snapshot; current operations have at most two direct operands. Workers do not read shared mutable graph state.

### Dynamic ready-queue scheduling

The scheduler maintains remaining predecessor counts, a sorted ready set, completed values, pending tasks, and an in-flight count.

1. Source events enter the ready set.
2. While worker slots are available, the smallest ready ID is dispatched.
3. A worker evaluates the pure operation on its input snapshot.
4. A successful completion writes the value and decrements dependent counts.
5. Newly enabled dependents enter the ready set immediately.
6. On the first observed execution error, new tasks stop dispatching and already in-flight tasks are drained before returning an error.

The scheduler does not wait for a full topological layer barrier. Actual completion order can vary, but successful values and the final canonical report are deterministic.

### Adaptive policy

run_adaptive() uses cached graph metadata and an estimated work score. The current policy selects the parallel runtime when:
- maximum graph width is at least two;
- at least two workers are configured;
- estimated work units meet the current threshold of 100,000.

Otherwise it executes the sequential reference path without starting the worker pool.

This policy is a simple heuristic. It is not trained, not guaranteed to improve latency on all workloads, and is not part of the mathematical semantics. The selected strategy is included in ExecutionMetrics so policy decisions can be benchmarked.

## Incremental recomputation

CompiledGraph::recompute_inputs() accepts a prior successful ExecutionReport and a map of input event IDs to replacement i64 values.

It validates the base report's canonical order, layers, and event coverage. It rejects unknown IDs and updates to non-input events.

For changed input values, it computes the union of their transitive successor cones. It then:
- overwrites the changed source values;
- re-evaluates affected descendants in canonical topological order;
- reuses prior values of unaffected events.

The result returns a full updated ExecutionReport, plus ordered recomputed/reused IDs and applied input overrides.

Trust condition: the method trusts the values in the prior report after checking its structure. It does not re-evaluate every unaffected value. Use replay or a verified certificate first when the base report is untrusted.

## Execution certificates

A certificate binds to the canonical program fingerprint and contains one entry per event in canonical topological order.

Each entry includes:
- event ID;
- exact direct predecessor IDs and values;
- operation output;
- previous entry hash;
- entry hash.

The entry hash is SHA-256 of compact JSON for event ID, inputs, output, and previous hash. The final entry hash is the root hash.

Verification validates the program, checks the fingerprint, checks canonical order and exact predecessor values, re-evaluates every operation from already-verified predecessor values, verifies outputs and hash links, and checks the root.

This is a hash-chained trace, not a Merkle tree, digital signature, zero-knowledge proof, or succinct proof. It makes a computation portable and independently checkable but still re-executes every operation during verification. It does not authenticate the certificate author.

## Text source language

`source.rs` parses the named-event `.chronos` syntax and lowers it into the same `Program` type used by JSON. Event names map to stable IDs in declaration order; references are resolved after parsing, so forward references work. The graph compiler then validates dependency existence and acyclicity. `compile` writes the canonical JSON interchange format, while `run-source` parses and executes directly through the same adaptive runtime. Source files share the 64 MiB program input cap.

## Formats and fingerprints

Program format v1 is UTF-8 JSON with format_version, a non-empty program name (maximum 4,096 UTF-8 bytes), and an array of events. File reads are byte-capped while reading; the public graph compiler and source parser enforce event/label/name bounds before building unbounded intermediate structures.

Run-record format v2 contains:
- program fingerprint;
- canonical execution report;
- runtime metrics including selected strategy;
- observed completion order.

Program fingerprints use SHA-256 over compact JSON of a program whose event array is sorted by event ID. The hash is a content identifier, not an identity proof or signature.

Replay checks record version, program fingerprint, and canonical execution report. It intentionally ignores timings, operational metrics, and completion order.

Program, run-record, and certificate saves use a sibling temporary file created exclusively, flush its contents, and rename it into place. CLI-generated DOT and incremental-result files use the same helper. A failed write or rename attempts to clean up the temporary file and does not truncate the previous destination.

## CLI

The CLI provides:
- init, validate, inspect, explain, export-dot;
- run, replay;
- certify, verify-cert, recompute;
- benchmark, version, help.

run uses adaptive execution by default; --force-parallel bypasses the policy. benchmark compares sequential, forced-parallel, and adaptive execution using warm-up iterations and median/p95 samples.

## Input and resource limits

- Program JSON: 64 MiB.
- Program events: 100,000.
- Event label: 4,096 UTF-8 bytes.
- Run record: 128 MiB.
- Certificate: 128 MiB.
- HashRounds per event: 5,000,000.
- Aggregate HashRounds per graph: 25,000,000.

These are defensive bounds, not a complete resource sandbox. The runtime is intended for trusted local workloads.

## Known limitations

- finite in-memory programs only;
- deterministic pure i64 operations only;
- one-machine CPU execution;
- no dynamic graph mutation, external effects, transactions, database persistence, remote/distributed workers, GPU backend, or graphical UI;
- simple adaptive work estimator;
- certificates are linear SHA-256 hash chains without signatures or Merkle inclusion proofs;
- incremental recomputation trusts its base values after structural validation;
- no research novelty claim has been established.
