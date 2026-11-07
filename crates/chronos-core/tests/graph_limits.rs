use chronos_core::{CompiledGraph, Event, GraphError, MAX_EVENT_LABEL_BYTES, MAX_GRAPH_EVENTS};

#[test]
fn direct_graph_compilation_enforces_event_count_limit() {
    let events = (0..=MAX_GRAPH_EVENTS).map(|index| Event::input(index as u64 + 1, "event", 0));

    assert!(matches!(
        CompiledGraph::compile(events),
        Err(GraphError::TooManyEvents(count)) if count == MAX_GRAPH_EVENTS + 1
    ));
}

#[test]
fn direct_graph_compilation_enforces_label_byte_limit() {
    let event = Event::input(1, "x".repeat(MAX_EVENT_LABEL_BYTES + 1), 7);

    assert!(matches!(
        CompiledGraph::compile([event]),
        Err(GraphError::EventLabelTooLong { event_id: 1, bytes })
            if bytes == MAX_EVENT_LABEL_BYTES + 1
    ));
}
