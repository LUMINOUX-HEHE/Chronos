# CHRONOS Formal Specification v0.2

**Status:** executable baseline semantics; not a novelty claim.  
**Domain:** finite DAGs of deterministic pure signed-64-bit integer operations.

## 1. Research boundary

CHRONOS defines a concrete event-graph language and execution contract. DAGs, partial orders, event structures, dataflow, work queues, and deterministic replay are established prior art. The goal of this specification is precision and testability, not to rename known mechanisms as new theory.

## 2. Program model

A program is a tuple:

\[
P=(E,D,\lambda,op)
\]

where:

- \(E\) is a finite set of unique event identifiers.
- \(D\subseteq E\times E\) is the direct dependency relation. \((u,v)\in D\) means event \(v\) consumes the value produced by event \(u\).
- \(\lambda:E\rightarrow\text{Label}\) maps each event to a non-empty label.
- \(op:E\rightarrow\text{Operation}\) maps each event to a deterministic operation.

The graph \(G=(E,D)\) must be acyclic. The strict causal order \(\prec\) is the transitive closure of \(D\). For an event \(v\):

\[
Pred(v)=\{u\in E\mid (u,v)\in D\}
\]

The format-v1 operation set is:

- \(Input(n)\), where \(n\in\mathbb{Z}_{64}\);
- \(Add(u,v)\);
- \(Sub(u,v)\);
- \(Mul(u,v)\);
- \(Div(u,v)\).

Arithmetic is checked signed 64-bit arithmetic. Overflow and division by zero are explicit errors. There are no floating-point values or externally visible effects in this version.

## 3. Well-formedness

A program is well formed iff:

1. IDs are unique.
2. Labels are non-empty after trimming.
3. Every dependency references an event in \(E\).
4. \(G\) is acyclic.
5. Every operation is pure and deterministic.
6. Every dependency is declared explicitly by the operation.

A hidden read, clock read, random value, external I/O, or mutation is outside this semantic domain. A future effectful extension must declare resource conflicts and define ordering rules rather than assuming incomparability implies independence.

## 4. Causality and concurrency

The causal relation is a strict partial order:

\[
u\prec v \iff \text{there exists a directed path from }u\text{ to }v
\]

For distinct events \(u,v\), graph incomparability is:

\[
u\parallel v \iff \neg(u\prec v)\land\neg(v\prec u)
\]

Within the pure v0.2 domain, incomparable events are safe candidates for concurrent evaluation because their explicit input values are produced by completed predecessors and they have no side effects.

Incomparability is not a general guarantee of safe concurrency for stateful or effectful systems.

## 5. Reference semantics

A valid topological order is a sequence \(\pi\) containing every event exactly once such that every predecessor occurs before its dependent.

Given a partial value map \(V\), an event \(v\) may be evaluated only when all its direct predecessors are present:

\[
Enabled(v,V)\iff Pred(v)\subseteq dom(V)
\]

Its value is the checked deterministic evaluation of \(op(v)\) over those predecessor values.

The reference evaluator processes a deterministic canonical topological order. The canonical order is the flattening of deterministic Kahn layers, with event IDs sorted within each layer. Layering is also exposed as graph structure and analysis metadata; the dynamic runtime does not impose a full-layer execution barrier.

## 6. Dynamic runtime state

The scheduler maintains:

- \(C\): completed event IDs;
- \(V\): values of completed events;
- \(R\): ready event IDs whose uncompleted-predecessor count is zero;
- \(I\): in-flight event IDs;
- \(d(v)\): remaining uncompleted predecessor count for each event.

Initial state:

\[
C=\emptyset,\quad V=\emptyset,\quad I=\emptyset
\]

\[
R=\{v\in E\mid Pred(v)=\emptyset\}
\]

The worker pool has a fixed configured capacity \(W\), bounded by the implementation to 1 through 256.

### Dispatch

While no failure has been observed, \(R\neq\emptyset\), and \(|I|<W\), the scheduler removes the smallest event ID from \(R\), snapshots values for its direct predecessors, and submits that event to a worker. It then adds the event to \(I\).

### Successful completion

When worker \(v\) returns value \(x\):

1. Remove \(v\) from \(I\).
2. Add \(v\) to \(C\) and set \(V(v)=x\).
3. For every direct dependent \(w\) of \(v\), decrement \(d(w)\).
4. If \(d(w)=0\), add \(w\) to \(R\).
5. Dispatch newly enabled work immediately, subject to worker capacity.

This is a work-conserving ready-queue strategy: it does not wait for all events at one topological depth to finish before dispatching newly enabled work.

### Failure

On the first observed operation failure, the scheduler stops dispatching new events and drains tasks already in flight before returning an execution error. Because independent failures may complete in different orders, the identity of the first reported error is not promised to be deterministic when multiple in-flight operations fail simultaneously. Successful-program values and canonical reports remain deterministic.

## 7. Canonical outputs versus runtime observations

A successful run returns:

\[
R(P)=(V,\pi,L)
\]

where:
- \(V\) is the event-value map;
- \(\pi\) is the canonical topological order;
- \(L\) is the deterministic Kahn layer partition.

The runtime additionally reports observations such as:
- actual worker completion order;
- elapsed time;
- maximum in-flight work;
- maximum ready-queue size;
- graph size, depth, and width;
- worker count.

These operational observations are not part of the pure program's mathematical result. Completion order and timing may vary across runs.

## 8. Properties and proof obligations

### P1 — Dependency safety

No event is dispatched before all direct predecessors have completed.

**Argument:** the remaining-predecessor count is initialized to \(|Pred(v)|\), decremented only when a predecessor successfully completes, and the event enters \(R\) only when the count reaches zero.

### P2 — Finite-DAG termination on successful operations

For a finite DAG, at least one source exists in every non-empty residual subgraph. Each successful completion removes one event from the residual graph. Therefore, if all operations terminate successfully, the scheduler eventually completes all events.

### P3 — Deterministic values

For a well-formed program whose operations do not fail, any legal topological execution computes identical values for every event.

**Proof sketch by induction on causal depth:** source inputs are constants. Assume all strict predecessors of event \(v\) have unique values. The pure deterministic operation \(op(v)\) is a function only of those values, so its output is unique. The induction covers the finite DAG.

### P4 — Scheduler/reference equivalence

The dynamic scheduler dispatches an event only after its direct predecessor values are available. Its worker computes the same pure operation as the reference evaluator. By P3, each completed value equals the reference value. The runtime constructs its final report using the graph's canonical order and layers, so the deterministic execution report equals the reference report.

### P5 — Replay verification

A run record is tied to a canonical program fingerprint. Replay checks the format version and fingerprint, re-executes the program, and compares the deterministic execution report. Runtime timing and completion order are deliberately excluded from equality.

## 9. Fingerprinting

The program fingerprint is SHA-256 over compact JSON for a canonicalized program object whose events are sorted by event ID. The fingerprint is an integrity identifier for content. It is not a digital signature, identity proof, or guarantee of trustworthiness.

## 10. Unresolved research questions

1. What precise formal property distinguishes this runtime from known dataflow and DAG schedulers?
2. Can execution traces carry compact, independently checkable certificates?
3. Can causal slices support correct incremental recomputation?
4. How should the semantics extend to effects and shared resources?
5. Which workloads show benefits after accounting for runtime startup, scheduling cost, and memory overhead?


# CHRONOS Specification Addendum v0.3

## A. Bounded deterministic mixing operation

The v1 program format adds:

\[
HashRounds(x,r)
\]

where \(x\) is one direct predecessor value and \(r\) is an unsigned round count bounded by 5,000,000. The operation repeatedly applies a fixed 64-bit integer mixing transform using explicitly wrapping unsigned arithmetic, then returns the final 64-bit pattern interpreted as i64.

This operation is a deterministic workload primitive, not a cryptographic hash. Its purpose is to provide controlled CPU work for scheduler experiments.

## B. Adaptive execution policy

The adaptive policy computes a coarse estimated-work score:

\[
W(P)=\sum_{e\in E} cost(op(e))
\]

with small fixed weights for elementary operations and the declared round count for HashRounds. It selects the parallel runtime only when:
- maximum Kahn-layer width is at least two;
- configured worker capacity is at least two;
- estimated work is at least the current threshold.

Otherwise it selects the sequential reference evaluator. The worker pool is lazily constructed only when a parallel run is selected or explicitly forced.

The policy is an engineering heuristic. The score is not a learned model, a proof of speedup, or a stable public semantic guarantee. The selected strategy is recorded in ExecutionMetrics.

## C. Incremental recomputation

Given a successful prior ExecutionReport and a map of source-input overrides:
1. reject unknown event IDs and non-input events;
2. identify updated source values that actually changed;
3. compute the union of their transitive descendant cones;
4. preserve prior values for all unaffected events;
5. recompute changed inputs and affected descendants in canonical topological order;
6. return the full new report plus recomputed/reused event ID lists.

Correctness assumes the supplied base values were produced by this exact compiled graph. The API checks report structure and event coverage but deliberately does not recompute the entire base report before using it. A caller requiring verification of an untrusted base must replay it or validate an execution certificate first.

## D. Per-event execution certificate

Certificate format v1 stores one entry per event in canonical topological order. Each entry contains direct predecessor IDs and values, the computed output, the prior entry hash, and its own SHA-256 entry hash.

Verification re-evaluates each operation using predecessor values already verified in earlier entries. It checks graph/program fingerprint, exact causal inputs, outputs, entry hash links, and final root hash.

The root is a linear hash-chain root. It is neither a Merkle root nor an authenticated signature. Verification recomputes all operations and is intended for auditability, not succinct proof verification.


## 11. Incremental recomputation theorem (trusted base)

Let \(P=(E,D,\lambda,op)\) be a well-formed program. Let \(V_0\) be the value map of a successful execution of \(P\). Let \(\Delta\subseteq E\) be input events whose values are changed by an update map \(U\), and define the affected set:

\[
A=\Delta\cup\{v\in E\mid \exists u\in\Delta:\ u\prec v\}
\]

Let \(P_U\) be the program with only the input constants in \(\Delta\) replaced by their values from \(U\). Assume both the original and updated executions succeed without overflow or division errors.

**Theorem.** The incremental algorithm that re-evaluates exactly \(A\) in canonical topological order and reuses \(V_0(v)\) for every \(v\notin A\) produces the same value map as a full evaluation of \(P_U\).

**Proof sketch by induction on canonical topological order.**

For \(v\in\Delta\), the incremental algorithm writes the new input constant, which is exactly the semantics of \(P_U\).

For \(v\in A\setminus\Delta\), every direct predecessor \(u\) is either in \(A\) or outside \(A\). If \(u\in A\), then \(u\prec v\), so the induction hypothesis says its incrementally computed value equals its value under full evaluation of \(P_U\). If \(u\notin A\), then no changed input reaches \(u\); its prior value \(V_0(u)\) is unchanged under \(P_U\). Therefore every direct input supplied to \(op(v)\) equals the corresponding value in a full evaluation. Because \(op(v)\) is pure and deterministic, the output is equal.

For \(v\notin A\), no changed input is a causal ancestor of \(v\). No direct predecessor of \(v\) can be in \(A\), because that would make \(v\) a descendant of a changed input and therefore put \(v\) in \(A\). Thus every direct predecessor is unaffected and retains its original value. Since the base report is assumed correct for \(P\), reusing \(V_0(v)\) equals the value under \(P_U\).

Every event is covered by one of these cases, so the resulting full value map equals a fresh evaluation of \(P_U\). ∎

**Trust condition.** The proof assumes \(V_0\) is a valid value map for \(P\). The implementation checks the base report's graph structure and event coverage, but it does not independently prove the truth of all reused values. Replay or certificate verification is required when the base is untrusted.

## 12. Execution-certificate soundness theorem

Let \(P\) be a well-formed program and \(C\) a certificate accepted by the verifier. Assume the program fingerprint identifies the supplied program and the operation semantics are deterministic.

**Theorem.** Every accepted certificate output equals the value produced by the reference semantics for the corresponding event.

**Proof sketch by induction over certificate entries in canonical topological order.**

The verifier checks that entry \(i\) names exactly the next canonical event \(v_i\), and that its input list contains exactly the direct predecessors of \(v_i\). For a source event, the predecessor list is empty; the verifier evaluates its input constant and checks the certified output. For a non-source event, each predecessor occurs earlier in a topological order. By the induction hypothesis, every predecessor's certified value equals its reference value. The verifier evaluates the operation using these already-verified values and checks equality with the certified output. Determinism therefore establishes the reference value for \(v_i\).

The verifier also checks each previous-hash link, entry hash, and final root hash. These hash checks provide trace-integrity linkage; they do not replace operation evaluation. The semantic result follows from the exact input and operation checks, while program-fingerprint binding relies on the collision-resistance assumption for SHA-256. ∎

**Limitations.** Verification evaluates every operation again. The certificate is a verifiable execution trace, not a succinct proof. The hash chain does not authenticate its author because no signing key is used.

## 13. Adaptive-policy semantic equivalence

The adaptive policy chooses one of two implementations:
- sequential reference evaluation;
- dynamic worker-pool evaluation.

For successful programs under the pure deterministic operation contract, P3 and P4 establish that both implementations return the same canonical ExecutionReport. Therefore, policy selection changes only execution strategy and runtime observations, not program meaning.

The cost estimate is a heuristic used for dispatch selection. It is not part of the formal program semantics and no optimality claim is made.

## 14. Resource-budget semantics

For every HashRounds event \(v\), the round count must satisfy:

\[
0\le rounds(v)\le 5{,}000{,}000
\]

For a complete graph:

\[
\sum_{v\in E} rounds(v)\le 25{,}000{,}000
\]

The compiler rejects programs exceeding either bound before execution. These bounds restrict the synthetic work primitive but do not constitute a complete CPU, memory, or wall-clock sandbox for all possible program sizes and runtime overheads.
