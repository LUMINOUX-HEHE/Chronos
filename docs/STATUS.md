# CHRONOS project status

## Current engineering release: 0.3.2

- [x] Rust workspace and release CLI
- [x] Textual CHRONOS source language with named events and forward references
- [x] Source-to-JSON compiler and direct source execution CLI
- [x] Versioned JSON program format
- [x] Graph validation and causal analysis
- [x] Core graph compiler enforces event-count and label-byte limits
- [x] Program name limited to 4,096 UTF-8 bytes
- [x] Checked signed-64-bit arithmetic
- [x] Bounded deterministic HashRounds workload operation
- [x] Per-event and aggregate HashRounds compute budgets (5M per event, 25M per graph)
- [x] Deterministic sequential reference evaluator
- [x] Lazy persistent CPU worker pool
- [x] Dynamic ready-queue scheduler
- [x] Adaptive sequential/parallel selection with recorded strategy
- [x] Incremental recomputation of changed inputs and descendants
- [x] Per-event hash-chained execution certificates
- [x] Independent certificate verification
- [x] Program fingerprints and run records (record format v2)
- [x] Deterministic replay verification
- [x] Strict run-record structural validation
- [x] Certificate structural validation before save/load/verification
- [x] File input byte caps enforced while reading
- [x] Program and run-record serialization stops at configured byte limits
- [x] CLI rejects unknown options, duplicate options, and extra positional arguments
- [x] Source parser enforces event and identifier limits during parsing
- [x] CLI rejects duplicated valued options
- [x] CLI: init, compile, run-source, validate, inspect, explain, export-dot, run, replay, certify, verify-cert, recompute, benchmark
- [x] Windows CI, PowerShell launcher, and full project documentation
- [x] Deterministic mutation-based robustness tests for source and JSON inputs
- [x] Atomic replacement for program, run-record, certificate, DOT, and incremental-result files

## Verified test coverage

- Arithmetic-boundary tests: 2 passed
- Certificate tests: 7 passed
- CLI end-to-end tests: 2 passed
- Format/resource tests: 10 passed
- Generated-DAG property tests: 2 passed (32 seeds each)
- Adversarial-input tests: 2 passed (8,192 source mutations and 8,192 JSON mutations)
- Atomic-save tests: 2 passed (overwrite and failed-replacement preservation)
- Direct graph resource-limit tests: 2 passed
- Graph-query tests: 2 passed
- Incremental recomputation tests: 5 passed
- Invariant tests: 4 passed
- Runtime/scheduler tests: 11 passed
- Source-language parser tests: 6 passed
- Total: 57 integration tests passed

## Verified quality gates on the target laptop

- cargo fmt --all -- --check — passed
- cargo test --workspace — passed
- cargo check --workspace --all-targets — passed
- cargo clippy --workspace --all-targets -- -D warnings — passed
- cargo build --workspace --release — passed
- .\scripts\run_checks.ps1 — passed end-to-end, including release build

## Verified release workflows

- JSON init, validate, inspect, explain, DOT export, adaptive/forced-parallel run, JSON output, and record save
- Run-record replay against the same program
- Execution certificate create, load, and independent verification
- Incremental recomputation against a saved base record
- Text source compile to JSON, direct source run, and replay against the compiled program
- Aggregate compute-budget rejection
- Unknown-flag, duplicate-option, and extra-positional rejection
- Release launcher detects stale Rust sources and rebuilds before execution

## Initial empirical result

On the target Windows 11 laptop (Intel Core i5-12450HX, 12 logical processors), the adaptive policy selected sequential execution for all 15 inexpensive graph cases. For the bounded compute-heavy independent workload, it selected parallel execution and measured about 7.30x faster median latency than sequential execution (about 7.805 ms versus 1.069 ms).

This is a single-machine synthetic workload result, not a general performance guarantee. Raw details are in results/benchmark-2026-11-07-v0.3.txt.

## Remaining engineering limitations

- Finite in-memory programs only.
- Pure signed 64-bit integer operations only.
- Single-machine CPU runtime; no GPU, distributed workers, web service, GUI, or persistence database.
- Adaptive cost model is a heuristic and requires broader tuning.
- Incremental recomputation trusts base values after structural validation; replay/certificate verification is needed for untrusted inputs.
- Execution certificates are linear hash chains, not signed or succinct proofs.
- The mixing workload is not a cryptographic primitive.
- Resource caps are defensive bounds, not a complete isolation sandbox for hostile multi-tenant workloads.

## Remaining research gates

- Repeat benchmarks across additional machines and realistic task-cost distributions.
- Add fuzzing and adversarial tests to complement fixed-seed generated-DAG tests.
- Compare against mature external DAG/dataflow systems.
- Complete formal proofs and prior-art review.
- Establish a precise, defensible research contribution before publication claims.
