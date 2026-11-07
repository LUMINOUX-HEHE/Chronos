use chronos_core::{CausalEngine, Event, ExecutionCertificate, Program, RunRecord};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn unique_path(label: &str) -> PathBuf {
    let sequence = PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "chronos-{label}-{}-{sequence}.json",
        std::process::id()
    ))
}

fn assert_no_temporary_siblings(destination: &Path) {
    let parent = destination.parent().unwrap();
    let prefix = format!(
        ".{}.chronos-tmp-",
        destination.file_name().unwrap().to_string_lossy()
    );
    let leftovers: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .filter(|name| name.starts_with(&prefix))
                .map(str::to_owned)
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporary files left behind: {leftovers:?}"
    );
}

#[test]
fn program_run_record_and_certificate_saves_replace_existing_files() {
    let program = Program::new(
        "atomic-save-test",
        vec![
            Event::input(1, "left", 19),
            Event::input(2, "right", 6),
            Event::sub(3, "difference", 1, 2),
        ],
    );
    let graph = program.validate().unwrap();
    let mut engine = CausalEngine::new(2).unwrap();
    let output = engine.run(&graph).unwrap();

    let program_path = unique_path("program");
    let record_path = unique_path("record");
    let certificate_path = unique_path("certificate");

    for path in [&program_path, &record_path, &certificate_path] {
        fs::write(path, b"old incomplete content").unwrap();
    }

    program.save(&program_path).unwrap();
    assert_eq!(Program::load(&program_path).unwrap(), program);

    let record = RunRecord::from_output(&program, output.clone()).unwrap();
    record.save(&record_path).unwrap();
    let loaded_record = RunRecord::load(&record_path).unwrap();
    assert_eq!(
        loaded_record
            .verify_replay(&program, &mut engine)
            .unwrap()
            .execution,
        output.execution
    );

    let certificate = ExecutionCertificate::create(&program, &output.execution).unwrap();
    certificate.save(&certificate_path).unwrap();
    let loaded_certificate = ExecutionCertificate::load(&certificate_path).unwrap();
    assert_eq!(
        loaded_certificate.verify(&program).unwrap().execution,
        output.execution
    );

    for path in [&program_path, &record_path, &certificate_path] {
        assert_no_temporary_siblings(path);
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn failed_atomic_replace_preserves_existing_destination_and_cleans_temporary_file() {
    let destination = unique_path("directory-target");
    fs::create_dir(&destination).unwrap();
    let marker = destination.join("keep.txt");
    fs::write(&marker, b"preserve me").unwrap();

    let program = Program::new("failed-replace", vec![Event::input(1, "x", 1)]);
    assert!(program.save(&destination).is_err());

    assert!(destination.is_dir());
    assert_eq!(fs::read(&marker).unwrap(), b"preserve me");
    assert_no_temporary_siblings(&destination);

    fs::remove_dir_all(destination).unwrap();
}
