use crate::{CompiledGraph, EventId, ExecutionError, ExecutionReport, Operation};
use crossbeam_channel::{Receiver, Sender, unbounded};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread::{self, JoinHandle};
use std::time::Instant;

const MAX_WORKERS: usize = 256;
const MIN_PARALLEL_WORK_UNITS: u64 = 100_000;

enum WorkerMessage {
    Execute(WorkItem),
    Stop,
}

struct WorkItem {
    event_id: EventId,
    operation: Operation,
    inputs: [(EventId, i64); 2],
    input_count: usize,
}

struct Completion {
    event_id: EventId,
    result: Result<i64, ExecutionError>,
}

struct WorkerPool {
    jobs: Sender<WorkerMessage>,
    completions: Receiver<Completion>,
    threads: Vec<JoinHandle<()>>,
    worker_count: usize,
}

impl WorkerPool {
    fn new(worker_count: usize) -> Result<Self, RuntimeError> {
        if !(1..=MAX_WORKERS).contains(&worker_count) {
            return Err(RuntimeError::InvalidWorkerCount(worker_count));
        }

        let (job_tx, job_rx) = unbounded::<WorkerMessage>();
        let (completion_tx, completion_rx) = unbounded::<Completion>();
        let mut threads = Vec::with_capacity(worker_count);

        for index in 0..worker_count {
            let jobs = job_rx.clone();
            let completions = completion_tx.clone();
            let builder = thread::Builder::new().name(format!("chronos-worker-{index}"));
            match builder.spawn(move || worker_loop(jobs, completions)) {
                Ok(handle) => threads.push(handle),
                Err(error) => {
                    for _ in 0..threads.len() {
                        let _ = job_tx.send(WorkerMessage::Stop);
                    }
                    for handle in threads {
                        let _ = handle.join();
                    }
                    return Err(RuntimeError::WorkerStart(error.to_string()));
                }
            }
        }

        Ok(Self {
            jobs: job_tx,
            completions: completion_rx,
            threads,
            worker_count,
        })
    }

    fn submit(&self, work: WorkItem) -> Result<(), RuntimeError> {
        self.jobs
            .send(WorkerMessage::Execute(work))
            .map_err(|_| RuntimeError::WorkerPoolClosed)
    }

    fn receive(&self) -> Result<Completion, RuntimeError> {
        self.completions
            .recv()
            .map_err(|_| RuntimeError::WorkerPoolClosed)
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() {
            let _ = self.jobs.send(WorkerMessage::Stop);
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

fn worker_loop(jobs: Receiver<WorkerMessage>, completions: Sender<Completion>) {
    while let Ok(message) = jobs.recv() {
        match message {
            WorkerMessage::Stop => break,
            WorkerMessage::Execute(work) => {
                let WorkItem {
                    event_id,
                    operation,
                    inputs,
                    input_count,
                } = work;
                let result = catch_unwind(AssertUnwindSafe(|| {
                    operation.evaluate_inputs(event_id, &inputs[..input_count])
                }))
                .unwrap_or(Err(ExecutionError::WorkerPanicked));

                if completions.send(Completion { event_id, result }).is_err() {
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    InvalidWorkerCount(usize),
    WorkerStart(String),
    WorkerPoolClosed,
    SchedulerStalled { completed: usize, total: usize },
    Execution(ExecutionError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkerCount(count) => {
                write!(
                    f,
                    "worker count must be between 1 and {MAX_WORKERS}, got {count}"
                )
            }
            Self::WorkerStart(message) => write!(f, "could not start worker thread: {message}"),
            Self::WorkerPoolClosed => write!(f, "CHRONOS worker pool closed unexpectedly"),
            Self::SchedulerStalled { completed, total } => write!(
                f,
                "scheduler stalled after completing {completed} of {total} events"
            ),
            Self::Execution(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<ExecutionError> for RuntimeError {
    fn from(value: ExecutionError) -> Self {
        Self::Execution(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    Sequential,
    Parallel,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionMetrics {
    pub strategy: ExecutionStrategy,
    pub elapsed_ns: u128,
    pub event_count: usize,
    pub dependency_count: usize,
    pub worker_count: usize,
    pub maximum_in_flight: usize,
    pub maximum_ready_queue: usize,
    pub graph_depth: usize,
    pub graph_maximum_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeOutput {
    pub execution: ExecutionReport,
    pub metrics: ExecutionMetrics,
    /// Actual worker completion order; may vary between runs.
    pub completion_order: Vec<EventId>,
}

pub struct CausalEngine {
    configured_workers: usize,
    pool: Option<WorkerPool>,
}

impl CausalEngine {
    /// Creates a runtime configuration. Worker threads start lazily on the first parallel run and are then reused.
    pub fn new(worker_count: usize) -> Result<Self, RuntimeError> {
        if !(1..=MAX_WORKERS).contains(&worker_count) {
            return Err(RuntimeError::InvalidWorkerCount(worker_count));
        }
        Ok(Self {
            configured_workers: worker_count,
            pool: None,
        })
    }

    pub fn worker_count(&self) -> usize {
        self.configured_workers
    }

    fn ensure_pool(&mut self) -> Result<&WorkerPool, RuntimeError> {
        if self.pool.is_none() {
            self.pool = Some(WorkerPool::new(self.configured_workers)?);
        }
        Ok(self.pool.as_ref().expect("worker pool initialized"))
    }

    /// Select sequential execution for inexpensive/narrow graphs and use the
    /// persistent worker pool only when the graph exposes enough estimated work
    /// and parallel width. The estimate is a conservative heuristic, not a
    /// promise of speedup; the chosen strategy is included in the run metrics.
    pub fn run_adaptive(&mut self, graph: &CompiledGraph) -> Result<RuntimeOutput, RuntimeError> {
        let work_units = graph.estimated_work_units();
        let should_parallelize = graph.maximum_width() >= 2
            && self.configured_workers >= 2
            && work_units >= MIN_PARALLEL_WORK_UNITS;

        if should_parallelize {
            return self.run(graph);
        }

        let started = Instant::now();
        let execution = graph.evaluate()?;
        let elapsed_ns = started.elapsed().as_nanos();
        let event_count = graph.event_count();
        Ok(RuntimeOutput {
            completion_order: execution.order.clone(),
            execution,
            metrics: ExecutionMetrics {
                strategy: ExecutionStrategy::Sequential,
                elapsed_ns,
                event_count,
                dependency_count: graph.dependency_count(),
                worker_count: usize::from(event_count > 0),
                maximum_in_flight: usize::from(event_count > 0),
                maximum_ready_queue: graph.layers().first().map_or(0, Vec::len),
                graph_depth: graph.depth(),
                graph_maximum_width: graph.maximum_width(),
            },
        })
    }

    /// Work-conserving ready-queue scheduler.
    ///
    /// It dispatches newly enabled events as soon as a worker slot is free;
    /// it does not wait for an entire topological layer to complete.
    /// Stable event IDs break ties in the ready queue. Because all operations
    /// are pure and inputs are explicit, completion order cannot affect values.
    pub fn run(&mut self, graph: &CompiledGraph) -> Result<RuntimeOutput, RuntimeError> {
        let pool = self.ensure_pool()?;
        let started = Instant::now();
        let total = graph.event_count();
        let mut remaining: BTreeMap<EventId, usize> = graph
            .dependencies
            .iter()
            .map(|(id, dependencies)| (*id, dependencies.len()))
            .collect();
        let mut ready: BTreeSet<EventId> = remaining
            .iter()
            .filter_map(|(id, count)| (*count == 0).then_some(*id))
            .collect();
        let mut values = BTreeMap::new();
        let mut completion_order = Vec::with_capacity(total);
        let mut pending = 0usize;
        let mut maximum_in_flight = 0usize;
        let mut maximum_ready_queue = ready.len();
        let mut first_error: Option<ExecutionError> = None;

        loop {
            while first_error.is_none() && pending < pool.worker_count && !ready.is_empty() {
                let event_id = ready.pop_first().expect("ready queue is non-empty");
                let event = &graph.events[&event_id];
                let dependencies = &graph.dependencies[&event_id];
                let mut inputs = [(0, 0); 2];
                for (index, dependency) in dependencies.iter().enumerate() {
                    let value =
                        values
                            .get(dependency)
                            .copied()
                            .ok_or(RuntimeError::SchedulerStalled {
                                completed: values.len(),
                                total,
                            })?;
                    inputs[index] = (*dependency, value);
                }

                pool.submit(WorkItem {
                    event_id,
                    operation: event.operation,
                    inputs,
                    input_count: dependencies.len(),
                })?;
                pending += 1;
                maximum_in_flight = maximum_in_flight.max(pending);
            }

            if pending == 0 {
                if let Some(error) = first_error.take() {
                    return Err(error.into());
                }
                if values.len() == total {
                    break;
                }
                return Err(RuntimeError::SchedulerStalled {
                    completed: values.len(),
                    total,
                });
            }

            let completion = pool.receive()?;
            pending -= 1;
            completion_order.push(completion.event_id);

            match completion.result {
                Ok(value) => {
                    if first_error.is_none() {
                        values.insert(completion.event_id, value);
                        if let Some(children) = graph.dependents.get(&completion.event_id) {
                            for child in children {
                                let count = remaining
                                    .get_mut(child)
                                    .expect("dependent event has indegree entry");
                                *count -= 1;
                                if *count == 0 {
                                    ready.insert(*child);
                                }
                            }
                        }
                        maximum_ready_queue = maximum_ready_queue.max(ready.len());
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        let elapsed_ns = started.elapsed().as_nanos();
        let execution = ExecutionReport {
            values,
            order: graph.canonical_order().to_vec(),
            layers: graph.layers().to_vec(),
        };

        Ok(RuntimeOutput {
            execution,
            metrics: ExecutionMetrics {
                strategy: ExecutionStrategy::Parallel,
                elapsed_ns,
                event_count: total,
                dependency_count: graph.dependency_count(),
                worker_count: pool.worker_count,
                maximum_in_flight,
                maximum_ready_queue,
                graph_depth: graph.depth(),
                graph_maximum_width: graph.maximum_width(),
            },
            completion_order,
        })
    }
}
