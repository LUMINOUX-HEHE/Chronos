# CHRONOS CLI reference

Run commands from the repository root with cargo, or use the compiled PowerShell launcher:

    .\scripts\chronos.ps1 <command> ...

The launcher invokes target/release/chronos.exe and builds it if it is missing. Text source syntax is documented in SOURCE_LANGUAGE.md.

## init

    chronos init <program.json> [--force]

Creates a demonstration program. Existing files are protected unless --force is provided.

## compile

    chronos compile <source.chronos> <output.json> [--force]

Parses the textual source language, validates named references and graph semantics, and writes a portable format-v1 JSON program. Existing output files are protected unless --force is provided. See SOURCE_LANGUAGE.md for the grammar.

## run-source

    chronos run-source <source.chronos> [--workers N] [--record <run.json>] [--json] [--force-parallel]

Parses and validates a source file, then executes it using the same adaptive runtime and semantics as the JSON run command. A run record can be replayed against the compiled JSON program produced by compile.

## validate

    chronos validate <program.json>

Validates format version, event count, labels, operation limits, dependencies, duplicate IDs, and graph acyclicity. Prints the canonical SHA-256 program fingerprint and graph summary.

## inspect

    chronos inspect <program.json>

Shows event/dependency counts, depth, maximum/average layer width, source/sink counts, Kahn layers, and program fingerprint.

## explain

    chronos explain <program.json> <event-id>

Shows the event's direct dependencies, transitive ancestors, transitive descendants, and reference-evaluated values in causal order.

## export-dot

    chronos export-dot <program.json> <output.dot>

Writes a Graphviz DOT graph. Rendering is optional and requires Graphviz separately:

    dot -Tpng output.dot -o output.png

## run

    chronos run <program.json> [--workers N] [--record <run.json>] [--json] [--force-parallel]

By default, adaptive execution estimates graph width and operation work:
- narrow/cheap graphs run sequentially on the calling thread;
- graphs with sufficient estimated work and width use the persistent worker pool;
- the worker pool is created lazily, so an adaptive sequential run does not start background workers.

The threshold is a heuristic and may be changed as benchmarks improve. --force-parallel bypasses the policy. Worker count is bounded to 1–256. Valued options such as --workers and --record may only be specified once.

The report contains strategy, timing, event/dependency counts, effective worker count, maximum in-flight tasks, ready-queue peak, canonical values, and completion order. Completion order is informational and can vary on parallel runs.

--json writes a JSON run record to stdout. --record saves the record. Run-record format version is 2 because execution metrics now include the selected strategy.

## replay

    chronos replay <program.json> <run.json> [--workers N]

Checks run-record version, program fingerprint, and deterministic execution report. Timing metrics and completion order are intentionally not compared.

## certify

    chronos certify <program.json> <run.json> <certificate.json>

Creates a hash-chained per-event certificate from the run's deterministic report. Creation first checks that the report equals the sequential reference semantics.

## verify-cert

    chronos verify-cert <program.json> <certificate.json>

Independently checks:
1. certificate version and program fingerprint;
2. canonical event order and exact predecessor list;
3. predecessor values already verified earlier in the trace;
4. each operation's computed output;
5. each per-entry hash link;
6. the final root hash.

The root is a hash-chain root, not a Merkle root or digital signature. It is not authenticated by a secret key.

## recompute

    chronos recompute <program.json> <base-run.json> <input-event-id> <new-value> <report.json>

Computes the changed input's transitive descendant cone and re-evaluates only those events. Unaffected event values are reused. The output JSON contains the full updated execution report, recomputed IDs, reused IDs, and applied input overrides.

The base report must originate from a successful execution of the exact program. The API checks graph structure and event coverage but trusts its prior values. For untrusted records, run replay before recomputing or verify an execution certificate.

## benchmark

    chronos benchmark <program.json> [--workers N] [--iterations N]

Compares:
- sequential reference execution;
- forced dynamic worker-pool execution;
- adaptive execution.

Runs 10 adaptive warm-ups, then samples each mode. Reports median/p95 nanoseconds and adaptive policy choice. Iterations must be between 3 and 100000. Process/parse/compile and pool startup are excluded from timed samples. Results must be interpreted per workload and machine.

## Exit behavior

Successful commands exit with code 0. Input, validation, or execution failures are printed to stderr and exit with code 2.
