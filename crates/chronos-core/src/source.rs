use crate::{
    Event, FormatError, MAX_EVENT_LABEL_BYTES, MAX_GRAPH_EVENTS, MAX_PROGRAM_FILE_BYTES, Operation,
    Program,
};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub enum SourceError {
    Line { line: usize, message: String },
    InvalidProgram(String),
    SourceFileTooLarge(u64),
    Io(std::io::Error),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line { line, message } => write!(f, "source line {line}: {message}"),
            Self::InvalidProgram(message) => write!(f, "invalid CHRONOS source program: {message}"),
            Self::SourceFileTooLarge(bytes) => write!(
                f,
                "source file is {bytes} bytes; limit is {MAX_PROGRAM_FILE_BYTES}"
            ),
            Self::Io(error) => write!(f, "source file I/O error: {error}"),
        }
    }
}

impl std::error::Error for SourceError {}

impl From<std::io::Error> for SourceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<FormatError> for SourceError {
    fn from(value: FormatError) -> Self {
        Self::InvalidProgram(value.to_string())
    }
}

#[derive(Clone, Debug)]
enum RawOperation {
    Input(i64),
    Add(String, String),
    Sub(String, String),
    Mul(String, String),
    Div(String, String),
    HashRounds(String, u32),
}

#[derive(Clone, Debug)]
struct RawEvent {
    id: u64,
    name: String,
    line: usize,
    operation: RawOperation,
}

/// Parse the CHRONOS source language into a validated versioned Program.
///
/// Grammar (one declaration per line):
///
/// ```text
/// program "optional program name"
/// x = input(12)
/// sum = add(x, y)
/// difference = sub(x, y)
/// product = mul(sum, difference)
/// quotient = div(x, y)
/// work = hash_rounds(x, 100000)
/// ```
///
/// References may point forward; the graph compiler validates cycles and
/// dependency completeness after all names have been assigned event IDs.
pub fn parse_source(source: &str) -> Result<Program, SourceError> {
    if source.len() as u64 > MAX_PROGRAM_FILE_BYTES {
        return Err(SourceError::SourceFileTooLarge(source.len() as u64));
    }
    let mut program_name: Option<String> = None;
    let mut names: BTreeMap<String, u64> = BTreeMap::new();
    let mut raw_events = Vec::new();

    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let without_comment = strip_comment(original_line);
        let mut line = without_comment.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(without_semicolon) = line.strip_suffix(';') {
            line = without_semicolon.trim();
        }
        if line.is_empty() {
            continue;
        }

        if let Some(name_literal) = line.strip_prefix("program ") {
            if program_name.is_some() {
                return Err(line_error(
                    line_number,
                    "program name may be declared only once",
                ));
            }
            let parsed: String = serde_json::from_str(name_literal.trim()).map_err(|error| {
                line_error(
                    line_number,
                    format!("program name must be a quoted JSON string: {error}"),
                )
            })?;
            if parsed.trim().is_empty() {
                return Err(line_error(line_number, "program name cannot be empty"));
            }
            if parsed.len() > crate::MAX_PROGRAM_NAME_BYTES {
                return Err(line_error(
                    line_number,
                    format!(
                        "program name is {} bytes; limit is {}",
                        parsed.len(),
                        crate::MAX_PROGRAM_NAME_BYTES
                    ),
                ));
            }
            program_name = Some(parsed);
            continue;
        }

        let Some((raw_name, expression)) = line.split_once('=') else {
            return Err(line_error(
                line_number,
                "expected a declaration in the form name = operation(arguments)",
            ));
        };
        let name = raw_name.trim();
        if !valid_identifier(name) {
            return Err(line_error(
                line_number,
                format!("invalid event name '{name}'"),
            ));
        }
        if name.len() > MAX_EVENT_LABEL_BYTES {
            return Err(line_error(
                line_number,
                format!(
                    "event name is {} bytes; limit is {MAX_EVENT_LABEL_BYTES}",
                    name.len()
                ),
            ));
        }
        if names.contains_key(name) {
            return Err(line_error(
                line_number,
                format!("event name '{name}' is declared more than once"),
            ));
        }
        if raw_events.len() >= MAX_GRAPH_EVENTS {
            return Err(line_error(
                line_number,
                format!("program exceeds the {MAX_GRAPH_EVENTS}-event limit"),
            ));
        }

        let id = raw_events.len() as u64 + 1;
        let operation = parse_operation(expression.trim(), line_number)?;
        names.insert(name.to_string(), id);
        raw_events.push(RawEvent {
            id,
            name: name.to_string(),
            line: line_number,
            operation,
        });
    }

    if raw_events.is_empty() {
        return Err(SourceError::Line {
            line: 1,
            message: "source program contains no event declarations".into(),
        });
    }

    let mut events = Vec::with_capacity(raw_events.len());
    for raw in raw_events {
        let operation = match raw.operation {
            RawOperation::Input(value) => Operation::Input { value },
            RawOperation::Add(left, right) => Operation::Add {
                left: resolve_name(&names, &left, raw.line)?,
                right: resolve_name(&names, &right, raw.line)?,
            },
            RawOperation::Sub(left, right) => Operation::Sub {
                left: resolve_name(&names, &left, raw.line)?,
                right: resolve_name(&names, &right, raw.line)?,
            },
            RawOperation::Mul(left, right) => Operation::Mul {
                left: resolve_name(&names, &left, raw.line)?,
                right: resolve_name(&names, &right, raw.line)?,
            },
            RawOperation::Div(left, right) => Operation::Div {
                left: resolve_name(&names, &left, raw.line)?,
                right: resolve_name(&names, &right, raw.line)?,
            },
            RawOperation::HashRounds(input, rounds) => Operation::HashRounds {
                input: resolve_name(&names, &input, raw.line)?,
                rounds,
            },
        };
        events.push(Event::new(raw.id, raw.name, operation));
    }

    let program = Program::new(
        program_name.unwrap_or_else(|| "untitled".to_string()),
        events,
    );
    program.validate()?;
    Ok(program)
}

pub fn load_source(path: impl AsRef<Path>) -> Result<Program, SourceError> {
    let path = path.as_ref();
    let mut source = String::new();
    File::open(path)?
        .take(MAX_PROGRAM_FILE_BYTES + 1)
        .read_to_string(&mut source)?;
    if source.len() as u64 > MAX_PROGRAM_FILE_BYTES {
        return Err(SourceError::SourceFileTooLarge(source.len() as u64));
    }
    parse_source(&source)
}

fn line_error(line: usize, message: impl Into<String>) -> SourceError {
    SourceError::Line {
        line,
        message: message.into(),
    }
}

fn resolve_name(
    names: &BTreeMap<String, u64>,
    name: &str,
    line: usize,
) -> Result<u64, SourceError> {
    names
        .get(name)
        .copied()
        .ok_or_else(|| line_error(line, format!("unknown event name '{name}'")))
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_operation(expression: &str, line: usize) -> Result<RawOperation, SourceError> {
    let Some(open) = expression.find('(') else {
        return Err(line_error(line, "expected operation(arguments)"));
    };
    if !expression.ends_with(')') || open == 0 {
        return Err(line_error(line, "operation call must end with ')'"));
    }

    let operation_name = expression[..open].trim();
    let argument_text = &expression[open + 1..expression.len() - 1];
    let arguments: Vec<&str> = if argument_text.trim().is_empty() {
        Vec::new()
    } else {
        argument_text.split(',').map(str::trim).collect()
    };

    match operation_name {
        "input" => {
            require_arity(operation_name, &arguments, 1, line)?;
            let value = arguments[0]
                .parse::<i64>()
                .map_err(|_| line_error(line, "input() requires one signed 64-bit integer"))?;
            Ok(RawOperation::Input(value))
        }
        "add" | "sub" | "mul" | "div" => {
            require_arity(operation_name, &arguments, 2, line)?;
            let left = parse_identifier_argument(arguments[0], line)?;
            let right = parse_identifier_argument(arguments[1], line)?;
            match operation_name {
                "add" => Ok(RawOperation::Add(left, right)),
                "sub" => Ok(RawOperation::Sub(left, right)),
                "mul" => Ok(RawOperation::Mul(left, right)),
                "div" => Ok(RawOperation::Div(left, right)),
                _ => unreachable!(),
            }
        }
        "hash_rounds" => {
            require_arity(operation_name, &arguments, 2, line)?;
            let input = parse_identifier_argument(arguments[0], line)?;
            let rounds = arguments[1].parse::<u32>().map_err(|_| {
                line_error(
                    line,
                    "hash_rounds() requires an unsigned 32-bit round count",
                )
            })?;
            Ok(RawOperation::HashRounds(input, rounds))
        }
        other => Err(line_error(line, format!("unknown operation '{other}'"))),
    }
}

fn require_arity(
    operation: &str,
    arguments: &[&str],
    expected: usize,
    line: usize,
) -> Result<(), SourceError> {
    if arguments.len() != expected {
        return Err(line_error(
            line,
            format!(
                "{operation}() expects {expected} argument(s), got {}",
                arguments.len()
            ),
        ));
    }
    Ok(())
}

fn parse_identifier_argument(value: &str, line: usize) -> Result<String, SourceError> {
    if !valid_identifier(value) {
        return Err(line_error(
            line,
            format!("'{value}' is not a valid event reference"),
        ));
    }
    Ok(value.to_string())
}

/// Remove # and // comments only when outside a quoted string.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == '#' || line[index..].starts_with("//") {
            return &line[..index];
        }
    }

    line
}
