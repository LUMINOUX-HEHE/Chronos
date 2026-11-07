use chronos_core::{CausalEngine, CompiledGraph, Event, ExecutionStrategy};
use std::hint::black_box;
use std::time::{Duration, Instant};

const WARMUPS: usize = 5;
const SAMPLES: usize = 31;
const WIDTHS: [usize; 3] = [64, 256, 1024];

fn chain(width: usize) -> Vec<Event> {
    let mut events = vec![
        Event::input(1, "chain-source", 1),
        Event::input(2, "chain-one", 1),
    ];
    let mut previous = 1;
    for index in 0..width {
        let id = index as u64 + 3;
        events.push(Event::add(id, format!("chain-{index}"), previous, 2));
        previous = id;
    }
    events
}

fn wide_antichain(width: usize) -> Vec<Event> {
    (0..width)
        .map(|index| Event::input(index as u64 + 1, format!("wide-{index}"), 1))
        .collect()
}

fn balanced_tree(width: usize) -> Vec<Event> {
    assert!(width >= 2 && width.is_power_of_two());
    let mut events = Vec::with_capacity(width * 2);
    let mut frontier = Vec::with_capacity(width);

    for index in 0..width {
        let id = index as u64 + 1;
        events.push(Event::input(id, format!("leaf-{index}"), 1));
        frontier.push(id);
    }

    let mut next_id = width as u64 + 1;
    while frontier.len() > 1 {
        let mut next = Vec::with_capacity(frontier.len() / 2);
        for index in (0..frontier.len()).step_by(2) {
            let left = frontier[index];
            let right = frontier[index + 1];
            events.push(Event::add(next_id, format!("sum-{next_id}"), left, right));
            next.push(next_id);
            next_id += 1;
        }
        frontier = next;
    }
    events
}

fn fanout(width: usize) -> Vec<Event> {
    let mut events = vec![
        Event::input(1, "fanout-source", 1),
        Event::input(2, "fanout-token", 1),
    ];
    for index in 0..width {
        let id = index as u64 + 3;
        events.push(Event::add(id, format!("fanout-{index}"), 1, 2));
    }
    events
}

fn skewed_depth(width: usize) -> Vec<Event> {
    let mut events = vec![
        Event::input(1, "critical-source", 1),
        Event::input(2, "critical-one", 1),
    ];
    let mut previous = 1;
    for index in 0..width {
        let id = index as u64 + 3;
        events.push(Event::add(id, format!("critical-{index}"), previous, 2));
        previous = id;
    }

    let independent_start = width as u64 + 3;
    for index in 0..width {
        let id = independent_start + index as u64;
        events.push(Event::input(id, format!("independent-{index}"), 1));
    }
    events
}

fn expensive_wide(width: usize, rounds: u32) -> Vec<Event> {
    let mut events = Vec::with_capacity(width * 2);
    for index in 0..width {
        events.push(Event::input(
            index as u64 + 1,
            format!("expensive-source-{index}"),
            index as i64 + 11,
        ));
    }
    for index in 0..width {
        let source = index as u64 + 1;
        let output = width as u64 + index as u64 + 1;
        events.push(Event::hash_rounds(
            output,
            format!("expensive-hash-{index}"),
            source,
            rounds,
        ));
    }
    events
}

fn latency_samples(mut f: impl FnMut(), warmups: usize, samples: usize) -> Vec<Duration> {
    for _ in 0..warmups {
        f();
    }

    let mut elapsed = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        f();
        elapsed.push(start.elapsed());
    }
    elapsed.sort_unstable();
    elapsed
}

fn median(samples: &[Duration]) -> Duration {
    samples[samples.len() / 2]
}

fn p95(samples: &[Duration]) -> Duration {
    let index = (samples.len() * 95).div_ceil(100) - 1;
    samples[index]
}

fn benchmark_case(name: &str, width: usize, events: Vec<Event>, workers: usize) {
    let compile_start = Instant::now();
    let graph = CompiledGraph::compile(events).expect("generated DAG is valid");
    let compile_time = compile_start.elapsed();
    let mut engine = CausalEngine::new(workers).expect("runtime is created");
    let expected = graph.evaluate().expect("reference evaluation succeeds");

    let sequential = latency_samples(
        || {
            black_box(graph.evaluate().expect("sequential evaluation"));
        },
        WARMUPS,
        SAMPLES,
    );
    let runtime = latency_samples(
        || {
            let output = engine.run(&graph).expect("causal runtime evaluation");
            assert_eq!(output.execution, expected);
            black_box(output);
        },
        WARMUPS,
        SAMPLES,
    );

    let mut selected_strategy = ExecutionStrategy::Sequential;
    let adaptive = latency_samples(
        || {
            let output = engine.run_adaptive(&graph).expect("adaptive evaluation");
            assert_eq!(output.execution, expected);
            selected_strategy = output.metrics.strategy;
            black_box(output);
        },
        WARMUPS,
        SAMPLES,
    );

    let summary = graph.summary();
    let seq_median = median(&sequential);
    let runtime_median = median(&runtime);
    let adaptive_median = median(&adaptive);
    let runtime_ratio = seq_median.as_nanos() as f64 / runtime_median.as_nanos().max(1) as f64;
    let adaptive_ratio = seq_median.as_nanos() as f64 / adaptive_median.as_nanos().max(1) as f64;

    println!(
        "shape={name:<16} width={width:>4} events={:>5} edges={:>5} depth={:>4} max_width={:>4} compile={compile_time:?} seq={seq_median:?}/{:?} parallel={runtime_median:?}/{:?} adaptive={adaptive_median:?}/{:?} policy={selected_strategy:?} seq/parallel={runtime_ratio:.3}x seq/adaptive={adaptive_ratio:.3}x",
        summary.events,
        summary.dependencies,
        summary.depth,
        summary.maximum_width,
        p95(&sequential),
        p95(&runtime),
        p95(&adaptive),
    );
}

fn main() {
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(256);

    println!("CHRONOS execution-policy workload suite");
    println!("workers={workers} warmups={WARMUPS} samples={SAMPLES} release profile");
    println!(
        "Each timing prints median/p95; parse, compile, process startup and pool startup excluded."
    );
    println!("seq/adaptive >1 means adaptive is faster than sequential; <1 means slower.");

    for width in WIDTHS {
        benchmark_case("chain", width, chain(width), workers);
        benchmark_case("wide-antichain", width, wide_antichain(width), workers);
        benchmark_case("balanced-tree", width, balanced_tree(width), workers);
        benchmark_case("fanout", width, fanout(width), workers);
        benchmark_case("skewed-depth", width, skewed_depth(width), workers);
    }

    // This bounded compute-heavy case tests whether useful task work can amortize
    // dynamic scheduling cost; it is deliberately separate from trivial arithmetic.
    benchmark_case("expensive-wide", 16, expensive_wide(16, 150_000), workers);
}
