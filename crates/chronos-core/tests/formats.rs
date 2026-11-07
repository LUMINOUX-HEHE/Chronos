use chronos_core::{CausalEngine, Event, FormatError, Program, RunRecord};

fn program() -> Program {
    Program::new(
        "format and replay test",
        vec![
            Event::input(1, "x", 8),
            Event::input(2, "y", 3),
            Event::sub(3, "difference", 1, 2),
        ],
    )
}

#[test]
fn program_json_round_trip_preserves_meaning() {
    let original = program();
    let encoded = serde_json::to_string_pretty(&original).unwrap();
    let decoded: Program = serde_json::from_str(&encoded).unwrap();
    assert_eq!(original, decoded);
    decoded.validate().unwrap();
}

#[test]
fn fingerprint_is_independent_of_event_declaration_order() {
    let first = program();
    let mut second = first.clone();
    second.events.reverse();
    assert_eq!(first.fingerprint().unwrap(), second.fingerprint().unwrap());
}

#[test]
fn run_record_verifies_replay() {
    let program = program();
    let graph = program.validate().unwrap();
    let mut engine = CausalEngine::new(3).unwrap();
    let output = engine.run(&graph).unwrap();
    let record = RunRecord::from_output(&program, output).unwrap();

    let encoded = serde_json::to_vec(&record).unwrap();
    let decoded: RunRecord = serde_json::from_slice(&encoded).unwrap();
    let replay = decoded.verify_replay(&program, &mut engine).unwrap();
    assert_eq!(replay.execution, decoded.execution);
}

#[test]
fn run_record_rejects_modified_program() {
    let original = program();
    let graph = original.validate().unwrap();
    let mut engine = CausalEngine::new(2).unwrap();
    let output = engine.run(&graph).unwrap();
    let record = RunRecord::from_output(&original, output).unwrap();

    let mut changed = original;
    changed.events[0] = Event::input(1, "x", 9);
    assert!(matches!(
        record.verify_replay(&changed, &mut engine),
        Err(FormatError::FingerprintMismatch { .. })
    ));
}

#[test]
fn unknown_program_version_is_rejected() {
    let mut invalid = program();
    invalid.format_version = 999;
    assert!(matches!(
        invalid.validate(),
        Err(FormatError::UnsupportedProgramVersion(999))
    ));
}

#[test]
fn program_rejects_event_count_above_resource_limit() {
    use chronos_core::MAX_PROGRAM_EVENTS;
    let events = (0..=MAX_PROGRAM_EVENTS)
        .map(|id| Event::input(id as u64 + 1, "event", 0))
        .collect();
    let program = Program::new("too many", events);
    assert!(matches!(
        program.validate(),
        Err(FormatError::TooManyEvents(_))
    ));
}

#[test]
fn program_rejects_oversized_event_label() {
    use chronos_core::MAX_EVENT_LABEL_BYTES;
    let program = Program::new(
        "oversized label",
        vec![Event::input(1, "x".repeat(MAX_EVENT_LABEL_BYTES + 1), 0)],
    );
    assert!(matches!(
        program.validate(),
        Err(FormatError::EventLabelTooLong { event_id: 1, .. })
    ));
}

#[test]
fn program_rejects_oversized_name() {
    use chronos_core::MAX_PROGRAM_NAME_BYTES;
    let program = Program::new("x".repeat(MAX_PROGRAM_NAME_BYTES + 1), vec![]);
    assert!(matches!(
        program.validate(),
        Err(FormatError::ProgramNameTooLong(_))
    ));
}

#[test]
fn run_record_rejects_unknown_execution_report_fields() {
    let program = program();
    let graph = program.validate().unwrap();
    let mut engine = CausalEngine::new(2).unwrap();
    let output = engine.run(&graph).unwrap();
    let record = RunRecord::from_output(&program, output).unwrap();

    let mut value = serde_json::to_value(record).unwrap();
    value["execution"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RunRecord>(value).is_err());
}

#[test]
fn run_record_rejects_duplicate_completion_ids_before_saving() {
    let program = program();
    let graph = program.validate().unwrap();
    let mut engine = CausalEngine::new(2).unwrap();
    let output = engine.run(&graph).unwrap();
    let mut record = RunRecord::from_output(&program, output).unwrap();
    record.completion_order[0] = record.completion_order[1];

    assert!(matches!(
        record.save(std::env::temp_dir().join("chronos-invalid-run-record.json")),
        Err(FormatError::InvalidRunRecord(_))
    ));
}
