use crate::{
    CausalEngine, CompiledGraph, Event, ExecutionMetrics, ExecutionReport, GraphError,
    RuntimeOutput, write_file_atomic,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const PROGRAM_FORMAT_VERSION: u32 = 1;
pub const RUN_RECORD_VERSION: u32 = 2;
pub const MAX_PROGRAM_EVENTS: usize = crate::graph::MAX_GRAPH_EVENTS;
pub const MAX_EVENT_LABEL_BYTES: usize = crate::graph::MAX_EVENT_LABEL_BYTES;
pub const MAX_PROGRAM_NAME_BYTES: usize = 4_096;
pub const MAX_PROGRAM_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_RUN_RECORD_FILE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Serialize)]
struct CanonicalProgram<'a> {
    format_version: u32,
    name: &'a str,
    events: Vec<&'a Event>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub format_version: u32,
    pub name: String,
    pub events: Vec<Event>,
}

#[derive(Debug)]
pub enum FormatError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedProgramVersion(u32),
    UnsupportedRecordVersion(u32),
    EmptyProgramName,
    ProgramNameTooLong(usize),
    TooManyEvents(usize),
    EventLabelTooLong { event_id: u64, bytes: usize },
    ProgramFileTooLarge(u64),
    RunRecordFileTooLarge(u64),
    Graph(GraphError),
    Runtime(String),
    FingerprintMismatch { expected: String, actual: String },
    ReplayMismatch,
    InvalidRunRecord(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "file I/O error: {error}"),
            Self::Json(error) => write!(f, "invalid CHRONOS JSON: {error}"),
            Self::UnsupportedProgramVersion(version) => {
                write!(f, "unsupported program format version {version}")
            }
            Self::UnsupportedRecordVersion(version) => {
                write!(f, "unsupported run-record version {version}")
            }
            Self::EmptyProgramName => write!(f, "program name cannot be empty"),
            Self::ProgramNameTooLong(bytes) => write!(
                f,
                "program name is {bytes} bytes; limit is {MAX_PROGRAM_NAME_BYTES}"
            ),
            Self::TooManyEvents(count) => write!(
                f,
                "program has {count} events; limit is {MAX_PROGRAM_EVENTS}"
            ),
            Self::EventLabelTooLong { event_id, bytes } => write!(
                f,
                "event {event_id} label is {bytes} bytes; limit is {MAX_EVENT_LABEL_BYTES}"
            ),
            Self::ProgramFileTooLarge(bytes) => write!(
                f,
                "program file is {bytes} bytes; limit is {MAX_PROGRAM_FILE_BYTES}"
            ),
            Self::RunRecordFileTooLarge(bytes) => write!(
                f,
                "run record is {bytes} bytes; limit is {MAX_RUN_RECORD_FILE_BYTES}"
            ),
            Self::Graph(error) => write!(f, "invalid causal graph: {error}"),
            Self::Runtime(error) => write!(f, "replay execution failed: {error}"),
            Self::FingerprintMismatch { expected, actual } => write!(
                f,
                "program fingerprint mismatch: record expects {expected}, current program is {actual}"
            ),
            Self::ReplayMismatch => {
                write!(
                    f,
                    "replayed values or canonical execution structure do not match the record"
                )
            }
            Self::InvalidRunRecord(message) => write!(f, "invalid run record: {message}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<std::io::Error> for FormatError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for FormatError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<GraphError> for FormatError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub record_version: u32,
    pub program_hash: String,
    pub execution: ExecutionReport,
    pub metrics: ExecutionMetrics,
    /// Actual worker completion order; informational and not replay-compared.
    pub completion_order: Vec<u64>,
}

impl Program {
    pub fn new(name: impl Into<String>, events: Vec<Event>) -> Self {
        Self {
            format_version: PROGRAM_FORMAT_VERSION,
            name: name.into(),
            events,
        }
    }

    pub fn validate(&self) -> Result<CompiledGraph, FormatError> {
        if self.format_version != PROGRAM_FORMAT_VERSION {
            return Err(FormatError::UnsupportedProgramVersion(self.format_version));
        }
        if self.name.trim().is_empty() {
            return Err(FormatError::EmptyProgramName);
        }
        if self.name.len() > MAX_PROGRAM_NAME_BYTES {
            return Err(FormatError::ProgramNameTooLong(self.name.len()));
        }
        if self.events.len() > MAX_PROGRAM_EVENTS {
            return Err(FormatError::TooManyEvents(self.events.len()));
        }
        if let Some(event) = self
            .events
            .iter()
            .find(|event| event.label.len() > MAX_EVENT_LABEL_BYTES)
        {
            return Err(FormatError::EventLabelTooLong {
                event_id: event.id,
                bytes: event.label.len(),
            });
        }
        let minimum_bytes = self
            .events
            .iter()
            .fold(self.name.len() as u64, |total, event| {
                total.saturating_add(event.label.len() as u64)
            });
        if minimum_bytes > MAX_PROGRAM_FILE_BYTES {
            return Err(FormatError::ProgramFileTooLarge(minimum_bytes));
        }
        Ok(CompiledGraph::compile(self.events.clone())?)
    }

    pub fn fingerprint(&self) -> Result<String, FormatError> {
        if self.format_version != PROGRAM_FORMAT_VERSION {
            return Err(FormatError::UnsupportedProgramVersion(self.format_version));
        }
        if self.name.trim().is_empty() {
            return Err(FormatError::EmptyProgramName);
        }
        self.validate()?;

        let mut canonical_events: Vec<&Event> = self.events.iter().collect();
        canonical_events.sort_by_key(|event| event.id);
        let canonical = CanonicalProgram {
            format_version: self.format_version,
            name: &self.name,
            events: canonical_events,
        };
        let bytes = serialize_bounded(
            &canonical,
            false,
            MAX_PROGRAM_FILE_BYTES,
            OutputLimit::Program,
        )?;
        let digest = Sha256::digest(bytes);
        Ok(format!("{digest:x}"))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, FormatError> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_PROGRAM_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PROGRAM_FILE_BYTES {
            return Err(FormatError::ProgramFileTooLarge(bytes.len() as u64));
        }
        let program: Self = serde_json::from_slice(&bytes)?;
        program.validate()?;
        Ok(program)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FormatError> {
        self.validate()?;
        let bytes = serialize_bounded(self, true, MAX_PROGRAM_FILE_BYTES, OutputLimit::Program)?;
        write_file_atomic(path, &bytes)?;
        Ok(())
    }
}

enum OutputLimit {
    Program,
    RunRecord,
}

struct BoundedBuffer {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}

impl Write for BoundedBuffer {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.bytes.len());
        let accepted = remaining.min(buffer.len());
        self.bytes.extend_from_slice(&buffer[..accepted]);
        if accepted != buffer.len() {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "serialized JSON exceeds configured size limit",
            ));
        }
        Ok(accepted)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_bounded<T: Serialize>(
    value: &T,
    pretty: bool,
    maximum: u64,
    kind: OutputLimit,
) -> Result<Vec<u8>, FormatError> {
    let mut buffer = BoundedBuffer {
        bytes: Vec::new(),
        maximum: maximum as usize,
        exceeded: false,
    };
    let result = if pretty {
        serde_json::to_writer_pretty(&mut buffer, value)
    } else {
        serde_json::to_writer(&mut buffer, value)
    };
    if buffer.exceeded {
        return Err(match kind {
            OutputLimit::Program => FormatError::ProgramFileTooLarge(maximum + 1),
            OutputLimit::RunRecord => FormatError::RunRecordFileTooLarge(maximum + 1),
        });
    }
    result?;
    Ok(buffer.bytes)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RunRecord {
    pub fn from_output(program: &Program, output: RuntimeOutput) -> Result<Self, FormatError> {
        let record = Self {
            record_version: RUN_RECORD_VERSION,
            program_hash: program.fingerprint()?,
            execution: output.execution,
            metrics: output.metrics,
            completion_order: output.completion_order,
        };
        record.validate_shape()?;
        Ok(record)
    }

    fn validate_shape(&self) -> Result<(), FormatError> {
        if self.record_version != RUN_RECORD_VERSION {
            return Err(FormatError::UnsupportedRecordVersion(self.record_version));
        }
        if !is_sha256_hex(&self.program_hash) {
            return Err(FormatError::InvalidRunRecord(
                "program_hash must be a 64-character lowercase SHA-256 hex string".into(),
            ));
        }

        let count = self.execution.values.len();
        if count > MAX_PROGRAM_EVENTS
            || self.execution.order.len() != count
            || self.completion_order.len() != count
            || self.metrics.event_count != count
        {
            return Err(FormatError::InvalidRunRecord(
                "event values, canonical order, completion order, and metrics must have the same bounded event count".into(),
            ));
        }

        let value_ids: BTreeSet<u64> = self.execution.values.keys().copied().collect();
        let order_ids: BTreeSet<u64> = self.execution.order.iter().copied().collect();
        let completion_ids: BTreeSet<u64> = self.completion_order.iter().copied().collect();
        if order_ids.len() != count || order_ids != value_ids {
            return Err(FormatError::InvalidRunRecord(
                "canonical order must contain each value event exactly once".into(),
            ));
        }
        if completion_ids.len() != count || completion_ids != value_ids {
            return Err(FormatError::InvalidRunRecord(
                "completion order must contain each value event exactly once".into(),
            ));
        }

        let mut layer_ids = BTreeSet::new();
        for layer in &self.execution.layers {
            for event_id in layer {
                if !layer_ids.insert(*event_id) {
                    return Err(FormatError::InvalidRunRecord(
                        "execution layers contain duplicate event IDs".into(),
                    ));
                }
            }
        }
        if self.execution.layers.len() > MAX_PROGRAM_EVENTS
            || self.execution.layers.iter().any(Vec::is_empty)
            || layer_ids != value_ids
            || self.metrics.graph_depth != self.execution.layers.len()
        {
            return Err(FormatError::InvalidRunRecord(
                "execution layers must partition all events and match graph depth".into(),
            ));
        }

        let maximum_width = self
            .execution
            .layers
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        if self.metrics.graph_maximum_width != maximum_width
            || self.metrics.maximum_ready_queue > count
            || self.metrics.maximum_in_flight > self.metrics.worker_count
            || self.metrics.worker_count > 256
            || self.metrics.dependency_count > count.saturating_mul(2)
            || (count > 0
                && (self.metrics.worker_count == 0
                    || self.metrics.maximum_in_flight == 0
                    || self.metrics.maximum_ready_queue == 0))
            || (self.metrics.strategy == crate::ExecutionStrategy::Sequential
                && count > 0
                && (self.metrics.worker_count != 1 || self.metrics.maximum_in_flight != 1))
        {
            return Err(FormatError::InvalidRunRecord(
                "runtime metrics are outside the valid bounds for this report".into(),
            ));
        }

        if self.metrics.strategy == crate::ExecutionStrategy::Sequential
            && self.completion_order != self.execution.order
        {
            return Err(FormatError::InvalidRunRecord(
                "sequential completion order must equal canonical execution order".into(),
            ));
        }
        Ok(())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FormatError> {
        self.validate_shape()?;
        let bytes = serialize_bounded(
            self,
            true,
            MAX_RUN_RECORD_FILE_BYTES,
            OutputLimit::RunRecord,
        )?;
        write_file_atomic(path, &bytes)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, FormatError> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_RUN_RECORD_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RUN_RECORD_FILE_BYTES {
            return Err(FormatError::RunRecordFileTooLarge(bytes.len() as u64));
        }
        let record: Self = serde_json::from_slice(&bytes)?;
        record.validate_shape()?;
        Ok(record)
    }

    /// Re-executes the program and verifies the deterministic portion of the record.
    pub fn verify_replay(
        &self,
        program: &Program,
        engine: &mut CausalEngine,
    ) -> Result<RuntimeOutput, FormatError> {
        self.validate_shape()?;
        let actual_hash = program.fingerprint()?;
        if self.program_hash != actual_hash {
            return Err(FormatError::FingerprintMismatch {
                expected: self.program_hash.clone(),
                actual: actual_hash,
            });
        }
        let graph = program.validate()?;
        let replay = engine
            .run(&graph)
            .map_err(|error| FormatError::Runtime(error.to_string()))?;
        if replay.execution != self.execution {
            return Err(FormatError::ReplayMismatch);
        }
        Ok(replay)
    }
}
