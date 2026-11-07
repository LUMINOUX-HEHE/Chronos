use chronos_core::{CompiledGraph, Event};
use std::collections::{BTreeMap, BTreeSet};

fn sample_events() -> Vec<Event> {
    vec![
        Event::input(11, "source-x", 12),
        Event::input(12, "source-y", 4),
        Event::add(13, "sum", 11, 12),
        Event::input(14, "independent", 99),
        Event::sub(15, "difference", 11, 12),
        Event::mul(16, "product", 13, 14),
    ]
}

#[test]
fn declaration_order_does_not_change_semantics() {
    let events = sample_events();
    let expected = CompiledGraph::compile(events.clone())
        .unwrap()
        .evaluate()
        .unwrap();

    for shift in 0..events.len() {
        let mut reordered = events.clone();
        reordered.rotate_left(shift);
        let actual = CompiledGraph::compile(reordered)
            .unwrap()
            .evaluate()
            .unwrap();
        assert_eq!(actual, expected);
    }

    let mut reversed = events;
    reversed.reverse();
    let actual = CompiledGraph::compile(reversed)
        .unwrap()
        .evaluate()
        .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn layers_partition_events_and_respect_every_dependency() {
    let events = sample_events();
    let graph = CompiledGraph::compile(events.clone()).unwrap();
    let mut layer_of = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for (layer_index, layer) in graph.layers().iter().enumerate() {
        for id in layer {
            assert!(seen.insert(*id), "event appears in multiple layers");
            layer_of.insert(*id, layer_index);
        }
        for left_index in 0..layer.len() {
            for right_index in (left_index + 1)..layer.len() {
                assert!(graph.are_concurrent(layer[left_index], layer[right_index]));
            }
        }
    }

    assert_eq!(seen.len(), graph.event_count());
    for event in events {
        let dependencies: Vec<u64> = match event.id {
            13 => vec![11, 12],
            15 => vec![11, 12],
            16 => vec![13, 14],
            _ => vec![],
        };
        for dependency in dependencies {
            assert!(layer_of[&dependency] < layer_of[&event.id]);
        }
    }
}

#[test]
fn repeated_operand_is_one_dependency_but_two_operands() {
    let graph =
        CompiledGraph::compile([Event::input(1, "x", 9), Event::add(2, "x plus x", 1, 1)]).unwrap();

    assert_eq!(graph.dependents_of(1), Some(&[2][..]));
    assert_eq!(graph.evaluate().unwrap().values[&2], 18);
}

#[test]
fn subtraction_and_unknown_event_queries_are_well_defined() {
    let graph = CompiledGraph::compile([
        Event::input(1, "minuend", 20),
        Event::input(2, "subtrahend", 5),
        Event::sub(3, "difference", 1, 2),
    ])
    .unwrap();

    assert_eq!(graph.evaluate().unwrap().values[&3], 15);
    assert!(!graph.is_causally_before(999, 3));
    assert!(!graph.are_concurrent(999, 3));
}
