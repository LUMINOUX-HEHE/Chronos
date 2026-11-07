use chronos_core::{CausalEngine, CompiledGraph, Event, ExecutionCertificate, Operation};
use std::collections::BTreeMap;

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, upper_exclusive: u64) -> u64 {
        self.next() % upper_exclusive
    }
}

fn generated_events(seed: u64, count: u64) -> Vec<Event> {
    let mut rng = DeterministicRng::new(seed);
    let mut events = Vec::with_capacity(count as usize);

    for id in 1..=count {
        let choice = rng.next();
        if id == 1 || choice.is_multiple_of(5) {
            let value = rng.range(3) as i64 - 1;
            events.push(Event::input(id, format!("source-{id}"), value));
            continue;
        }

        // Every edge points to an earlier ID, so these generated programs are DAGs.
        let left = rng.range(id - 1) + 1;
        let right = rng.range(id - 1) + 1;
        let operation = if rng.next().is_multiple_of(2) {
            Operation::Add { left, right }
        } else {
            Operation::Sub { left, right }
        };
        events.push(Event::new(id, format!("event-{id}"), operation));
    }

    events
}

#[test]
fn generated_dags_parallel_execution_and_certificates_match_reference() {
    let mut engine = CausalEngine::new(4).unwrap();

    for seed in 1..=32 {
        let events = generated_events(seed * 7919, 40);
        let graph = CompiledGraph::compile(events.clone()).unwrap();
        let reference = graph.evaluate().unwrap();

        let parallel = engine.run(&graph).unwrap();
        assert_eq!(
            parallel.execution, reference,
            "parallel mismatch for seed {seed}"
        );

        let program = chronos_core::Program::new(format!("generated-{seed}"), events);
        let certificate = ExecutionCertificate::create(&program, &reference).unwrap();
        let audit = certificate.verify(&program).unwrap();
        assert_eq!(
            audit.execution, reference,
            "certificate mismatch for seed {seed}"
        );
    }
}

#[test]
fn generated_dags_incremental_recompute_matches_full_reexecution() {
    for seed in 101..=132 {
        let mut rng = DeterministicRng::new(seed * 104729);
        let events = generated_events(seed * 17, 48);
        let graph = CompiledGraph::compile(events.clone()).unwrap();
        let base = graph.evaluate().unwrap();

        let mut updates = BTreeMap::new();
        for event in &events {
            if matches!(event.operation, Operation::Input { .. }) && rng.range(4) == 0 {
                updates.insert(event.id, rng.range(11) as i64 - 5);
            }
        }

        let incremental = graph.recompute_inputs(&base, &updates).unwrap();

        let updated_events: Vec<Event> = events
            .into_iter()
            .map(|event| match updates.get(&event.id) {
                Some(value) => Event::input(event.id, event.label, *value),
                None => event,
            })
            .collect();
        let rebuilt = CompiledGraph::compile(updated_events).unwrap();
        let full = rebuilt.evaluate().unwrap();

        assert_eq!(
            incremental.execution, full,
            "incremental/full execution mismatch for seed {seed}"
        );

        let recomputed: std::collections::BTreeSet<_> =
            incremental.recomputed_events.iter().copied().collect();
        let reused: std::collections::BTreeSet<_> =
            incremental.reused_events.iter().copied().collect();
        assert!(recomputed.is_disjoint(&reused));
        assert_eq!(recomputed.len() + reused.len(), graph.event_count());
    }
}
