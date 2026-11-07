use chronos_core::{
    CausalEngine, Event, ExecutionCertificate, Program, RunRecord, load_source, write_file_atomic,
};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

fn main() {
    if let Err(error) = run() {
        eprintln!("chronos: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    validate_command_args(command, &args[1..])?;

    match command {
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-V" => {
            println!("CHRONOS {}", env!("CARGO_PKG_VERSION"));
        }
        "init" => command_init(&args[1..])?,
        "compile" => command_compile_source(&args[1..])?,
        "run-source" => command_run_source(&args[1..])?,
        "validate" => command_validate(&args[1..])?,
        "inspect" => command_inspect(&args[1..])?,
        "explain" => command_explain(&args[1..])?,
        "export-dot" => command_export_dot(&args[1..])?,
        "run" => command_run(&args[1..])?,
        "replay" => command_replay(&args[1..])?,
        "certify" => command_certify(&args[1..])?,
        "verify-cert" => command_verify_certificate(&args[1..])?,
        "recompute" => command_recompute(&args[1..])?,
        "benchmark" => command_benchmark(&args[1..])?,
        other => return Err(cli_error(format!("unknown command '{other}'")).into()),
    }
    Ok(())
}

fn cli_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn validate_command_args(command: &str, args: &[String]) -> Result<(), io::Error> {
    let (minimum_positionals, maximum_positionals, valued_options, boolean_flags): (
        usize,
        usize,
        &[&str],
        &[&str],
    ) = match command {
        "help" | "--help" | "-h" | "version" | "--version" | "-V" => (0, 0, &[], &[]),
        "init" => (1, 1, &[], &["--force"]),
        "compile" => (2, 2, &[], &["--force"]),
        "run-source" | "run" => (
            1,
            1,
            &["--workers", "--record"],
            &["--json", "--force-parallel"],
        ),
        "validate" | "inspect" => (1, 1, &[], &[]),
        "explain" | "export-dot" | "verify-cert" => (2, 2, &[], &[]),
        "replay" => (2, 2, &["--workers"], &[]),
        "certify" => (3, 3, &[], &[]),
        "recompute" => (5, 5, &[], &[]),
        "benchmark" => (1, 1, &["--workers", "--iterations"], &[]),
        _ => return Ok(()),
    };

    let mut seen = std::collections::BTreeSet::new();
    let mut positionals = 0usize;
    let mut index = 0usize;
    while index < args.len() {
        let argument = &args[index];
        if !argument.starts_with("--") {
            positionals += 1;
            index += 1;
            continue;
        }

        if !valued_options.contains(&argument.as_str())
            && !boolean_flags.contains(&argument.as_str())
        {
            return Err(cli_error(format!(
                "unknown option '{argument}' for '{command}'"
            )));
        }
        if !seen.insert(argument.as_str()) {
            return Err(cli_error(format!("{argument} may be specified only once")));
        }

        if valued_options.contains(&argument.as_str()) {
            let value = args
                .get(index + 1)
                .ok_or_else(|| cli_error(format!("{argument} requires a value")))?;
            if value.starts_with("--") {
                return Err(cli_error(format!("{argument} requires a value")));
            }
            index += 2;
        } else {
            index += 1;
        }
    }

    if positionals < minimum_positionals || positionals > maximum_positionals {
        return Err(cli_error(format!(
            "'{command}' expects {minimum_positionals} positional argument(s), got {positionals}"
        )));
    }
    Ok(())
}

fn positional<'a>(
    args: &'a [String],
    index: usize,
    description: &str,
) -> Result<&'a str, io::Error> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| cli_error(format!("missing {description}")))
}

fn option_value<'a>(args: &'a [String], option: &str) -> Result<Option<&'a str>, io::Error> {
    let mut found = None;
    for (index, argument) in args.iter().enumerate() {
        if argument != option {
            continue;
        }
        if found.is_some() {
            return Err(cli_error(format!("{option} may be specified only once")));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| cli_error(format!("{option} requires a value")))?;
        if value.starts_with("--") {
            return Err(cli_error(format!("{option} requires a value")));
        }
        found = Some(value.as_str());
    }
    Ok(found)
}

fn parse_workers(args: &[String]) -> Result<usize, io::Error> {
    option_value(args, "--workers")?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| cli_error("--workers must be an integer"))
        })
        .unwrap_or_else(|| {
            Ok(std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1))
        })
}

fn load_program(path: &str) -> Result<(Program, chronos_core::CompiledGraph), Box<dyn Error>> {
    let program = Program::load(path)?;
    let graph = program.validate()?;
    Ok((program, graph))
}

fn command_init(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "output .chronos.json path")?;
    let force = args.iter().any(|arg| arg == "--force");
    if Path::new(path).exists() && !force {
        return Err(cli_error(format!("{path} already exists; pass --force to replace it")).into());
    }

    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let program = Program::new(
        "CHRONOS demonstration",
        vec![
            Event::input(1, "A", 10),
            Event::input(2, "B", 20),
            Event::add(3, "C = A + B", 1, 2),
            Event::input(4, "scale", 2),
            Event::mul(5, "D = C * scale", 3, 4),
            Event::input(6, "independent", 50),
        ],
    );
    program.save(path)?;
    println!("Created {path}");
    println!("Next: chronos validate {path}");
    Ok(())
}

fn command_compile_source(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source_path = positional(args, 0, "source .chronos path")?;
    let output_path = positional(args, 1, "output .chronos.json path")?;
    let force = args.iter().any(|arg| arg == "--force");

    if Path::new(output_path).exists() && !force {
        return Err(cli_error(format!(
            "{output_path} already exists; pass --force to replace it"
        ))
        .into());
    }

    let program = load_source(source_path)?;
    if let Some(parent) = Path::new(output_path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    program.save(output_path)?;
    println!("SOURCE COMPILED");
    println!("program: {}", program.name);
    println!("events: {}", program.events.len());
    println!("sha256: {}", program.fingerprint()?);
    println!("output: {output_path}");
    Ok(())
}

fn command_run_source(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source_path = positional(args, 0, "source .chronos path")?;
    let workers = parse_workers(args)?;
    let record_path = option_value(args, "--record")?;
    let json = args.iter().any(|arg| arg == "--json");
    let force_parallel = args.iter().any(|arg| arg == "--force-parallel");

    let program = load_source(source_path)?;
    let graph = program.validate()?;
    let mut engine = CausalEngine::new(workers)?;
    let output = if force_parallel {
        engine.run(&graph)?
    } else {
        engine.run_adaptive(&graph)?
    };
    let record = RunRecord::from_output(&program, output)?;

    if let Some(path) = record_path {
        record.save(path)?;
        if !json {
            println!("record: {path}");
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!("CHRONOS source execution complete");
        println!("program: {}", program.name);
        println!("source: {source_path}");
        println!("sha256: {}", record.program_hash);
        println!(
            "events: {} | dependencies: {} | strategy: {:?} | workers: {}",
            record.metrics.event_count,
            record.metrics.dependency_count,
            record.metrics.strategy,
            record.metrics.worker_count
        );
        println!(
            "elapsed: {} us | max in-flight: {} | ready-queue peak: {}",
            record.metrics.elapsed_ns / 1_000,
            record.metrics.maximum_in_flight,
            record.metrics.maximum_ready_queue
        );
        println!("canonical result values:");
        for (event_id, value) in &record.execution.values {
            let label = graph
                .event(*event_id)
                .map(|event| event.label.as_str())
                .unwrap_or("?");
            println!("  {event_id:>6}  {label:<24} = {value}");
        }
        println!("worker completion order: {:?}", record.completion_order);
    }
    Ok(())
}

fn command_validate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "program path")?;
    let (program, graph) = load_program(path)?;
    println!("VALID");
    println!("name: {}", program.name);
    println!("sha256: {}", program.fingerprint()?);
    let summary = graph.summary();
    println!(
        "events: {} | dependencies: {} | depth: {} | max width: {}",
        summary.events, summary.dependencies, summary.depth, summary.maximum_width
    );
    Ok(())
}

fn command_inspect(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "program path")?;
    let (program, graph) = load_program(path)?;
    println!("program: {}", program.name);
    println!("fingerprint: {}", program.fingerprint()?);
    println!("{}", serde_json::to_string_pretty(&graph.summary())?);
    Ok(())
}

fn command_explain(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "program path")?;
    let raw_id = positional(args, 1, "event id")?;
    let event_id = raw_id
        .parse::<u64>()
        .map_err(|_| cli_error("event id must be an unsigned integer"))?;
    let (program, graph) = load_program(path)?;
    let event = graph
        .event(event_id)
        .ok_or_else(|| cli_error(format!("event {event_id} does not exist")))?;
    let ancestors = graph
        .ancestors_of(event_id)
        .ok_or_else(|| cli_error(format!("event {event_id} does not exist")))?;
    let descendants = graph
        .descendants_of(event_id)
        .ok_or_else(|| cli_error(format!("event {event_id} does not exist")))?;
    let report = graph.evaluate()?;

    println!("program: {}", program.name);
    println!("target: {} ({})", event.id, event.label);
    println!(
        "direct dependencies: {:?}",
        graph.dependencies_of(event_id).unwrap_or(&[])
    );
    println!("ancestors: {:?}", ancestors);
    println!("descendants: {:?}", descendants);
    println!("causal explanation (topological order):");
    for id in ancestors.iter().copied().chain(std::iter::once(event_id)) {
        let item = graph.event(id).expect("causal event exists");
        let dependencies = graph.dependencies_of(id).unwrap_or(&[]);
        let value = report
            .values
            .get(&id)
            .expect("reference execution evaluated event");
        println!(
            "  {id:>6}  {:<24} deps={dependencies:?} value={value}",
            item.label
        );
    }
    Ok(())
}

fn dot_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn command_export_dot(args: &[String]) -> Result<(), Box<dyn Error>> {
    let program_path = positional(args, 0, "program path")?;
    let output_path = positional(args, 1, "output .dot path")?;
    let (program, graph) = load_program(program_path)?;

    let mut dot =
        String::from("digraph CHRONOS {\n  rankdir=LR;\n  node [shape=box, style=rounded];\n");
    for event_id in graph.canonical_order() {
        let event = graph.event(*event_id).expect("canonical event exists");
        let label = dot_escape(&format!("{}: {}", event.id, event.label));
        dot.push_str(&format!("  n{} [label=\"{}\"];\n", event.id, label));
    }
    for event_id in graph.canonical_order() {
        for dependency in graph.dependencies_of(*event_id).unwrap_or(&[]) {
            dot.push_str(&format!("  n{dependency} -> n{event_id};\n"));
        }
    }
    dot.push_str("}\n");

    if let Some(parent) = Path::new(output_path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    write_file_atomic(output_path, dot.as_bytes())?;
    println!(
        "Exported {} events and {} dependencies to {output_path}",
        graph.event_count(),
        graph.dependency_count()
    );
    println!("program: {}", program.name);
    Ok(())
}

fn command_run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "program path")?;
    let workers = parse_workers(args)?;
    let record_path = option_value(args, "--record")?;
    let json = args.iter().any(|arg| arg == "--json");
    let (program, graph) = load_program(path)?;

    let force_parallel = args.iter().any(|arg| arg == "--force-parallel");
    let mut engine = CausalEngine::new(workers)?;
    let output = if force_parallel {
        engine.run(&graph)?
    } else {
        engine.run_adaptive(&graph)?
    };
    let record = RunRecord::from_output(&program, output)?;

    if let Some(path) = record_path {
        record.save(path)?;
        if !json {
            println!("record: {path}");
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!("CHRONOS execution complete");
        println!("program: {}", program.name);
        println!("sha256: {}", record.program_hash);
        println!(
            "events: {} | dependencies: {} | strategy: {:?} | workers: {}",
            record.metrics.event_count,
            record.metrics.dependency_count,
            record.metrics.strategy,
            record.metrics.worker_count
        );
        println!(
            "elapsed: {} us | max in-flight: {} | ready-queue peak: {}",
            record.metrics.elapsed_ns / 1_000,
            record.metrics.maximum_in_flight,
            record.metrics.maximum_ready_queue
        );
        println!("canonical result values:");
        for (event_id, value) in &record.execution.values {
            let label = graph
                .event(*event_id)
                .map(|event| event.label.as_str())
                .unwrap_or("?");
            println!("  {event_id:>6}  {label:<24} = {value}");
        }
        println!("worker completion order: {:?}", record.completion_order);
    }
    Ok(())
}

fn command_replay(args: &[String]) -> Result<(), Box<dyn Error>> {
    let program_path = positional(args, 0, "program path")?;
    let record_path = positional(args, 1, "run record path")?;
    let workers = parse_workers(args)?;
    let program = Program::load(program_path)?;
    let record = RunRecord::load(record_path)?;
    let mut engine = CausalEngine::new(workers)?;
    let replay = record.verify_replay(&program, &mut engine)?;

    println!("REPLAY VERIFIED");
    println!("program sha256: {}", record.program_hash);
    println!("events verified: {}", replay.execution.values.len());
    println!("worker completion order is intentionally not compared.");
    Ok(())
}

fn command_certify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let program_path = positional(args, 0, "program path")?;
    let record_path = positional(args, 1, "run record path")?;
    let certificate_path = positional(args, 2, "output certificate path")?;
    let program = Program::load(program_path)?;
    let record = RunRecord::load(record_path)?;
    if record.program_hash != program.fingerprint()? {
        return Err(cli_error("run record belongs to a different program").into());
    }
    let certificate = ExecutionCertificate::create(&program, &record.execution)?;
    certificate.save(certificate_path)?;
    println!("CERTIFICATE CREATED");
    println!("verified events: {}", certificate.entries.len());
    println!("program sha256: {}", certificate.program_hash);
    println!("trace root: {}", certificate.root_hash);
    println!("saved: {certificate_path}");
    Ok(())
}

fn command_verify_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let program_path = positional(args, 0, "program path")?;
    let certificate_path = positional(args, 1, "certificate path")?;
    let program = Program::load(program_path)?;
    let certificate = ExecutionCertificate::load(certificate_path)?;
    let audit = certificate.verify(&program)?;
    println!("CERTIFICATE VERIFIED");
    println!("events independently evaluated: {}", audit.verified_events);
    println!("trace root: {}", audit.root_hash);
    Ok(())
}

fn command_recompute(args: &[String]) -> Result<(), Box<dyn Error>> {
    let program_path = positional(args, 0, "program path")?;
    let record_path = positional(args, 1, "base run record path")?;
    let raw_event_id = positional(args, 2, "input event id")?;
    let raw_value = positional(args, 3, "new input value")?;
    let output_path = positional(args, 4, "output incremental report path")?;
    let event_id = raw_event_id
        .parse::<u64>()
        .map_err(|_| cli_error("event id must be an unsigned integer"))?;
    let value = raw_value
        .parse::<i64>()
        .map_err(|_| cli_error("new input value must be a signed 64-bit integer"))?;

    let program = Program::load(program_path)?;
    let graph = program.validate()?;
    let record = RunRecord::load(record_path)?;
    let actual_hash = program.fingerprint()?;
    if record.program_hash != actual_hash {
        return Err(cli_error("base run record belongs to a different program").into());
    }

    let updates = std::collections::BTreeMap::from([(event_id, value)]);
    let incremental = graph.recompute_inputs(&record.execution, &updates)?;
    let bytes = serde_json::to_vec_pretty(&incremental)?;
    if let Some(parent) = Path::new(output_path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    write_file_atomic(output_path, &bytes)?;

    println!("INCREMENTAL RECOMPUTATION COMPLETE");
    println!("changed input: {event_id} = {value}");
    println!("recomputed events: {:?}", incremental.recomputed_events);
    println!("reused events: {}", incremental.reused_events.len());
    println!("report: {output_path}");
    Ok(())
}

fn command_benchmark(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = positional(args, 0, "program path")?;
    let workers = parse_workers(args)?;
    let iterations = option_value(args, "--iterations")?
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| cli_error("--iterations must be an integer"))?
        .unwrap_or(51);
    if !(3..=100_000).contains(&iterations) {
        return Err(cli_error("--iterations must be between 3 and 100000").into());
    }
    let (program, graph) = load_program(path)?;
    let mut engine = CausalEngine::new(workers)?;

    for _ in 0..10 {
        let _ = engine.run(&graph)?;
        let _ = engine.run_adaptive(&graph)?;
    }

    let mut sequential = Vec::with_capacity(iterations);
    let mut parallel = Vec::with_capacity(iterations);
    let mut adaptive = Vec::with_capacity(iterations);
    let mut selected_strategy = None;
    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let reference = graph.evaluate()?;
        sequential.push(start.elapsed().as_nanos());

        let start = std::time::Instant::now();
        let parallel_output = engine.run(&graph)?;
        parallel.push(start.elapsed().as_nanos());
        if parallel_output.execution != reference {
            return Err(cli_error("parallel runtime diverged from sequential reference").into());
        }

        let start = std::time::Instant::now();
        let adaptive_output = engine.run_adaptive(&graph)?;
        adaptive.push(start.elapsed().as_nanos());
        if adaptive_output.execution != reference {
            return Err(cli_error("adaptive runtime diverged from sequential reference").into());
        }
        selected_strategy = Some(adaptive_output.metrics.strategy);
    }

    sequential.sort_unstable();
    parallel.sort_unstable();
    adaptive.sort_unstable();
    let median_seq = sequential[sequential.len() / 2];
    let median_par = parallel[parallel.len() / 2];
    let median_adaptive = adaptive[adaptive.len() / 2];
    let p95_seq = sequential[(sequential.len() * 95).div_ceil(100) - 1];
    let p95_par = parallel[(parallel.len() * 95).div_ceil(100) - 1];
    let p95_adaptive = adaptive[(adaptive.len() * 95).div_ceil(100) - 1];

    println!("CHRONOS benchmark: {}", program.name);
    println!(
        "events: {} | workers: {} | warmups: 10 | samples: {iterations}",
        graph.event_count(),
        workers
    );
    println!("sequential median: {median_seq} ns | p95: {p95_seq} ns");
    println!("forced-parallel median: {median_par} ns | p95: {p95_par} ns");
    println!(
        "adaptive median: {median_adaptive} ns | p95: {p95_adaptive} ns | selected: {:?}",
        selected_strategy.unwrap_or(chronos_core::ExecutionStrategy::Sequential)
    );
    if median_par > 0 && median_adaptive > 0 {
        println!(
            "adaptive/forced-parallel speed ratio: {:.3}x",
            median_par as f64 / median_adaptive as f64
        );
    }
    println!("All three execution modes matched the sequential reference.");
    Ok(())
}

fn print_help() {
    println!(
        r#"CHRONOS — deterministic causal computation runtime

USAGE:
  chronos init <file> [--force]
  chronos compile <source.chronos> <output.json> [--force]
  chronos run-source <source.chronos> [--workers N] [--record <run.json>] [--json] [--force-parallel]
  chronos validate <file>
  chronos inspect <file>
  chronos explain <file> <event-id>
  chronos export-dot <file> <output.dot>
  chronos run <file> [--workers N] [--record <run.json>] [--json] [--force-parallel]
  chronos replay <file> <run.json> [--workers N]
  chronos certify <file> <run.json> <certificate.json>
  chronos verify-cert <file> <certificate.json>
  chronos recompute <file> <base-run.json> <input-id> <new-value> <report.json>
  chronos benchmark <file> [--workers N] [--iterations N]
  chronos version

PROGRAM FORMAT:
  JSON .chronos.json files and named .chronos source files.
  Source expressions: input, add, sub, mul, div, hash_rounds.
  See docs/SOURCE_LANGUAGE.md for the source grammar.

EXAMPLE:
  chronos run examples\quickstart.chronos.json --record results\quickstart.run.json
  chronos certify examples\quickstart.chronos.json results\quickstart.run.json results\quickstart.certificate.json
  chronos verify-cert examples\quickstart.chronos.json results\quickstart.certificate.json
"#
    );
}
