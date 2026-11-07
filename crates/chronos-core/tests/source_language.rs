use chronos_core::{SourceError, parse_source};

#[test]
fn parses_named_events_and_resolves_forward_references() {
    let source = r#"
        program "forward reference example";
        result = add(left, right);
        left = input(10);
        right = input(20);
    "#;

    let program = parse_source(source).unwrap();
    assert_eq!(program.name, "forward reference example");
    let graph = program.validate().unwrap();
    let report = graph.evaluate().unwrap();

    assert_eq!(report.values[&1], 30);
    assert_eq!(graph.layers(), &[vec![2, 3], vec![1]]);
}

#[test]
fn parses_comments_and_all_operation_forms() {
    let source = r#"
        program "language // comments are ignored";
        a = input(-12); # inline comment
        b = input(3);
        sum = add(a, b); // line comment
        difference = sub(a, b);
        product = mul(sum, difference);
        quotient = div(a, b);
        work = hash_rounds(a, 128);
    "#;

    let program = parse_source(source).unwrap();
    assert_eq!(program.name, "language // comments are ignored");
    let report = program.validate().unwrap().evaluate().unwrap();

    assert_eq!(report.values[&3], -9);
    assert_eq!(report.values[&4], -15);
    assert_eq!(report.values[&5], 135);
    assert_eq!(report.values[&6], -4);
    assert_eq!(program.events.len(), 7);
}

#[test]
fn rejects_duplicate_names_and_unknown_references() {
    let duplicate = r#"
        a = input(1);
        a = input(2);
    "#;
    assert!(matches!(
        parse_source(duplicate),
        Err(SourceError::Line { line: 3, .. })
    ));

    let unknown = r#"
        a = add(missing, missing);
    "#;
    assert!(matches!(
        parse_source(unknown),
        Err(SourceError::Line { line: 2, .. })
    ));
}

#[test]
fn rejects_cycles_bad_arity_and_unknown_operations() {
    let cycle = r#"
        a = add(b, b);
        b = sub(a, a);
    "#;
    assert!(matches!(
        parse_source(cycle),
        Err(SourceError::InvalidProgram(_))
    ));

    let arity = "a = add(1);";
    assert!(matches!(parse_source(arity), Err(SourceError::Line { .. })));

    let operation = "a = transcendental(1);";
    assert!(matches!(
        parse_source(operation),
        Err(SourceError::Line { .. })
    ));
}

#[test]
fn rejects_empty_source_and_invalid_program_declarations() {
    assert!(parse_source("# only a comment").is_err());
    assert!(parse_source("program not-a-string\na = input(1);").is_err());
    assert!(parse_source("program \"\"\na = input(1);").is_err());
}

#[test]
fn source_parser_enforces_event_and_identifier_limits_during_parsing() {
    use chronos_core::{MAX_EVENT_LABEL_BYTES, MAX_GRAPH_EVENTS};

    let oversized_name = format!("{} = input(1);", "x".repeat(MAX_EVENT_LABEL_BYTES + 1));
    assert!(matches!(
        parse_source(&oversized_name),
        Err(SourceError::Line { .. })
    ));

    let mut source = String::with_capacity(MAX_GRAPH_EVENTS * 20);
    for index in 0..=MAX_GRAPH_EVENTS {
        source.push_str(&format!("event_{index} = input(0);\n"));
    }
    assert!(matches!(
        parse_source(&source),
        Err(SourceError::Line { .. })
    ));
}
