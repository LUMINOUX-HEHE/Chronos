# CHRONOS Source Language 0.3

## Purpose

The CHRONOS source language is a small textual front end for the causal event-graph model. It gives each event a readable name and lets users write dependencies without manually assigning numeric IDs.

The source compiler lowers the program into the same validated Program and CompiledGraph model used by the JSON CLI. It enforces the 64 MiB source-file, 100,000-event, 4,096-byte identifier/label, and 4,096-byte program-name limits while parsing. The language does not introduce a separate execution semantics.

## Example

    program "basic arithmetic";

    a = input(144);
    b = input(12);

    sum = add(a, b);
    difference = sub(a, b);
    product = mul(sum, difference);
    quotient = div(a, b);

## Declarations

Each event declaration has this form:

    name = operation(arguments);

Event names must:
- begin with an ASCII letter or underscore;
- contain only ASCII letters, digits, and underscores after the first character;
- be unique within the program.

The parser assigns event IDs in declaration order. References are resolved after all declarations are collected, so a statement can refer to an event declared later in the file. The graph compiler then validates that the resulting dependency graph is acyclic.

## Operations

Input constant:

    a = input(-12);

Addition:

    sum = add(a, b);

Subtraction:

    difference = sub(a, b);

Multiplication:

    product = mul(a, b);

Division:

    quotient = div(a, b);

Bounded deterministic CPU workload:

    work = hash_rounds(a, 250000);

The operation set, numeric limits, checked arithmetic rules, and pure-operation contract are identical to program format v1. hash_rounds is a deterministic workload primitive, not a cryptographic hash.

## Program name

An optional program declaration supplies a name:

    program "my causal computation";

The name is a quoted JSON string, so JSON escapes are supported. If omitted, the program name defaults to untitled.

## Comments and whitespace

- Blank lines are ignored.
- A line may end with an optional semicolon.
- # begins a line comment when outside a quoted string.
- // begins a line comment when outside a quoted string.
- Spaces and tabs around declaration names, equals signs, operation names, and arguments are ignored.

## Forward references

Forward references are supported:

    result = add(left, right);
    left = input(40);
    right = input(2);

The compiler resolves names after parsing all declarations. Missing names are source errors. Cycles are rejected by the graph validator.

## Compile and run

Compile source into the portable JSON program format:

    .\scripts\chronos.ps1 compile examples\source-quickstart.chronos examples\source-quickstart.chronos.json

Run source directly:

    .\scripts\chronos.ps1 run-source examples\source-quickstart.chronos --workers 4 --record results\source-quickstart.run.json

The resulting run record can be replayed against the compiled JSON program:

    .\scripts\chronos.ps1 replay examples\source-quickstart.chronos.json results\source-quickstart.run.json

The source compiler validates the program before writing or execution. The JSON representation remains the canonical portable interchange format.

## Current language limitations

- One event declaration per line.
- No variables that change value; each name refers to one immutable event result.
- No loops, branches, functions, modules, imports, macros, user-defined operations, or external effects.
- No string, array, floating-point, rational, tensor, or algebraic-number values.
- No implicit dependencies; all reads are operation arguments.
- No general-purpose computation claim.

The source language is intentionally small so the causal graph semantics remain explicit.
