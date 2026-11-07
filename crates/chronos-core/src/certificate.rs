use crate::{EventId, ExecutionReport, Program, write_file_atomic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const CERTIFICATE_FORMAT_VERSION: u32 = 1;
pub const MAX_CERTIFICATE_FILE_BYTES: u64 = 128 * 1024 * 1024;
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertifiedInput {
    pub event_id: EventId,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateEntry {
    pub event_id: EventId,
    pub inputs: Vec<CertifiedInput>,
    pub output: i64,
    pub previous_hash: String,
    pub entry_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCertificate {
    pub format_version: u32,
    pub program_hash: String,
    pub entries: Vec<CertificateEntry>,
    pub root_hash: String,
}

#[derive(Serialize)]
struct EntryPayload<'a> {
    event_id: EventId,
    inputs: &'a [CertifiedInput],
    output: i64,
    previous_hash: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateAudit {
    pub verified_events: usize,
    pub root_hash: String,
    pub execution: ExecutionReport,
}

#[derive(Debug)]
pub enum CertificateError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Program(String),
    UnsupportedVersion(u32),
    FileTooLarge(u64),
    ProgramFingerprintMismatch,
    EntryCountMismatch {
        expected: usize,
        actual: usize,
    },
    EventOrderMismatch {
        position: usize,
        expected: EventId,
        actual: EventId,
    },
    PreviousHashMismatch {
        event_id: EventId,
    },
    InputMismatch {
        event_id: EventId,
    },
    OutputMismatch {
        event_id: EventId,
    },
    EntryHashMismatch {
        event_id: EventId,
    },
    RootHashMismatch,
    InvalidStructure(String),
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "certificate I/O error: {error}"),
            Self::Json(error) => write!(f, "invalid certificate JSON: {error}"),
            Self::Program(error) => write!(f, "program validation failed: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported certificate format version {version}")
            }
            Self::FileTooLarge(bytes) => {
                write!(
                    f,
                    "certificate is {bytes} bytes; limit is {MAX_CERTIFICATE_FILE_BYTES}"
                )
            }
            Self::ProgramFingerprintMismatch => {
                write!(f, "certificate belongs to a different program")
            }
            Self::EntryCountMismatch { expected, actual } => {
                write!(f, "certificate has {actual} entries; expected {expected}")
            }
            Self::EventOrderMismatch {
                position,
                expected,
                actual,
            } => write!(
                f,
                "certificate entry {position} is event {actual}; expected event {expected}"
            ),
            Self::PreviousHashMismatch { event_id } => {
                write!(f, "previous-hash link is invalid at event {event_id}")
            }
            Self::InputMismatch { event_id } => {
                write!(
                    f,
                    "certified inputs do not match causal predecessors at event {event_id}"
                )
            }
            Self::OutputMismatch { event_id } => {
                write!(f, "certified output is incorrect at event {event_id}")
            }
            Self::EntryHashMismatch { event_id } => {
                write!(f, "trace entry hash is invalid at event {event_id}")
            }
            Self::RootHashMismatch => write!(f, "certificate root hash is invalid"),
            Self::InvalidStructure(message) => {
                write!(f, "invalid certificate structure: {message}")
            }
        }
    }
}

impl std::error::Error for CertificateError {}

impl From<std::io::Error> for CertificateError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CertificateError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl ExecutionCertificate {
    fn validate_structure(&self) -> Result<(), CertificateError> {
        if self.format_version != CERTIFICATE_FORMAT_VERSION {
            return Err(CertificateError::UnsupportedVersion(self.format_version));
        }
        if self.entries.len() > crate::MAX_PROGRAM_EVENTS {
            return Err(CertificateError::InvalidStructure(
                "entry count exceeds the program event limit".into(),
            ));
        }
        if !is_sha256_hex(&self.program_hash) || !is_sha256_hex(&self.root_hash) {
            return Err(CertificateError::InvalidStructure(
                "program_hash and root_hash must be lowercase SHA-256 hex strings".into(),
            ));
        }
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.inputs.len() > 2 {
                return Err(CertificateError::InvalidStructure(format!(
                    "entry {index} has more than two direct inputs"
                )));
            }
            if !is_sha256_hex(&entry.previous_hash) || !is_sha256_hex(&entry.entry_hash) {
                return Err(CertificateError::InvalidStructure(format!(
                    "entry {index} contains a malformed hash"
                )));
            }
        }
        Ok(())
    }

    /// Construct a compact, hash-chained execution trace from a valid reference report.
    /// This validates the report before issuing the certificate.
    pub fn create(program: &Program, report: &ExecutionReport) -> Result<Self, CertificateError> {
        let graph = program
            .validate()
            .map_err(|error| CertificateError::Program(error.to_string()))?;
        let expected = graph
            .evaluate()
            .map_err(|error| CertificateError::Program(error.to_string()))?;
        if &expected != report {
            return Err(CertificateError::Program(
                "provided execution report differs from reference semantics".into(),
            ));
        }

        let program_hash = program
            .fingerprint()
            .map_err(|error| CertificateError::Program(error.to_string()))?;
        let mut previous_hash = GENESIS_HASH.to_string();
        let mut entries = Vec::with_capacity(graph.event_count());

        for event_id in graph.canonical_order() {
            let dependencies = graph.dependencies_of(*event_id).unwrap_or(&[]);
            let inputs: Vec<CertifiedInput> = dependencies
                .iter()
                .map(|dependency| {
                    Ok(CertifiedInput {
                        event_id: *dependency,
                        value: *report.values.get(dependency).ok_or_else(|| {
                            CertificateError::Program(format!(
                                "reference report is missing dependency {dependency}"
                            ))
                        })?,
                    })
                })
                .collect::<Result<_, CertificateError>>()?;
            let output = *report.values.get(event_id).ok_or_else(|| {
                CertificateError::Program(format!("reference report is missing event {event_id}"))
            })?;
            let entry_hash = hash_entry(*event_id, &inputs, output, &previous_hash)?;
            entries.push(CertificateEntry {
                event_id: *event_id,
                inputs,
                output,
                previous_hash: previous_hash.clone(),
                entry_hash: entry_hash.clone(),
            });
            previous_hash = entry_hash;
        }

        Ok(Self {
            format_version: CERTIFICATE_FORMAT_VERSION,
            program_hash,
            entries,
            root_hash: previous_hash,
        })
    }

    /// Verify every operation from the supplied program and certified predecessor values.
    /// No previously trusted execution output is required.
    pub fn verify(&self, program: &Program) -> Result<CertificateAudit, CertificateError> {
        self.validate_structure()?;
        let actual_program_hash = program
            .fingerprint()
            .map_err(|error| CertificateError::Program(error.to_string()))?;
        if self.program_hash != actual_program_hash {
            return Err(CertificateError::ProgramFingerprintMismatch);
        }
        let graph = program
            .validate()
            .map_err(|error| CertificateError::Program(error.to_string()))?;

        if self.entries.len() != graph.event_count() {
            return Err(CertificateError::EntryCountMismatch {
                expected: graph.event_count(),
                actual: self.entries.len(),
            });
        }

        let mut values = BTreeMap::new();
        let mut previous_hash = GENESIS_HASH.to_string();

        for (position, (expected_id, entry)) in graph
            .canonical_order()
            .iter()
            .zip(&self.entries)
            .enumerate()
        {
            if entry.event_id != *expected_id {
                return Err(CertificateError::EventOrderMismatch {
                    position,
                    expected: *expected_id,
                    actual: entry.event_id,
                });
            }
            if entry.previous_hash != previous_hash {
                return Err(CertificateError::PreviousHashMismatch {
                    event_id: entry.event_id,
                });
            }

            let dependencies = graph.dependencies_of(entry.event_id).unwrap_or(&[]);
            let expected_inputs: Vec<CertifiedInput> = dependencies
                .iter()
                .map(|dependency| {
                    values
                        .get(dependency)
                        .copied()
                        .map(|value| CertifiedInput {
                            event_id: *dependency,
                            value,
                        })
                        .ok_or(CertificateError::InputMismatch {
                            event_id: entry.event_id,
                        })
                })
                .collect::<Result<_, _>>()?;

            if entry.inputs != expected_inputs {
                return Err(CertificateError::InputMismatch {
                    event_id: entry.event_id,
                });
            }

            let event = graph
                .event(entry.event_id)
                .expect("validated canonical event exists");
            let calculated = event
                .operation
                .evaluate(entry.event_id, &values)
                .map_err(|_| CertificateError::OutputMismatch {
                    event_id: entry.event_id,
                })?;
            if entry.output != calculated {
                return Err(CertificateError::OutputMismatch {
                    event_id: entry.event_id,
                });
            }

            let expected_hash = hash_entry(
                entry.event_id,
                &entry.inputs,
                entry.output,
                &entry.previous_hash,
            )?;
            if entry.entry_hash != expected_hash {
                return Err(CertificateError::EntryHashMismatch {
                    event_id: entry.event_id,
                });
            }

            values.insert(entry.event_id, entry.output);
            previous_hash = expected_hash;
        }

        if self.root_hash != previous_hash {
            return Err(CertificateError::RootHashMismatch);
        }

        Ok(CertificateAudit {
            verified_events: graph.event_count(),
            root_hash: self.root_hash.clone(),
            execution: ExecutionReport {
                values,
                order: graph.canonical_order().to_vec(),
                layers: graph.layers().to_vec(),
            },
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), CertificateError> {
        self.validate_structure()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() as u64 > MAX_CERTIFICATE_FILE_BYTES {
            return Err(CertificateError::FileTooLarge(bytes.len() as u64));
        }
        write_file_atomic(path, &bytes)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, CertificateError> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_CERTIFICATE_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_CERTIFICATE_FILE_BYTES {
            return Err(CertificateError::FileTooLarge(bytes.len() as u64));
        }
        let certificate: Self = serde_json::from_slice(&bytes)?;
        certificate.validate_structure()?;
        Ok(certificate)
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_entry(
    event_id: EventId,
    inputs: &[CertifiedInput],
    output: i64,
    previous_hash: &str,
) -> Result<String, CertificateError> {
    let payload = EntryPayload {
        event_id,
        inputs,
        output,
        previous_hash,
    };
    let encoded = serde_json::to_vec(&payload)?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}
