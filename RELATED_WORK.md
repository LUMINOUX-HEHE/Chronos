# Related Work and Novelty Boundary

This is a starter map, not an exhaustive survey. CHRONOS v0.2 deliberately avoids claiming that causal graphs, DAG execution, partial orders, event structures, ready queues, deterministic replay, or work-conserving scheduling are new.

## Dataflow and process networks

1. **Lee and Parks — Dataflow Process Networks (Proceedings of the IEEE, 1995).** Describes dataflow process networks and their formal properties, relating them to Kahn process networks and dataflow programming models. DOI: https://doi.org/10.1109/5.381846
   - Public paper page: https://ptolemy.berkeley.edu/publications/papers/95/processNets/
2. **Kahn Process Networks.** A foundational concurrent model in which processes communicate through FIFO channels. A causal dependency graph of pure operations is not automatically a new model compared with these families.

## Partial orders and event structures

3. **Mazurkiewicz traces — Theory of traces (1988).** Develops a mathematical account of concurrent behavior using partial commutativity and partial orders. DOI: https://doi.org/10.1016/0304-3975(88)90051-5
4. **Winskel and Nielsen — Models for Concurrency.** Surveys transition systems, Petri nets, asynchronous transition systems, event structures, pomsets, and trace models. DOI: https://doi.org/10.7146/dpb.v22i463.6936
5. **Godefroid and Wolper — A Partial Approach to Model Checking (1994).** Uses partial-order/trace ideas to reduce redundant exploration of concurrent executions. DOI: https://doi.org/10.1006/inco.1994.1035

## Deterministic parallel runtimes

6. **Liu, Unal, Parkinson, and Kogias — DORADD: Deterministic Parallel Execution in the Era of Microsecond-Scale Computing (PPoPP 2025), pp. 282–296.** Proposes a deterministic parallel runtime with dynamic dependency capture, work-conserving worker pools, and core pipelining for datacenter services. DOI: https://doi.org/10.1145/3710848.3710872
   - Microsoft Research publication page: https://www.microsoft.com/en-us/research/publication/doradd-deterministic-parallel-execution-in-the-era-of-microsecond-scale-computing/
   - Conference record: https://ppopp25.sigplan.org/details/PPoPP-2025-Main-Conference-1/23/DORADD-Deterministic-Parallel-Execution-in-the-Era-of-Microsecond-Scale-Computing
   - Reference implementation: https://github.com/doradd-rt/doradd

This is highly relevant prior art. CHRONOS must not claim to have invented dynamic dependency scheduling, deterministic parallel execution, worker pools, or work-conserving dispatch. DORADD targets deterministic request execution for database/datacenter workloads; CHRONOS currently targets finite pure integer DAG programs on one machine. That difference in application scope is not, by itself, sufficient theoretical novelty.

## Implications for CHRONOS

- A dependency DAG is not, by itself, a new model of computation.
- Kahn topological sorting, layer scheduling, and dynamic ready queues are known techniques.
- Graph incomparability only justifies concurrent execution under a suitable purity/effect and dependency-completeness contract.
- Deterministic values do not imply deterministic completion order or deterministic first-error selection.
- A program fingerprint is an integrity identifier, not a digital signature.
- A replay check is a useful verification feature, but deterministic replay has substantial neighboring work.

## Required next review

Review primary literature and implementations for:
- Kahn process networks and dataflow languages/runtimes;
- event structures, pomset semantics, and Mazurkiewicz traces;
- deterministic parallel execution, including DORADD and earlier deterministic schedulers;
- partial-order reduction and deterministic replay;
- incremental build systems and self-adjusting computation;
- provenance, proof-carrying data, and execution certificates;
- distributed causal histories and logical clocks.

## Novelty statement policy

Until a concrete distinction is established, describe CHRONOS as an experimental causal-DAG language/runtime and verification platform. Do not call v0.2 a new theory of computation. The contribution may ultimately be a formal semantics, trace-verification mechanism, algorithm, systems implementation, or empirical study; those are distinct research outcomes.
