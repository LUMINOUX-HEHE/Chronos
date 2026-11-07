# CHRONOS Research Plan 0.3.0

## Research objective

Determine whether the combination of explicit causal computation, adaptive deterministic scheduling, incremental causal recomputation, and independently verifiable execution traces can produce a defensible formal, algorithmic, or systems contribution beyond established DAG/dataflow runtimes and deterministic execution research.

The current implementation is an experimental platform. Its components use known concepts; novelty has not yet been established.

## Related prior art to compare

- Kahn process networks and dataflow process networks.
- Petri nets, event structures, pomset semantics, and Mazurkiewicz traces.
- DAG schedulers, dynamic ready queues, and work-conserving worker pools.
- Deterministic parallel execution, including DORADD.
- Deterministic replay and execution provenance.
- Self-adjusting computation, incremental build systems, and incremental dataflow.
- Proof-carrying data and verifiable computation traces.

See RELATED_WORK.md for the initial source map. It is not yet an exhaustive literature review.

## Current research hypotheses

### H1 — Semantic determinism

For a valid finite DAG of deterministic pure operations, every legal topological execution computes the same event values.

Evidence required:
- formal induction over causal depth;
- generated DAG tests;
- schedule-permutation tests.

### H2 — Dynamic scheduler correctness

The ready-queue runtime never dispatches an event before all direct predecessors complete, and its successful report equals the sequential reference report.

Evidence:
- scheduler state invariant;
- property-based random DAG tests;
- worker count sweeps;
- injected arithmetic errors and panic recovery tests.

### H3 — Adaptive execution

A simple estimate of graph width and operation cost can avoid parallel scheduling overhead for cheap workloads while selecting parallel execution for expensive independent workloads.

Initial evidence on one i5-12450HX:
- the policy selected sequential execution for all 15 inexpensive DAG cases;
- the compute-heavy 16-branch HashRounds workload selected parallel execution;
- median time was about 7.73 ms sequential and 1.08 ms adaptive, approximately 7.13x faster for this one synthetic workload.

Limitations:
- one computer and one synthetic heavy workload;
- the threshold is hand selected;
- adaptive sequential overhead remains measurable;
- this does not establish general speedup.

Next:
- sweep thresholds and worker counts;
- test heterogeneous task costs, varied graph widths, and seeded random DAGs;
- compare policy selection against an oracle that picks the fastest mode after measurement;
- report false-positive and false-negative policy decisions.

### H4 — Incremental recomputation correctness

Given a valid prior report and a set of changed source inputs, recomputing exactly the union of their transitive descendant cones yields the same values as fully recompiling/re-executing the graph.

Evidence:
- proof based on topological induction;
- fixed-seed random DAG generation;
- random input update sets;
- comparison with full re-execution;
- correctness and speed as a function of affected-cone fraction.

Trust assumption:
- prior values are trusted. The current implementation checks report structure but does not re-evaluate unaffected values.

### H5 — Certificate soundness

A certificate verifier accepts only when each event's certified direct inputs and output agree with the program and already-verified predecessor values, each entry hash is valid, and the root is consistent.

Evidence:
- formal induction over certificate entries;
- tampering tests for program, order, inputs, outputs, entry hashes, and root;
- fuzzing malformed certificate JSON.

Limitations:
- hash chain is linear, not Merkle;
- no signer authenticity;
- verification re-evaluates all operations;
- collision-resistance assumptions apply to SHA-256 integrity.

### H6 — Auditability

A per-event certificate helps localize which causal event is wrong without trusting a monolithic final output.

Evidence:
- fault-injection experiments;
- verifier diagnostic precision;
- compare trace size and verification cost with a single-result checksum and full run replay.

## Benchmark design

### Workload families

- chain (depth-heavy, width one/two);
- wide antichain;
- balanced reduction tree;
- fan-out;
- skewed-depth graph;
- bounded compute-heavy independent tasks;
- heterogeneous-cost DAGs;
- fixed-seed random DAGs.

### Baselines

1. Sequential reference evaluator.
2. Static Kahn-layer scheduler.
3. CHRONOS dynamic ready-queue scheduler.
4. CHRONOS adaptive policy.
5. Mature external DAG/dataflow runtime with matched operation semantics.
6. Relevant published deterministic runtime systems where reproducible comparison is practical.

### Measurements

Separate:
- parse and validation;
- graph compilation;
- pool startup;
- warm sequential execution;
- warm parallel execution;
- adaptive execution;
- certificate generation;
- certificate verification;
- incremental recomputation.

Record:
- exact hardware and OS;
- compiler version and build profile;
- graph shape, event count, edge count, depth, width;
- worker count, thresholds, operation costs;
- warmups, samples, median, p95, dispersion;
- correctness checks for every mode.

## Proof obligations

1. Well-formedness and DAG termination.
2. Dependency safety of ready-queue dispatch.
3. Deterministic value semantics for pure deterministic operations.
4. Sequential/parallel equivalence.
5. Incremental recomputation equivalence under trusted base report.
6. Certificate verification soundness under the specified program semantics and SHA-256 collision-resistance assumption.

Each proof must state its assumptions and cover errors/overflow explicitly.

## Research gates

1. Correctness: generated tests and error cases pass.
2. Formal: key properties are proved under explicit assumptions.
3. Prior art: define precise distinctions from existing models and systems.
4. Empirical: repeatable benefits or a clearly bounded result on declared workloads.
5. Novelty: narrow the claim if no distinct contribution survives review.
6. Reproducibility: publish code, raw results, fixed seeds, and exact environment.

## Current conclusion

CHRONOS 0.3.0 is a functioning research platform with a promising adaptive result on one compute-heavy synthetic workload, incremental recomputation, and independently verifiable event certificates. It is not yet a novel theory of computation, a production-grade general runtime, or a proven overall improvement over mature systems.
