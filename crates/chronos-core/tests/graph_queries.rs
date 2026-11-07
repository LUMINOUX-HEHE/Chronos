use chronos_core::{CompiledGraph, Event};

#[test]
fn ancestors_and_descendants_are_transitive_and_ordered() {
    let graph = CompiledGraph::compile([
        Event::input(1, "a", 2),
        Event::input(2, "b", 3),
        Event::add(3, "c", 1, 2),
        Event::input(4, "independent", 7),
        Event::mul(5, "d", 3, 4),
        Event::input(6, "unrelated", 9),
    ])
    .unwrap();

    // Layer order is [[1, 2, 4, 6], [3], [5]].
    assert_eq!(graph.ancestors_of(5), Some(vec![1, 2, 4, 3]));
    assert_eq!(graph.descendants_of(1), Some(vec![3, 5]));
    assert_eq!(graph.ancestors_of(1), Some(vec![]));
    assert_eq!(graph.descendants_of(6), Some(vec![]));
    assert_eq!(graph.ancestors_of(999), None);
    assert_eq!(graph.descendants_of(999), None);
}

#[test]
fn causal_queries_match_partial_order() {
    let graph = CompiledGraph::compile([
        Event::input(1, "x", 1),
        Event::input(2, "y", 2),
        Event::add(3, "sum", 1, 2),
        Event::mul(4, "twice-sum", 3, 3),
    ])
    .unwrap();

    let ancestors = graph.ancestors_of(4).unwrap();
    assert_eq!(ancestors, vec![1, 2, 3]);
    assert!(graph.is_causally_before(1, 4));
    assert!(!graph.are_concurrent(1, 4));
    assert!(graph.are_concurrent(1, 2));
}
