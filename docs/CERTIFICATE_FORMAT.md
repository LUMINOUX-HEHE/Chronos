# CHRONOS execution certificate format v1

## Purpose

An execution certificate is a portable, independently checkable trace for a successful deterministic program execution. It is designed so a verifier can check each operation from the program definition and already-verified predecessor values without trusting a separate run record.

It is not a zero-knowledge proof, succinct proof, digital signature, Merkle inclusion proof, or authenticated timestamp.

## Top-level structure

- format_version: currently 1.
- program_hash: canonical SHA-256 fingerprint of the program.
- entries: one entry for every event in canonical topological order.
- root_hash: final hash-chain value; for an empty graph, the all-zero genesis hash.

## Entry structure

Each entry contains:
- event_id: event ID at this position in canonical order.
- inputs: exact direct predecessor IDs and values, in the graph's canonical dependency order.
- output: computed signed 64-bit result.
- previous_hash: hash of the immediately preceding entry, or genesis for the first entry.
- entry_hash: SHA-256 of compact JSON encoding of event_id, inputs, output, and previous_hash.

## Verification algorithm

The verifier:
1. checks certificate version and program fingerprint;
2. validates the program graph;
3. checks entry count and canonical event order;
4. checks each predecessor list and predecessor value against values already verified;
5. evaluates the operation using those predecessor values;
6. checks the certified output;
7. recomputes and checks the entry hash and previous-hash link;
8. checks that the final entry hash equals root_hash.

A certificate with a modified input, output, event order, hash link, root hash, or program is rejected.

## Trust model and limitations

- SHA-256 detects accidental or malicious modification only relative to a root value that the verifier trusts or stores independently.
- Anyone can generate a new certificate and root for a different trace; the program fingerprint and operation checks ensure the trace matches the named program.
- There is no private signing key. Certificate origin/authorship is not authenticated.
- Verification recomputes every operation, so it provides auditability and portable validation, not faster-than-execution proof checking.
- The hash chain is linear, not a Merkle tree. It does not provide logarithmic inclusion proofs.
- Certificates are currently bounded by a 128 MiB file limit.
