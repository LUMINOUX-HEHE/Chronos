use crate::{CompiledGraph, EventId, ExecutionError, ExecutionReport, Operation};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncrementalReport {
    pub execution: ExecutionReport,
    /// Events actually evaluated during this update, in canonical order.
    pub recomputed_events: Vec<EventId>,
    /// Events whose prior values were reused, in canonical order.
    pub reused_events: Vec<EventId>,
    /// Source values supplied by the caller.
    pub applied_inputs: BTreeMap<EventId, i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IncrementalError {
    UnknownEvent(EventId),
    NotInputEvent(EventId),
    InvalidBaseReport,
    Execution(ExecutionError),
}

impl fmt::Display for IncrementalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(id) => write!(f, "cannot update unknown event {id}"),
            Self::NotInputEvent(id) => write!(f, "event {id} is not an input event"),
            Self::InvalidBaseReport => {
                write!(
                    f,
                    "base report does not match this graph's canonical structure"
                )
            }
            Self::Execution(error) => write!(f, "incremental execution failed: {error}"),
        }
    }
}

impl std::error::Error for IncrementalError {}

impl From<ExecutionError> for IncrementalError {
    fn from(value: ExecutionError) -> Self {
        Self::Execution(value)
    }
}

impl CompiledGraph {
    /// Recompute only changed input events and their transitive descendants.
    ///
    /// The base report must be a successful report previously produced by this
    /// exact compiled graph. Its structure and event coverage are checked, but
    /// its existing values are trusted rather than independently re-evaluated.
    pub fn recompute_inputs(
        &self,
        base: &ExecutionReport,
        updates: &BTreeMap<EventId, i64>,
    ) -> Result<IncrementalReport, IncrementalError> {
        if base.order != self.canonical_order()
            || base.layers != self.layers()
            || base.values.len() != self.event_count()
            || self.events.keys().any(|id| !base.values.contains_key(id))
        {
            return Err(IncrementalError::InvalidBaseReport);
        }

        for event_id in updates.keys() {
            let event = self
                .events
                .get(event_id)
                .ok_or(IncrementalError::UnknownEvent(*event_id))?;
            if !matches!(event.operation, Operation::Input { .. }) {
                return Err(IncrementalError::NotInputEvent(*event_id));
            }
        }

        let mut changed = BTreeSet::new();
        for (event_id, new_value) in updates {
            if base.values.get(event_id) != Some(new_value) {
                changed.insert(*event_id);
            }
        }

        let mut affected = changed.clone();
        let mut stack: Vec<EventId> = changed.iter().copied().collect();
        while let Some(event_id) = stack.pop() {
            if let Some(children) = self.dependents_of(event_id) {
                for child in children {
                    if affected.insert(*child) {
                        stack.push(*child);
                    }
                }
            }
        }

        let mut values = base.values.clone();
        let mut recomputed_events = Vec::with_capacity(affected.len());

        for event_id in self.canonical_order() {
            if !affected.contains(event_id) {
                continue;
            }

            let value = if let Some(input_value) = updates.get(event_id) {
                *input_value
            } else {
                self.events[event_id]
                    .operation
                    .evaluate(*event_id, &values)?
            };
            values.insert(*event_id, value);
            recomputed_events.push(*event_id);
        }

        let reused_events = self
            .canonical_order()
            .iter()
            .filter(|event_id| !affected.contains(event_id))
            .copied()
            .collect();

        Ok(IncrementalReport {
            execution: ExecutionReport {
                values,
                order: self.canonical_order().to_vec(),
                layers: self.layers().to_vec(),
            },
            recomputed_events,
            reused_events,
            applied_inputs: updates.clone(),
        })
    }
}
