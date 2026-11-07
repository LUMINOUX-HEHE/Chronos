# CHRONOS adversarial input testing

## Purpose

The release includes deterministic mutation-based regression tests for two untrusted-input boundaries: the textual source parser and the JSON program decoder.

## Method

The test harness starts from small valid and intentionally malformed seed inputs. A fixed xorshift generator applies bounded insertions, deletions, and byte replacements drawn from an ASCII syntax-oriented alphabet. The fixed seeds and edit procedure make failures reproducible.

Each test executes 8,192 candidates:
- Source parser: 4 seed programs, 2,048 mutations per seed. UTF-8 candidates are passed to parse_source; any successfully parsed program is also sent through semantic graph validation.
- JSON decoder: 8,192 mutations of a serialized valid Program. Any successfully deserialized program is sent through semantic graph validation.

The tests fail on an unexpected panic. Ordinary parse, decode, and validation errors are expected outcomes for malformed candidates.

## Results

On the target Windows laptop, both adversarial regression tests passed. The new test file contributes two tests to the workspace integration-test total, bringing it to 57.

## Scope and limitations

This is deterministic mutation testing, not coverage-guided fuzzing. It does not prove parser safety, memory safety, denial-of-service resistance, or correctness for all inputs. The mutation alphabet is ASCII-focused; the harness is not a comprehensive Unicode or arbitrary-byte fuzzer. No claim of exhaustive security testing is made.

Next step: run coverage-guided fuzz targets against parser, program/run-record/certificate deserialization, graph compilation, and certificate verification, with time and memory limits.
