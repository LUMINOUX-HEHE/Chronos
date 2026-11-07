# CHRONOS program format v1

Programs are UTF-8 JSON documents. Unknown object fields are rejected. The loader validates format version, name, event count, labels, operation bounds, dependencies, duplicate IDs, and graph acyclicity.

## Resource limits

- Program JSON file: maximum 64 MiB, enforced during the read.
- Program name: maximum 4,096 UTF-8 bytes.
- Events per program: maximum 100,000.
- Event label: maximum 4,096 UTF-8 bytes.
- Run record file: maximum 128 MiB, enforced during the read.
- Certificate file: maximum 128 MiB, enforced during the read.
- hash_rounds operation: maximum 5,000,000 rounds per event.
- Aggregate hash_rounds budget: maximum 25,000,000 rounds per graph.

These limits reduce accidental or unbounded input sizes. They are not a complete CPU or memory sandbox.

## Top-level fields

- format_version: required integer; currently 1.
- name: required non-empty string.
- events: required array.

## Event fields

- id: unique unsigned 64-bit integer.
- label: non-empty string for reports.
- operation: tagged operation object.

## Operations

Input:

    { "type": "input", "value": 144 }

Add:

    { "type": "add", "left": 1, "right": 2 }

Subtract:

    { "type": "sub", "left": 1, "right": 2 }

Multiply:

    { "type": "mul", "left": 1, "right": 2 }

Divide:

    { "type": "div", "left": 1, "right": 2 }

Bounded deterministic mixing workload:

    { "type": "hash_rounds", "input": 1, "rounds": 250000 }

All arithmetic operations use checked signed 64-bit integer semantics. Division by zero is an explicit error; MIN / -1 is an overflow error.

hash_rounds applies a fixed, deterministic 64-bit mixing transform repeatedly to one predecessor value. Its round count is bounded by the per-event and graph-wide limits. It exists to create a repeatable compute-heavy operation for scheduler experiments. It is not a cryptographic hash, password hash, or security primitive.

## Dependencies and concurrency

Dependencies are explicit event IDs. The engine does not infer undeclared reads or side effects. Operations are pure and deterministic. Incomparability is only treated as a concurrency candidate under this explicit pure-operation contract.

## Fingerprints

The program fingerprint is SHA-256 over compact JSON of a canonicalized program object whose event array is sorted by event ID. Event declaration order therefore does not affect the fingerprint. Name, labels, operation tags, operand references, and input values do affect it.

The fingerprint is a content-integrity identifier, not a digital signature or proof of authorship.

## Run record v2

A run record contains:
- record format version 2;
- program fingerprint;
- canonical event values and topological order;
- graph layers;
- runtime metrics, including selected strategy;
- worker completion order.

Run-record loading enforces exact event-set coverage, unique canonical/completion orders, a complete non-empty layer partition, and bounded metrics. Unknown fields in the run record, execution report, and metrics structures are rejected. Replay checks the program fingerprint and deterministic execution report. Timing metrics and actual completion order are deliberately not compared.

## Incremental report

An incremental report contains:
- complete updated canonical value map;
- ordered IDs that were recomputed;
- ordered IDs whose prior values were reused;
- input overrides supplied by the caller.

Incremental recomputation assumes the base report's values were produced successfully by the same compiled graph. Structural coverage is checked; old values are not re-evaluated by the incremental operation.

## Execution certificate v1

See CERTIFICATE_FORMAT.md. A certificate includes the program fingerprint and a canonical per-event chain of direct predecessor values, operation outputs, previous hashes, entry hashes, and a final hash-chain root. Structural bounds are checked before verification.
