use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub type EventId = u64;
pub const MAX_HASH_ROUNDS: u32 = 5_000_000;
pub const MAX_TOTAL_HASH_ROUNDS: u64 = 25_000_000;
pub const MAX_GRAPH_EVENTS: usize = 100_000;
pub const MAX_EVENT_LABEL_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Input {
        value: i64,
    },
    Add {
        left: EventId,
        right: EventId,
    },
    Sub {
        left: EventId,
        right: EventId,
    },
    Mul {
        left: EventId,
        right: EventId,
    },
    Div {
        left: EventId,
        right: EventId,
    },
    /// Deterministic bounded CPU work used for compute-heavy DAG workloads.
    HashRounds {
        input: EventId,
        rounds: u32,
    },
}

impl Operation {
    pub(crate) fn dependencies(&self) -> Vec<EventId> {
        let mut ids = match *self {
            Self::Input { .. } => Vec::new(),
            Self::Add { left, right }
            | Self::Sub { left, right }
            | Self::Mul { left, right }
            | Self::Div { left, right } => vec![left, right],
            Self::HashRounds { input, .. } => vec![input],
        };
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(crate) fn evaluate(
        &self,
        event_id: EventId,
        values: &BTreeMap<EventId, i64>,
    ) -> Result<i64, ExecutionError> {
        self.evaluate_with(event_id, |dependency| {
            values
                .get(&dependency)
                .copied()
                .ok_or(ExecutionError::MissingRuntimeValue {
                    event_id,
                    dependency,
                })
        })
    }

    /// Evaluate against a tiny immutable input slice prepared by the scheduler.
    pub(crate) fn evaluate_inputs(
        &self,
        event_id: EventId,
        inputs: &[(EventId, i64)],
    ) -> Result<i64, ExecutionError> {
        self.evaluate_with(event_id, |dependency| {
            inputs
                .iter()
                .find_map(|(id, value)| (*id == dependency).then_some(*value))
                .ok_or(ExecutionError::MissingRuntimeValue {
                    event_id,
                    dependency,
                })
        })
    }

    fn evaluate_with(
        &self,
        event_id: EventId,
        mut get: impl FnMut(EventId) -> Result<i64, ExecutionError>,
    ) -> Result<i64, ExecutionError> {
        match *self {
            Self::Input { value } => Ok(value),
            Self::Add { left, right } => get(left)?
                .checked_add(get(right)?)
                .ok_or(ExecutionError::ArithmeticOverflow { event_id }),
            Self::Sub { left, right } => get(left)?
                .checked_sub(get(right)?)
                .ok_or(ExecutionError::ArithmeticOverflow { event_id }),
            Self::Mul { left, right } => get(left)?
                .checked_mul(get(right)?)
                .ok_or(ExecutionError::ArithmeticOverflow { event_id }),
            Self::Div { left, right } => {
                let divisor = get(right)?;
                if divisor == 0 {
                    return Err(ExecutionError::DivisionByZero { event_id });
                }
                get(left)?
                    .checked_div(divisor)
                    .ok_or(ExecutionError::ArithmeticOverflow { event_id })
            }
            Self::HashRounds { input, rounds } => {
                let mut state = get(input)? as u64;
                for round in 0..rounds {
                    state ^= state >> 30;
                    state = state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
                    state ^= state >> 27;
                    state = state.wrapping_mul(0x94d0_49bb_1331_11eb);
                    state ^= state >> 31;
                    state = state.rotate_left(17) ^ u64::from(round);
                }
                Ok(state as i64)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: EventId,
    pub label: String,
    #[serde(alias = "op")]
    pub operation: Operation,
}

impl Event {
    pub fn new(id: EventId, label: impl Into<String>, operation: Operation) -> Self {
        Self {
            id,
            label: label.into(),
            operation,
        }
    }

    pub fn input(id: EventId, label: impl Into<String>, value: i64) -> Self {
        Self::new(id, label, Operation::Input { value })
    }

    pub fn add(id: EventId, label: impl Into<String>, left: EventId, right: EventId) -> Self {
        Self::new(id, label, Operation::Add { left, right })
    }

    pub fn sub(id: EventId, label: impl Into<String>, left: EventId, right: EventId) -> Self {
        Self::new(id, label, Operation::Sub { left, right })
    }

    pub fn mul(id: EventId, label: impl Into<String>, left: EventId, right: EventId) -> Self {
        Self::new(id, label, Operation::Mul { left, right })
    }

    pub fn div(id: EventId, label: impl Into<String>, left: EventId, right: EventId) -> Self {
        Self::new(id, label, Operation::Div { left, right })
    }

    pub fn hash_rounds(id: EventId, label: impl Into<String>, input: EventId, rounds: u32) -> Self {
        Self::new(id, label, Operation::HashRounds { input, rounds })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    DuplicateEvent(EventId),
    EmptyLabel(EventId),
    MissingDependency {
        event_id: EventId,
        dependency: EventId,
    },
    Cycle,
    InvalidHashRounds {
        event_id: EventId,
        rounds: u32,
    },
    AggregateWorkBudgetExceeded {
        total_rounds: u64,
    },
    TooManyEvents(usize),
    EventLabelTooLong {
        event_id: EventId,
        bytes: usize,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEvent(id) => write!(f, "duplicate event id {id}"),
            Self::EmptyLabel(id) => write!(f, "event {id} has an empty label"),
            Self::MissingDependency {
                event_id,
                dependency,
            } => write!(f, "event {event_id} depends on missing event {dependency}"),
            Self::Cycle => write!(f, "causal dependencies contain a cycle"),
            Self::InvalidHashRounds { event_id, rounds } => write!(
                f,
                "event {event_id} requests {rounds} hash rounds; limit is {MAX_HASH_ROUNDS}"
            ),
            Self::AggregateWorkBudgetExceeded { total_rounds } => write!(
                f,
                "program requests {total_rounds} total hash rounds; limit is {MAX_TOTAL_HASH_ROUNDS}"
            ),
            Self::TooManyEvents(count) => write!(
                f,
                "graph contains {count} events; limit is {MAX_GRAPH_EVENTS}"
            ),
            Self::EventLabelTooLong { event_id, bytes } => write!(
                f,
                "event {event_id} label is {bytes} bytes; limit is {MAX_EVENT_LABEL_BYTES}"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    ArithmeticOverflow {
        event_id: EventId,
    },
    DivisionByZero {
        event_id: EventId,
    },
    MissingRuntimeValue {
        event_id: EventId,
        dependency: EventId,
    },
    WorkerPanicked,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow { event_id } => {
                write!(f, "checked arithmetic overflow at event {event_id}")
            }
            Self::DivisionByZero { event_id } => {
                write!(f, "division by zero at event {event_id}")
            }
            Self::MissingRuntimeValue {
                event_id,
                dependency,
            } => write!(
                f,
                "event {event_id} has no runtime value for dependency {dependency}"
            ),
            Self::WorkerPanicked => write!(f, "an execution worker panicked"),
        }
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Clone, Debug)]
pub struct CompiledGraph {
    pub(crate) events: BTreeMap<EventId, Event>,
    pub(crate) dependencies: BTreeMap<EventId, Vec<EventId>>,
    pub(crate) dependents: BTreeMap<EventId, Vec<EventId>>,
    layers: Vec<Vec<EventId>>,
    canonical_order: Vec<EventId>,
    estimated_work_units: u64,
    maximum_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReport {
    pub values: BTreeMap<EventId, i64>,
    /// Canonical topological order, independent of worker completion order.
    pub order: Vec<EventId>,
    pub layers: Vec<Vec<EventId>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphSummary {
    pub events: usize,
    pub dependencies: usize,
    pub depth: usize,
    pub maximum_width: usize,
    pub average_width: f64,
    pub source_events: usize,
    pub sink_events: usize,
    pub layers: Vec<Vec<EventId>>,
}

impl CompiledGraph {
    /// Validate a finite event set and compile deterministic topological layers.
    pub fn compile(events: impl IntoIterator<Item = Event>) -> Result<Self, GraphError> {
        let mut by_id = BTreeMap::new();
        for event in events {
            if event.label.trim().is_empty() {
                return Err(GraphError::EmptyLabel(event.id));
            }
            if event.label.len() > MAX_EVENT_LABEL_BYTES {
                return Err(GraphError::EventLabelTooLong {
                    event_id: event.id,
                    bytes: event.label.len(),
                });
            }
            let id = event.id;
            if by_id.contains_key(&id) {
                return Err(GraphError::DuplicateEvent(id));
            }
            if by_id.len() >= MAX_GRAPH_EVENTS {
                return Err(GraphError::TooManyEvents(by_id.len() + 1));
            }
            by_id.insert(id, event);
        }

        let total_hash_rounds: u64 = by_id
            .values()
            .map(|event| match event.operation {
                Operation::HashRounds { rounds, .. } => u64::from(rounds),
                _ => 0,
            })
            .sum();
        if total_hash_rounds > MAX_TOTAL_HASH_ROUNDS {
            return Err(GraphError::AggregateWorkBudgetExceeded {
                total_rounds: total_hash_rounds,
            });
        }

        let mut dependencies = BTreeMap::new();
        let mut dependents: BTreeMap<EventId, Vec<EventId>> =
            by_id.keys().map(|id| (*id, Vec::new())).collect();

        for (event_id, event) in &by_id {
            if let Operation::HashRounds { rounds, .. } = event.operation
                && rounds > MAX_HASH_ROUNDS
            {
                return Err(GraphError::InvalidHashRounds {
                    event_id: *event_id,
                    rounds,
                });
            }
            let deps = event.operation.dependencies();
            for dependency in &deps {
                if !by_id.contains_key(dependency) {
                    return Err(GraphError::MissingDependency {
                        event_id: *event_id,
                        dependency: *dependency,
                    });
                }
                dependents
                    .get_mut(dependency)
                    .expect("all event keys initialized")
                    .push(*event_id);
            }
            dependencies.insert(*event_id, deps);
        }

        for children in dependents.values_mut() {
            children.sort_unstable();
        }

        let mut indegree: BTreeMap<EventId, usize> = dependencies
            .iter()
            .map(|(id, deps)| (*id, deps.len()))
            .collect();
        let mut ready: BTreeSet<EventId> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect();

        let mut layers = Vec::new();
        let mut processed = 0usize;
        while !ready.is_empty() {
            let layer: Vec<EventId> = ready.iter().copied().collect();
            ready.clear();
            for event_id in &layer {
                processed += 1;
                for dependent in &dependents[event_id] {
                    let degree = indegree
                        .get_mut(dependent)
                        .expect("dependent event must have an indegree");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(*dependent);
                    }
                }
            }
            layers.push(layer);
        }

        if processed != by_id.len() {
            return Err(GraphError::Cycle);
        }

        let canonical_order = layers.iter().flatten().copied().collect();
        let estimated_work_units = by_id
            .values()
            .map(|event| match event.operation {
                Operation::Input { .. } => 1,
                Operation::Add { .. }
                | Operation::Sub { .. }
                | Operation::Mul { .. }
                | Operation::Div { .. } => 4,
                Operation::HashRounds { rounds, .. } => u64::from(rounds),
            })
            .sum();
        let maximum_width = layers.iter().map(Vec::len).max().unwrap_or(0);
        Ok(Self {
            events: by_id,
            dependencies,
            dependents,
            layers,
            canonical_order,
            estimated_work_units,
            maximum_width,
        })
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn dependency_count(&self) -> usize {
        self.dependencies.values().map(Vec::len).sum()
    }

    /// Conservative estimate used only to decide whether worker scheduling may pay off.
    /// It is a policy heuristic, not a performance guarantee.
    pub fn estimated_work_units(&self) -> u64 {
        self.estimated_work_units
    }

    pub fn maximum_width(&self) -> usize {
        self.maximum_width
    }

    pub fn depth(&self) -> usize {
        self.layers.len()
    }

    pub fn layers(&self) -> &[Vec<EventId>] {
        &self.layers
    }

    pub fn canonical_order(&self) -> &[EventId] {
        &self.canonical_order
    }

    pub fn contains(&self, id: EventId) -> bool {
        self.events.contains_key(&id)
    }

    pub fn event(&self, id: EventId) -> Option<&Event> {
        self.events.get(&id)
    }

    pub fn dependencies_of(&self, id: EventId) -> Option<&[EventId]> {
        self.dependencies.get(&id).map(Vec::as_slice)
    }

    pub fn dependents_of(&self, id: EventId) -> Option<&[EventId]> {
        self.dependents.get(&id).map(Vec::as_slice)
    }

    /// All strict ancestors in canonical topological order.
    pub fn ancestors_of(&self, id: EventId) -> Option<Vec<EventId>> {
        if !self.contains(id) {
            return None;
        }
        let mut stack = self.dependencies[&id].clone();
        let mut visited = BTreeSet::new();
        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                stack.extend(self.dependencies[&current].iter().copied());
            }
        }
        Some(
            self.canonical_order
                .iter()
                .filter(|candidate| visited.contains(candidate))
                .copied()
                .collect(),
        )
    }

    /// All strict descendants in canonical topological order.
    pub fn descendants_of(&self, id: EventId) -> Option<Vec<EventId>> {
        if !self.contains(id) {
            return None;
        }
        let mut stack = self.dependents[&id].clone();
        let mut visited = BTreeSet::new();
        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                stack.extend(self.dependents[&current].iter().copied());
            }
        }
        Some(
            self.canonical_order
                .iter()
                .filter(|candidate| visited.contains(candidate))
                .copied()
                .collect(),
        )
    }

    pub fn summary(&self) -> GraphSummary {
        let events = self.event_count();
        let depth = self.layers.len();
        let maximum_width = self.maximum_width;
        let average_width = if depth == 0 {
            0.0
        } else {
            events as f64 / depth as f64
        };
        let source_events = self
            .dependencies
            .values()
            .filter(|deps| deps.is_empty())
            .count();
        let sink_events = self
            .dependents
            .values()
            .filter(|children| children.is_empty())
            .count();

        GraphSummary {
            events,
            dependencies: self.dependency_count(),
            depth,
            maximum_width,
            average_width,
            source_events,
            sink_events,
            layers: self.layers.clone(),
        }
    }

    /// Strict causal order: true exactly when one event is an ancestor of another.
    pub fn is_causally_before(&self, before: EventId, after: EventId) -> bool {
        if before == after || !self.contains(before) || !self.contains(after) {
            return false;
        }
        let mut stack = self.dependencies[&after].clone();
        let mut visited = BTreeSet::new();
        while let Some(current) = stack.pop() {
            if current == before {
                return true;
            }
            if visited.insert(current) {
                stack.extend(self.dependencies[&current].iter().copied());
            }
        }
        false
    }

    /// Incomparability in the causal partial order.
    pub fn are_concurrent(&self, left: EventId, right: EventId) -> bool {
        self.contains(left)
            && self.contains(right)
            && left != right
            && !self.is_causally_before(left, right)
            && !self.is_causally_before(right, left)
    }

    /// Reference sequential semantics.
    pub fn evaluate(&self) -> Result<ExecutionReport, ExecutionError> {
        let mut values = BTreeMap::new();
        for event_id in &self.canonical_order {
            let event = &self.events[event_id];
            let value = event.operation.evaluate(*event_id, &values)?;
            values.insert(*event_id, value);
        }
        Ok(ExecutionReport {
            values,
            order: self.canonical_order.clone(),
            layers: self.layers.clone(),
        })
    }

    /// Dynamic ready-queue evaluation using a persistent worker pool for this run.
    pub fn evaluate_parallel(&self) -> Result<ExecutionReport, crate::RuntimeError> {
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(self.event_count().max(1));
        let mut engine = crate::CausalEngine::new(workers)?;
        Ok(engine.run(self)?.execution)
    }
}
