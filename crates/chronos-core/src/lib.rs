#![forbid(unsafe_code)]

//! CHRONOS: a deterministic causal-computation runtime.
//!
//! The crate separates graph semantics, worker scheduling, and portable
//! program/run-record formats. Its research claims remain deliberately
//! narrower than its engineering surface.

mod atomic_file;
mod certificate;
mod format;
mod graph;
mod incremental;
mod runtime;
mod source;

pub use atomic_file::write_file_atomic;
pub use certificate::{
    CERTIFICATE_FORMAT_VERSION, CertificateAudit, CertificateEntry, CertificateError,
    CertifiedInput, ExecutionCertificate, MAX_CERTIFICATE_FILE_BYTES,
};
pub use format::{
    FormatError, MAX_PROGRAM_EVENTS, MAX_PROGRAM_FILE_BYTES, MAX_PROGRAM_NAME_BYTES,
    MAX_RUN_RECORD_FILE_BYTES, Program, RunRecord,
};
pub use graph::{
    CompiledGraph, Event, EventId, ExecutionError, ExecutionReport, GraphError, GraphSummary,
    MAX_EVENT_LABEL_BYTES, MAX_GRAPH_EVENTS, MAX_HASH_ROUNDS, MAX_TOTAL_HASH_ROUNDS, Operation,
};
pub use incremental::{IncrementalError, IncrementalReport};
pub use runtime::{CausalEngine, ExecutionMetrics, ExecutionStrategy, RuntimeError, RuntimeOutput};
pub use source::{SourceError, load_source, parse_source};
