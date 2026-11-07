use chronos_core::{CompiledGraph, Event};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A and B produce the inputs for C. D depends on C and on "two".
    // E is independent of the A -> B -> C -> D computation.
    let graph = CompiledGraph::compile([
        Event::input(1, "A", 10),
        Event::input(2, "B", 20),
        Event::add(3, "C = A + B", 1, 2),
        Event::input(4, "two", 2),
        Event::mul(5, "D = C * two", 3, 4),
        Event::input(6, "E", 50),
    ])?;

    println!("CHRONOS v0.3.2 - basic causal computation");
    println!("events: {}", graph.event_count());
    println!("causal layers: {:?}", graph.layers());
    println!("A precedes D: {}", graph.is_causally_before(1, 5));
    println!("A and E concurrent: {}", graph.are_concurrent(1, 6));

    let sequential = graph.evaluate()?;
    let parallel = graph.evaluate_parallel()?;

    assert_eq!(sequential, parallel);
    println!("sequential and parallel reports match");
    for (event_id, value) in &sequential.values {
        println!("event {event_id} -> {value}");
    }
    Ok(())
}
