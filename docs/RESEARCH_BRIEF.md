# CHRONOS research brief 0.3.2

## Research question

Under what conditions can an explicit causal computation representation, adaptive deterministic scheduling, incremental causal recomputation, and independently verifiable execution traces provide a formal or practical contribution beyond established DAG/dataflow runtimes?

## Implemented system

CHRONOS 0.3.2 contains:
- a textual named-event source language with forward references, lowered into a versioned JSON program format;
- a finite pure-operation DAG language with signed 64-bit input, arithmetic, and bounded deterministic CPU mixing operations;
- validation, topological layers, cached graph metadata, and deterministic reference semantics;
- a lazy persistent CPU worker pool with a dynamic ready-queue scheduler;
- an adaptive policy that selects sequential execution for cheap workloads and parallel execution for estimated-expensive wide graphs;
- canonical results independent of worker completion order;
- causal ancestry/descendency queries and DOT export;
- versioned JSON program files and SHA-256 content fingerprints;
- run records and deterministic replay checks;
- incremental recomputation of changed input events and transitive descendants;
- per-event execution certificates with predecessor values, operation outputs, and a SHA-256 hash chain;
- independent verification of certificate entries against program semantics;
- per-event and aggregate HashRounds compute budgets;
- bounded file readers, early parser limits, validated run-record structure, and certificate structure checks;
- atomic sibling-temporary-file replacement for saved programs, run records, certificates, and CLI-generated files.

## Initial empirical observation

On one Windows 11 laptop with an Intel Core i5-12450HX, the v0.3 benchmark suite measured five simple graph families at widths 64, 256, and 1024, plus one bounded compute-heavy independent workload.

The adaptive policy selected sequential execution for all 15 cheap-integer cases. For the compute-heavy workload (16 independent HashRounds operations, 150,000 rounds each), it selected parallel execution and measured approximately 7.30x faster median latency than sequential execution (about 7.805 ms sequential versus 1.069 ms adaptive).

This is preliminary evidence for one synthetic workload on one machine. It is not a general speedup claim. The benchmark record is results/benchmark-2026-11-07-v0.3.txt.

## Why this is not yet a new theory of computation

Causal order, partial orders, event structures, dataflow computation, ready-queue scheduling, deterministic replay, provenance, and incremental computation all have substantial prior art. CHRONOS currently combines known concepts in a working restricted runtime.

The research contribution remains an open question. Potentially interesting directions include:
1. a formally distinct execution semantics that adds more than a different scheduler;
2. compact, independently checkable causal execution evidence with measurable audit benefits;
3. incremental recomputation with useful causal-slice certificates and a formal correctness result;
4. adaptive scheduling with a cost model that generalizes across graph shapes and machines;
5. a sound semantics for effectful resource conflicts, if a genuinely useful formal model can be established.

These are research hypotheses, not completed novelty claims.

## Verification evidence

The current repository has 57 integration tests, including:
- certificate tampering tests for inputs, outputs, entry hashes, root hashes, and program binding;
- incremental recomputation tests;
- generated-DAG property tests over 32 deterministic seeds for parallel/reference and certificate agreement;
- generated-DAG incremental/full-reexecution equivalence over 32 deterministic seeds;
- deterministic adversarial mutation tests: 8,192 source strings and 8,192 JSON inputs;
- atomic-save tests for replacement of existing artifacts and preservation after a failed replacement;
- CLI lifecycle and replay checks;
- resource-budget and scheduler tests.

The mutation tests are deterministic regression tests, not coverage-guided fuzzing. The suite is an engineering baseline, not a substitute for formal proofs, coverage-guided fuzzing, or external independent review.

## Required comparisons

Compare against:
- sequential topological evaluation;
- static level/layer schedulers;
- dynamic ready-queue DAG schedulers;
- mature dataflow runtimes and build systems under matched operation semantics;
- deterministic parallel runtime research, including DORADD;
- established incremental computation and execution provenance mechanisms.

A fair benchmark separates parsing, validation, compilation, pool startup, warm execution, certificate generation/verification, incremental recomputation, and end-to-end process cost. It must include chains, wide antichains, balanced trees, fan-out, skewed-depth and heterogeneous-cost graphs, and fixed-seed random DAGs.

## Remaining success criteria

- A precise formal distinction or theorem beyond a restatement of existing DAG/partial-order definitions.
- Proofs of scheduler safety, successful-result determinism, incremental recomputation equivalence, and certificate verification soundness under declared assumptions.
- Controlled multi-machine benchmarks and a fair mature-runtime comparison.
- Coverage-guided fuzzing and expanded adversarial input testing.
- Independently reproducible code, seeds, environment, and raw measurements.
- No unsupported generalization from a single laptop or synthetic workload.
