use chronos_core::{CausalEngine, CompiledGraph, Event, ExecutionError, RuntimeError};

fn graph() -> CompiledGraph {
    CompiledGraph::compile([
        Event::input(1, "left", 90),
        Event::input(2, "right", 10),
        Event::sub(3, "difference", 1, 2),
        Event::input(4, "scale", 2),
        Event::mul(5, "scaled", 3, 4),
        Event::input(6, "independent", 1234),
        Event::div(7, "divide", 6, 2),
    ])
    .unwrap()
}

#[test]
fn dynamic_scheduler_matches_reference() {
    let graph = graph();
    let expected = graph.evaluate().unwrap();
    for workers in [1, 2, 4, 8] {
        let mut engine = CausalEngine::new(workers).unwrap();
        let output = engine.run(&graph).unwrap();
        assert_eq!(output.execution, expected);
        assert_eq!(output.metrics.event_count, 7);
        assert_eq!(output.metrics.dependency_count, 6);
        assert!(output.metrics.maximum_in_flight <= workers);
        assert!(output.metrics.maximum_in_flight >= 1);
        assert_eq!(output.completion_order.len(), 7);
    }
}

#[test]
fn persistent_engine_can_execute_multiple_programs() {
    let mut engine = CausalEngine::new(3).unwrap();
    let first = graph();
    let second = CompiledGraph::compile([
        Event::input(1, "a", 7),
        Event::input(2, "b", 8),
        Event::mul(3, "product", 1, 2),
    ])
    .unwrap();

    assert_eq!(engine.run(&first).unwrap().execution.values[&5], 160);
    assert_eq!(engine.run(&second).unwrap().execution.values[&3], 56);
}

#[test]
fn worker_pool_recovers_after_execution_error() {
    let mut engine = CausalEngine::new(2).unwrap();
    let invalid = CompiledGraph::compile([
        Event::input(1, "numerator", 5),
        Event::input(2, "zero", 0),
        Event::div(3, "division by zero", 1, 2),
        Event::input(4, "independent in-flight", 42),
    ])
    .unwrap();

    assert!(matches!(
        engine.run(&invalid),
        Err(RuntimeError::Execution(ExecutionError::DivisionByZero {
            event_id: 3
        }))
    ));

    let valid = CompiledGraph::compile([Event::input(10, "valid", 99)]).unwrap();
    assert_eq!(engine.run(&valid).unwrap().execution.values[&10], 99);
}

#[test]
fn rejects_invalid_worker_counts() {
    assert!(matches!(
        CausalEngine::new(0),
        Err(RuntimeError::InvalidWorkerCount(0))
    ));
    assert!(matches!(
        CausalEngine::new(257),
        Err(RuntimeError::InvalidWorkerCount(257))
    ));
}

#[test]
fn checked_division_reports_overflow() {
    let graph = CompiledGraph::compile([
        Event::input(1, "minimum", i64::MIN),
        Event::input(2, "negative one", -1),
        Event::div(3, "overflow", 1, 2),
    ])
    .unwrap();
    assert_eq!(
        graph.evaluate().unwrap_err(),
        ExecutionError::ArithmeticOverflow { event_id: 3 }
    );
}

#[test]
fn scheduler_handles_repeated_operand_with_one_declared_dependency() {
    let graph = CompiledGraph::compile([Event::input(1, "x", 21), Event::add(2, "x plus x", 1, 1)])
        .unwrap();
    let expected = graph.evaluate().unwrap();
    let mut engine = CausalEngine::new(2).unwrap();
    let actual = engine.run(&graph).unwrap();
    assert_eq!(actual.execution, expected);
    assert_eq!(actual.execution.values[&2], 42);
    assert_eq!(actual.metrics.dependency_count, 1);
}

#[test]
fn adaptive_policy_uses_sequential_for_cheap_integer_dags() {
    use chronos_core::ExecutionStrategy;
    let graph = graph();
    let mut engine = CausalEngine::new(4).unwrap();
    let output = engine.run_adaptive(&graph).unwrap();

    assert_eq!(output.metrics.strategy, ExecutionStrategy::Sequential);
    assert_eq!(output.execution, graph.evaluate().unwrap());
}

#[test]
fn adaptive_policy_parallelizes_expensive_wide_dags() {
    use chronos_core::ExecutionStrategy;
    let mut events = Vec::new();
    for index in 0..8u64 {
        let input_id = index + 1;
        let work_id = index + 101;
        events.push(Event::input(
            input_id,
            format!("source-{index}"),
            index as i64 + 3,
        ));
        events.push(Event::hash_rounds(
            work_id,
            format!("hash-{index}"),
            input_id,
            100_000,
        ));
    }
    let graph = CompiledGraph::compile(events).unwrap();
    let expected = graph.evaluate().unwrap();
    let mut engine = CausalEngine::new(4).unwrap();
    let output = engine.run_adaptive(&graph).unwrap();

    assert_eq!(output.metrics.strategy, ExecutionStrategy::Parallel);
    assert_eq!(output.execution, expected);
    assert!(output.metrics.maximum_in_flight > 1);
}

#[test]
fn hash_round_operation_is_deterministic_and_bounded() {
    use chronos_core::{GraphError, MAX_HASH_ROUNDS};
    let graph = CompiledGraph::compile([
        Event::input(1, "source", 123),
        Event::hash_rounds(2, "hash", 1, 1000),
    ])
    .unwrap();
    let first = graph.evaluate().unwrap();
    let second = graph.evaluate().unwrap();
    assert_eq!(first, second);

    let error = CompiledGraph::compile([
        Event::input(1, "source", 123),
        Event::hash_rounds(2, "too much", 1, MAX_HASH_ROUNDS + 1),
    ])
    .unwrap_err();
    assert!(matches!(
        error,
        GraphError::InvalidHashRounds { event_id: 2, .. }
    ));
}

#[test]
fn graph_rejects_excessive_aggregate_hash_work() {
    use chronos_core::{GraphError, MAX_HASH_ROUNDS, MAX_TOTAL_HASH_ROUNDS};
    let count = (MAX_TOTAL_HASH_ROUNDS / u64::from(MAX_HASH_ROUNDS)) as usize + 1;
    let mut events = Vec::new();
    for index in 0..count {
        let source = index as u64 + 1;
        let output = index as u64 + 100;
        events.push(Event::input(
            source,
            format!("source-{index}"),
            index as i64,
        ));
        events.push(Event::hash_rounds(
            output,
            format!("work-{index}"),
            source,
            MAX_HASH_ROUNDS,
        ));
    }

    assert!(matches!(
        CompiledGraph::compile(events),
        Err(GraphError::AggregateWorkBudgetExceeded { .. })
    ));
}

#[test]
fn graph_accepts_exact_aggregate_hash_round_budget() {
    use chronos_core::{MAX_HASH_ROUNDS, MAX_TOTAL_HASH_ROUNDS};
    let operations = (MAX_TOTAL_HASH_ROUNDS / u64::from(MAX_HASH_ROUNDS)) as usize;
    let mut events = Vec::with_capacity(operations * 2);
    for index in 0..operations {
        let input = index as u64 + 1;
        let output = input + 100;
        events.push(Event::input(input, format!("source-{index}"), index as i64));
        events.push(Event::hash_rounds(
            output,
            format!("work-{index}"),
            input,
            MAX_HASH_ROUNDS,
        ));
    }

    let graph = CompiledGraph::compile(events).unwrap();
    assert_eq!(graph.event_count(), operations * 2);
    assert_eq!(
        graph.estimated_work_units(),
        MAX_TOTAL_HASH_ROUNDS + operations as u64
    );
}
