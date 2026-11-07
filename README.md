# CHRONOS

**Deterministic causal computation runtime and verification research platform**

Current engineering release: **0.3.2**.

CHRONOS expresses pure computations as explicit event DAGs, validates causal dependencies, evaluates them against deterministic reference semantics, and schedules enabled events on a persistent CPU worker pool when useful. It also supports per-event execution certificates, deterministic replay, and incremental recomputation of changed input cones.

Research integrity matters: DAG execution, partial orders, event structures, dataflow, work queues, and deterministic replay all have substantial prior art. CHRONOS does not claim to have invented those concepts. Its novelty remains an open research question.

## Integrated capabilities

- Textual CHRONOS source language with named events, forward references, comments, and JSON compilation.
- Versioned JSON programs with 64 MiB file, 4 KiB program-name/label, 100,000-event, per-event work, and aggregate compute-budget limits; persisted-file readers enforce byte caps while streaming.
- Input, add, subtract, multiply, divide, and bounded deterministic mixing operations.
- DAG validation, deterministic topological ordering, graph statistics, and causal queries.
- Persistent worker pool with dynamic ready-queue scheduling.
- Adaptive execution: sequential path for small/cheap work; parallel path for wide, estimated-expensive work.
- Canonical deterministic results independent of worker completion order.
- Per-event execution certificates with predecessor values and a SHA-256 hash chain.
- Independent certificate verification by checking every operation against the program.
- Incremental recomputation of changed input events and their transitive descendants.
- Program fingerprints, run records, and deterministic replay.
- Atomic file replacement for program, run-record, certificate, and CLI-generated artifacts.
- CLI, DOT export, benchmark suite, Windows CI, and PowerShell launchers.

## Requirements

- Windows 10/11 or another Rust-supported platform.
- Stable Rust edition 2024 toolchain.
- rustfmt and clippy components for complete verification.
- No GPU required; execution is CPU-based.
- Network access is needed for first-time Cargo dependency downloads.

## Build and quality checks

From the repository root:

    cargo build --workspace --release
    cargo test --workspace
    cargo check --workspace --all-targets
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

PowerShell helper (formatting, tests, all-target checks, Clippy, example, and release build):

    .\scripts\run_checks.ps1

## Use the compiled release CLI

Build once:

    cargo build --workspace --release

Then run:

    .\scripts\chronos.ps1 help
    .\scripts\chronos.ps1 validate examples\quickstart.chronos.json
    .\scripts\chronos.ps1 inspect examples\quickstart.chronos.json

The launcher uses target/release/chronos.exe and rebuilds it when the executable is missing or older than the Rust sources/Cargo manifests.

## Typical workflow

### 1. Write and compile textual source

    .\scripts\chronos.ps1 compile examples\source-quickstart.chronos examples\source-quickstart.chronos.json

Or run the source file directly:

    .\scripts\chronos.ps1 run-source examples\source-quickstart.chronos --workers 4 --record results\source-quickstart.run.json

### 2. Run a small JSON graph (adaptive policy chooses sequential)

    .\scripts\chronos.ps1 run examples\quickstart.chronos.json --record results\quickstart.run.json

### 3. Run a compute-heavy graph (adaptive policy can choose parallel)

    .\scripts\chronos.ps1 run examples\compute-heavy.chronos.json --workers 8 --record results\compute-heavy.run.json

Use --force-parallel to bypass the adaptive policy and force worker-pool scheduling.

### 4. Replay a run

    .\scripts\chronos.ps1 replay examples\quickstart.chronos.json results\quickstart.run.json

### 5. Create and verify a per-event certificate

    .\scripts\chronos.ps1 certify examples\quickstart.chronos.json results\quickstart.run.json results\quickstart.certificate.json
    .\scripts\chronos.ps1 verify-cert examples\quickstart.chronos.json results\quickstart.certificate.json

The certificate verifier checks the program fingerprint, causal predecessor values, each operation output, each entry hash, and the final hash-chain root. It is an integrity/checkability mechanism, not a digital signature, trusted timestamp, Merkle inclusion proof, or zero-knowledge proof.

### 6. Incrementally recompute after changing an input

    .\scripts\chronos.ps1 recompute examples\quickstart.chronos.json results\quickstart.run.json 1 150 results\quickstart.incremental.json

Only the changed input and its transitive descendants are re-evaluated; unrelated event values are reused. The input run record is expected to be a trusted prior execution for this exact program. The incremental API checks report structure but does not independently re-evaluate the base values. Run replay first when the base record is untrusted.

### 7. Benchmark the selected policy

    .\scripts\chronos.ps1 benchmark examples\compute-heavy.chronos.json --workers 8 --iterations 21

The command reports sequential reference, forced-parallel, and adaptive timings. Benchmark conclusions are workload- and machine-specific.

## CLI commands

| Command | Purpose |
|---|---|
| init | Create a starter JSON program |
| compile | Compile named .chronos source into portable JSON |
| run-source | Compile and execute textual source directly |
| validate | Validate format and causal graph |
| inspect | Show fingerprint and graph statistics |
| explain | Show an event's ancestors, inputs, value, and descendants |
| export-dot | Export a Graphviz DOT graph |
| run | Execute with adaptive strategy; optionally force parallel |
| replay | Verify deterministic results against a program |
| certify | Generate a per-event execution certificate from a run record |
| verify-cert | Independently verify every certificate event against the program |
| recompute | Recompute changed input event cones from a prior report |
| benchmark | Compare sequential, forced-parallel, and adaptive execution |
| version / help | Display version or command reference |

See docs/CLI_REFERENCE.md, docs/SOURCE_LANGUAGE.md, docs/PROGRAM_FORMAT.md, docs/CERTIFICATE_FORMAT.md, and docs/ADVERSARIAL_TESTING.md.

## Repository structure

- crates/chronos-core/src/graph.rs — graph model, operations, validation, causal queries.
- crates/chronos-core/src/runtime.rs — lazy persistent worker pool and adaptive/dynamic scheduler.
- crates/chronos-core/src/incremental.rs — affected-cone recomputation.
- crates/chronos-core/src/certificate.rs — certificate generation and verification.
- crates/chronos-core/src/format.rs — program and run-record formats plus structural validation.
- crates/chronos-core/src/atomic_file.rs — atomic artifact replacement.
- crates/chronos-core/src/source.rs — textual source parser and lowering.
- crates/chronos-core/src/bin/chronos.rs — CLI.
- crates/chronos-core/tests/ — atomic-save, adversarial-input, certificate, CLI, format, incremental, graph, runtime, and invariant tests.
- crates/chronos-core/benches/engine.rs — graph-shape and compute-heavy benchmarks.
- examples/ — ready-to-run programs.
- results/ — benchmark records, run records, certificates, and generated graphs.
- docs/ — research brief, CLI reference, format reference, and status.

## Research status and limitations

The software is substantially more capable than the first prototype, but it is not a finished research theory. The current runtime is a small single-machine CPU system for finite pure integer DAG programs. It has no general-purpose language, shared side effects, distributed workers, GPU backend, persistence database, or complete resource sandbox.

The adaptive cost estimate is a heuristic; it is not a learned cost model or a speedup guarantee. The deterministic mixing operation exists to create bounded compute-heavy benchmark workloads; it is not cryptographic hashing.

A hash-chained certificate detects corruption relative to its supplied content and lets a verifier re-execute all operations. It does not authenticate who produced it. Incremental recomputation assumes a trusted base report; use replay or a verified certificate if the base must be checked.

A publishable novelty claim has not been established. Compare against mature DAG/dataflow runtimes and deterministic execution research before making one.

## License

No redistribution license has been selected. Do not assume permission to redistribute the software until a license is added.
