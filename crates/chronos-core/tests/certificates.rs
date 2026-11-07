use chronos_core::{CertificateError, Event, ExecutionCertificate, Program};

fn program() -> Program {
    Program::new(
        "certificate test",
        vec![
            Event::input(1, "left", 12),
            Event::input(2, "right", 4),
            Event::add(3, "sum", 1, 2),
            Event::hash_rounds(4, "deterministic work", 3, 100),
        ],
    )
}

#[test]
fn certificate_verifies_every_event_and_round_trips() {
    let program = program();
    let graph = program.validate().unwrap();
    let report = graph.evaluate().unwrap();
    let certificate = ExecutionCertificate::create(&program, &report).unwrap();

    let encoded = serde_json::to_vec(&certificate).unwrap();
    let decoded: ExecutionCertificate = serde_json::from_slice(&encoded).unwrap();
    let audit = decoded.verify(&program).unwrap();

    assert_eq!(audit.verified_events, 4);
    assert_eq!(audit.execution, report);
    assert_eq!(audit.root_hash, certificate.root_hash);
}

#[test]
fn certificate_detects_modified_output() {
    let program = program();
    let report = program.validate().unwrap().evaluate().unwrap();
    let mut certificate = ExecutionCertificate::create(&program, &report).unwrap();
    certificate.entries[2].output += 1;

    assert!(matches!(
        certificate.verify(&program),
        Err(CertificateError::OutputMismatch { event_id: 3 })
    ));
}

#[test]
fn certificate_detects_modified_input_value() {
    let program = program();
    let report = program.validate().unwrap().evaluate().unwrap();
    let mut certificate = ExecutionCertificate::create(&program, &report).unwrap();
    certificate.entries[2].inputs[0].value += 5;

    assert!(matches!(
        certificate.verify(&program),
        Err(CertificateError::InputMismatch { event_id: 3 })
    ));
}

#[test]
fn certificate_detects_hash_chain_tampering() {
    let program = program();
    let report = program.validate().unwrap().evaluate().unwrap();
    let mut certificate = ExecutionCertificate::create(&program, &report).unwrap();
    certificate.entries[0].entry_hash = "f".repeat(64);

    assert!(matches!(
        certificate.verify(&program),
        Err(CertificateError::EntryHashMismatch { event_id: 1 })
    ));
}

#[test]
fn certificate_is_bound_to_program_fingerprint() {
    let original = program();
    let report = original.validate().unwrap().evaluate().unwrap();
    let certificate = ExecutionCertificate::create(&original, &report).unwrap();

    let mut changed = original;
    changed.events[0] = Event::input(1, "left", 13);

    assert!(matches!(
        certificate.verify(&changed),
        Err(CertificateError::ProgramFingerprintMismatch)
    ));
}

#[test]
fn certificate_detects_root_hash_tampering() {
    let program = program();
    let report = program.validate().unwrap().evaluate().unwrap();
    let mut certificate = ExecutionCertificate::create(&program, &report).unwrap();
    certificate.root_hash = "0".repeat(64);

    assert!(matches!(
        certificate.verify(&program),
        Err(CertificateError::RootHashMismatch)
    ));
}

#[test]
fn certificate_rejects_structurally_oversized_input_lists() {
    let program = program();
    let report = program.validate().unwrap().evaluate().unwrap();
    let mut certificate = ExecutionCertificate::create(&program, &report).unwrap();
    certificate.entries[0].inputs = vec![
        chronos_core::CertifiedInput {
            event_id: 1,
            value: 1,
        },
        chronos_core::CertifiedInput {
            event_id: 2,
            value: 2,
        },
        chronos_core::CertifiedInput {
            event_id: 3,
            value: 3,
        },
    ];

    assert!(matches!(
        certificate.verify(&program),
        Err(CertificateError::InvalidStructure(_))
    ));
}
