use chronos_core::{CompiledGraph, Event, IncrementalError};
use std::collections::BTreeMap;

fn graph() -> CompiledGraph {
    CompiledGraph::compile([
        Event::input(1, "a", 10),
        Event::input(2, "b", 20),
        Event::add(3, "sum", 1, 2),
        Event::mul(4, "scaled sum", 3, 2),
        Event::input(5, "independent", 7),
        Event::add(6, "independent result", 5, 2),
    ])
    .unwrap()
}

#[test]
fn recomputes_only_changed_input_and_descendants() {
    let graph = graph();
    let base = graph.evaluate().unwrap();
    let updates = BTreeMap::from([(1, 15)]);
    let incremental = graph.recompute_inputs(&base, &updates).unwrap();

    let expected_graph = CompiledGraph::compile([
        Event::input(1, "a", 15),
        Event::input(2, "b", 20),
        Event::add(3, "sum", 1, 2),
        Event::mul(4, "scaled sum", 3, 2),
        Event::input(5, "independent", 7),
        Event::add(6, "independent result", 5, 2),
    ])
    .unwrap();

    assert_eq!(incremental.execution, expected_graph.evaluate().unwrap());
    assert_eq!(incremental.recomputed_events, vec![1, 3, 4]);
    assert_eq!(incremental.reused_events, vec![2, 5, 6]);
    assert_eq!(incremental.execution.values[&4], 700);
    assert_eq!(incremental.execution.values[&6], 27);
}

#[test]
fn unchanged_input_reuses_everything() {
    let graph = graph();
    let base = graph.evaluate().unwrap();
    let updates = BTreeMap::from([(1, 10)]);
    let incremental = graph.recompute_inputs(&base, &updates).unwrap();

    assert_eq!(incremental.execution, base);
    assert!(incremental.recomputed_events.is_empty());
    assert_eq!(incremental.reused_events.len(), graph.event_count());
}

#[test]
fn multiple_changed_inputs_union_their_causal_slices() {
    let graph = graph();
    let base = graph.evaluate().unwrap();
    let updates = BTreeMap::from([(1, 11), (5, 9)]);
    let incremental = graph.recompute_inputs(&base, &updates).unwrap();

    assert_eq!(incremental.recomputed_events, vec![1, 5, 3, 6, 4]);
    assert_eq!(incremental.reused_events, vec![2]);
    assert_eq!(incremental.execution.values[&4], 620);
    assert_eq!(incremental.execution.values[&6], 29);
}

#[test]
fn rejects_unknown_and_non_input_updates() {
    let graph = graph();
    let base = graph.evaluate().unwrap();

    assert_eq!(
        graph.recompute_inputs(&base, &BTreeMap::from([(999, 1)])),
        Err(IncrementalError::UnknownEvent(999))
    );
    assert_eq!(
        graph.recompute_inputs(&base, &BTreeMap::from([(3, 1)])),
        Err(IncrementalError::NotInputEvent(3))
    );
}

#[test]
fn rejects_base_report_with_wrong_graph_structure() {
    let graph = graph();
    let mut base = graph.evaluate().unwrap();
    base.order.reverse();

    assert_eq!(
        graph.recompute_inputs(&base, &BTreeMap::from([(1, 12)])),
        Err(IncrementalError::InvalidBaseReport)
    );
}
