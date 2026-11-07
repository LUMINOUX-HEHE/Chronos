use chronos_core::{CompiledGraph, Event, ExecutionError};

#[test]
fn checked_add_subtract_and_multiply_report_overflow() {
    let add = CompiledGraph::compile([
        Event::input(1, "maximum", i64::MAX),
        Event::input(2, "one", 1),
        Event::add(3, "overflow add", 1, 2),
    ])
    .unwrap();
    assert_eq!(
        add.evaluate().unwrap_err(),
        ExecutionError::ArithmeticOverflow { event_id: 3 }
    );

    let subtract = CompiledGraph::compile([
        Event::input(1, "minimum", i64::MIN),
        Event::input(2, "one", 1),
        Event::sub(3, "overflow subtract", 1, 2),
    ])
    .unwrap();
    assert_eq!(
        subtract.evaluate().unwrap_err(),
        ExecutionError::ArithmeticOverflow { event_id: 3 }
    );

    let multiply = CompiledGraph::compile([
        Event::input(1, "maximum", i64::MAX),
        Event::input(2, "two", 2),
        Event::mul(3, "overflow multiply", 1, 2),
    ])
    .unwrap();
    assert_eq!(
        multiply.evaluate().unwrap_err(),
        ExecutionError::ArithmeticOverflow { event_id: 3 }
    );
}

#[test]
fn division_by_zero_is_an_explicit_error_in_reference_and_adaptive_modes() {
    use chronos_core::{CausalEngine, RuntimeError};

    let graph = CompiledGraph::compile([
        Event::input(1, "numerator", 42),
        Event::input(2, "zero", 0),
        Event::div(3, "divide by zero", 1, 2),
    ])
    .unwrap();

    assert_eq!(
        graph.evaluate().unwrap_err(),
        ExecutionError::DivisionByZero { event_id: 3 }
    );

    let mut engine = CausalEngine::new(2).unwrap();
    assert!(matches!(
        engine.run_adaptive(&graph),
        Err(RuntimeError::Execution(ExecutionError::DivisionByZero {
            event_id: 3
        }))
    ));
}
