use std::process::Command;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("chronos-cli-test-{}-{name}", std::process::id()))
}

fn run(binary: &str, args: &[&str]) -> std::process::Output {
    Command::new(binary).args(args).output().unwrap()
}

#[test]
fn cli_lifecycle_init_validate_inspect_explain_dot_run_replay() {
    let program = temp_path("demo.chronos.json");
    let record = temp_path("demo.run.json");
    let dot = temp_path("demo.dot");
    let _ = std::fs::remove_file(&program);
    let _ = std::fs::remove_file(&record);
    let _ = std::fs::remove_file(&dot);
    let binary = env!("CARGO_BIN_EXE_chronos");

    let help = run(binary, &["help"]);
    assert!(help.status.success());
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("chronos compile"));
    assert!(help_text.contains("chronos run-source"));
    assert!(help_text.contains("chronos certify"));
    assert!(help_text.contains("chronos verify-cert"));
    assert!(help_text.contains("chronos recompute"));

    let duplicate_workers = run(
        binary,
        &[
            "run",
            program.to_str().unwrap(),
            "--workers",
            "2",
            "--workers",
            "3",
        ],
    );
    assert!(!duplicate_workers.status.success());
    assert!(
        String::from_utf8_lossy(&duplicate_workers.stderr)
            .contains("--workers may be specified only once")
    );

    let unknown_option = run(binary, &["validate", program.to_str().unwrap(), "--typo"]);
    assert!(!unknown_option.status.success());
    assert!(String::from_utf8_lossy(&unknown_option.stderr).contains("unknown option"));

    let extra_argument = run(
        binary,
        &["validate", program.to_str().unwrap(), "unexpected"],
    );
    assert!(!extra_argument.status.success());
    assert!(String::from_utf8_lossy(&extra_argument.stderr).contains("expects 1 positional"));

    let init = run(binary, &["init", program.to_str().unwrap()]);
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let validate = run(binary, &["validate", program.to_str().unwrap()]);
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stderr)
    );
    assert!(String::from_utf8_lossy(&validate.stdout).contains("VALID"));

    let inspect = run(binary, &["inspect", program.to_str().unwrap()]);
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("maximum_width"));

    let explain = run(binary, &["explain", program.to_str().unwrap(), "5"]);
    assert!(
        explain.status.success(),
        "{}",
        String::from_utf8_lossy(&explain.stderr)
    );
    assert!(String::from_utf8_lossy(&explain.stdout).contains("causal explanation"));

    let export = run(
        binary,
        &[
            "export-dot",
            program.to_str().unwrap(),
            dot.to_str().unwrap(),
        ],
    );
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let dot_text = std::fs::read_to_string(&dot).unwrap();
    assert!(dot_text.contains("digraph CHRONOS"));
    assert!(dot_text.contains("n1 -> n3"));

    let normal_run = run(
        binary,
        &[
            "run",
            program.to_str().unwrap(),
            "--workers",
            "3",
            "--record",
            record.to_str().unwrap(),
        ],
    );
    assert!(
        normal_run.status.success(),
        "{}",
        String::from_utf8_lossy(&normal_run.stderr)
    );
    assert!(record.exists());

    let json_run = run(
        binary,
        &[
            "run",
            program.to_str().unwrap(),
            "--workers",
            "2",
            "--record",
            record.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        json_run.status.success(),
        "{}",
        String::from_utf8_lossy(&json_run.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&json_run.stdout).unwrap();
    assert_eq!(json["record_version"], 2);

    let replay = run(
        binary,
        &[
            "replay",
            program.to_str().unwrap(),
            record.to_str().unwrap(),
            "--workers",
            "2",
        ],
    );
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert!(String::from_utf8_lossy(&replay.stdout).contains("REPLAY VERIFIED"));

    let certificate = temp_path("demo.certificate.json");
    let incremental_report = temp_path("demo.incremental.json");

    let certify = run(
        binary,
        &[
            "certify",
            program.to_str().unwrap(),
            record.to_str().unwrap(),
            certificate.to_str().unwrap(),
        ],
    );
    assert!(
        certify.status.success(),
        "{}",
        String::from_utf8_lossy(&certify.stderr)
    );

    let verify_certificate = run(
        binary,
        &[
            "verify-cert",
            program.to_str().unwrap(),
            certificate.to_str().unwrap(),
        ],
    );
    assert!(
        verify_certificate.status.success(),
        "{}",
        String::from_utf8_lossy(&verify_certificate.stderr)
    );
    assert!(String::from_utf8_lossy(&verify_certificate.stdout).contains("CERTIFICATE VERIFIED"));

    let recompute = run(
        binary,
        &[
            "recompute",
            program.to_str().unwrap(),
            record.to_str().unwrap(),
            "1",
            "15",
            incremental_report.to_str().unwrap(),
        ],
    );
    assert!(
        recompute.status.success(),
        "{}",
        String::from_utf8_lossy(&recompute.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&incremental_report).unwrap()).unwrap();
    assert_eq!(report["execution"]["values"]["1"], 15);
    assert_eq!(report["execution"]["values"]["5"], 70);
    assert_eq!(report["recomputed_events"].as_array().unwrap().len(), 3);

    let _ = std::fs::remove_file(program);
    let _ = std::fs::remove_file(record);
    let _ = std::fs::remove_file(dot);
    let _ = std::fs::remove_file(certificate);
    let _ = std::fs::remove_file(incremental_report);
}

#[test]
fn cli_compiles_and_runs_textual_source() {
    let source_path = temp_path("source.chronos");
    let program_path = temp_path("source.chronos.json");
    let record_path = temp_path("source.run.json");
    let binary = env!("CARGO_BIN_EXE_chronos");

    std::fs::write(
        &source_path,
        r#"
program "CLI source test";
result = add(left, right);
left = input(40);
right = input(2);
"#,
    )
    .unwrap();

    let compile = run(
        binary,
        &[
            "compile",
            source_path.to_str().unwrap(),
            program_path.to_str().unwrap(),
        ],
    );
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(String::from_utf8_lossy(&compile.stdout).contains("SOURCE COMPILED"));

    let source_run = run(
        binary,
        &[
            "run-source",
            source_path.to_str().unwrap(),
            "--workers",
            "3",
            "--record",
            record_path.to_str().unwrap(),
        ],
    );
    assert!(
        source_run.status.success(),
        "{}",
        String::from_utf8_lossy(&source_run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&source_run.stdout).contains("CHRONOS source execution complete")
    );

    let replay = run(
        binary,
        &[
            "replay",
            program_path.to_str().unwrap(),
            record_path.to_str().unwrap(),
            "--workers",
            "2",
        ],
    );
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert!(String::from_utf8_lossy(&replay.stdout).contains("REPLAY VERIFIED"));

    let _ = std::fs::remove_file(source_path);
    let _ = std::fs::remove_file(program_path);
    let _ = std::fs::remove_file(record_path);
}
