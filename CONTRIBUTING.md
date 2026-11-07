# Contributing to CHRONOS

## Development principles

1. Preserve the single repository layout; do not create version-specific duplicate projects.
2. Update the formal specification before changing execution semantics.
3. Keep core operations pure unless a separately reviewed effect model is introduced.
4. Reject undeclared dependencies; never infer hidden dependencies.
5. Preserve a deterministic reference evaluator for correctness comparison.
6. Do not make performance claims without reproducible release-mode measurements.
7. Retain negative results in results/ rather than overwriting them.

## Local checks

    cargo fmt --all -- --check
    cargo test --workspace
    cargo check --workspace --all-targets
    cargo clippy --workspace --all-targets -- -D warnings

## Adding an operation

- Add the operation variant to Operation.
- Declare all event dependencies.
- Implement checked semantics with explicit error cases.
- Update docs/PROGRAM_FORMAT.md and SPECIFICATION.md.
- Add unit, runtime, serialization, and CLI tests where applicable.
- Add benchmark workload coverage if performance is a relevant claim.

## Changing scheduling

- Keep the sequential evaluator as the reference semantics.
- Show that all declared inputs are available before dispatch.
- Ensure in-flight task results are drained safely on failure.
- Test worker counts 1, 2, and multiple available cores.
- Verify that canonical values/order do not depend on completion order.
- Document which error details or metrics may vary between executions.

## Research claims

A new name for an existing mechanism is not novelty. Every novelty claim must cite prior work and state the precise distinction, assumptions, and evidence. Benchmarks must specify machine, compiler, build profile, workload generator, sample count, warmups, and variance/percentiles.
