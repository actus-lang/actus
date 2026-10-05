use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{ArgumentRoleSource, SemanticErrorKind, SemanticModel, analyze};

fn analyze_source(source: &str) -> Result<SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse"))
}

const VALID_WRITER: &str = "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return 1; } }";

#[test]
fn validates_a_performance_against_its_role_contract() {
    analyze_source(VALID_WRITER).expect("matching role performance should pass");
}

#[test]
fn accepts_zero_sized_structs_and_marker_roles() {
    analyze_source(
        "struct Marker { } role MarkerRole { } perform MarkerRole for Marker { } verb main() -> Int { return 0; }",
    )
    .expect("empty structs and marker roles have explicit language meaning");
}

#[test]
fn rejects_duplicate_roles_and_role_methods() {
    let duplicate_role = analyze_source(
        "role Writer { verb write(abs self: Int); } role Writer { verb read(abs self: Int); }",
    )
    .expect_err("duplicate roles must fail");
    assert!(
        matches!(duplicate_role.kind, SemanticErrorKind::DuplicateRoleName { name } if name == "Writer")
    );

    let duplicate_method =
        analyze_source("role Writer { verb write(abs self: Int); verb write(abs self: Int); }")
            .expect_err("duplicate role methods must fail");
    assert!(
        matches!(duplicate_method.kind, SemanticErrorKind::DuplicateRoleMethod { role, method } if role == "Writer" && method == "write")
    );
}

#[test]
fn rejects_unknown_roles_and_missing_methods() {
    let unknown = analyze_source(
        "perform Missing for Int { verb write(abs self: Int) -> Int { return 0; } }",
    )
    .expect_err("unknown roles must fail");
    assert!(matches!(unknown.kind, SemanticErrorKind::UnknownRole { name } if name == "Missing"));

    let missing = analyze_source("struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; verb flush(abs self: File); } perform Writer for File { verb write(abs self: File) -> Int { return 0; } }")
        .expect_err("incomplete performances must fail");
    assert!(
        matches!(missing.kind, SemanticErrorKind::MissingRoleMethod { role, method } if role == "Writer" && method == "flush")
    );
}

#[test]
fn rejects_mismatched_and_implicit_performance_receivers() {
    let mismatch = analyze_source("struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(erg self: File) -> Int { return 0; } }")
        .expect_err("receiver role mismatches must fail");
    assert!(
        matches!(mismatch.kind, SemanticErrorKind::RoleMethodMismatch { role, method } if role == "Writer" && method == "write")
    );

    let invalid_receiver = analyze_source("role Writer { verb write(abs stream: Int); }")
        .expect_err("role methods need an explicit self receiver");
    assert!(
        matches!(invalid_receiver.kind, SemanticErrorKind::InvalidRoleReceiver { role, method } if role == "Writer" && method == "write")
    );
}

#[test]
fn resolves_performance_calls_and_records_reachable_implementations() {
    let model = analyze_source(
        "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return 1; } } verb main() -> Int { erg source = File { value: 1, }; abs file = ref source; return file.write(); }",
    )
    .expect("performed method calls should resolve");
    assert_eq!(model.reachable_performances.len(), 1);
    assert_eq!(model.reachable_performances[0].role_name, "Writer");
    assert_eq!(model.reachable_performances[0].target_type, "File");
    assert_eq!(model.reachable_performances[0].method_name, "write");
}

#[test]
fn resolves_dat_receivers_and_moves_the_receiver_owner() {
    let model = analyze_source(
        "struct File { value: Int, } role Consumer { verb consume(dat self: File); } perform Consumer for File { verb consume(dat self: File) { } } verb main() { erg file = File { value: 1, }; file.consume(); }",
    )
    .expect("performed dat receiver should resolve");
    assert_eq!(model.reachable_performances[0].role_name, "Consumer");
    assert!(matches!(model.bindings[0].ownership, actus::semantic::OwnershipState::Moved));
}

#[test]
fn infers_abs_role_only_for_an_abs_binding() {
    let model = analyze_source(
        "verb read(abs value: Int) -> Int { return 0; } verb main() -> Int { erg source = 41; abs view = ref source; return read(value: view); }",
    )
    .expect("an abs binding has one safe read-only role");
    let fact = model.argument_roles.last().expect("call role fact");
    assert_eq!(fact.callee, "read");
    assert_eq!(fact.parameter, "value");
    assert_eq!(fact.role, actus::ast::Role::Abs);
    assert_eq!(fact.source, ArgumentRoleSource::Inferred);
}

#[test]
fn infers_ins_role_only_for_an_ins_binding_and_resumes_it() {
    let model = analyze_source(
        "verb update(ins value: Int) { value += 1; } verb main() -> Int { ins value = 0; update(value: value); return 0; }",
    )
    .expect("an ins binding has one safe exclusive-loan role");
    let fact = model.argument_roles.last().expect("call role fact");
    assert_eq!(fact.role, actus::ast::Role::Ins);
    assert_eq!(fact.source, ArgumentRoleSource::Inferred);
    assert!(matches!(
        model.bindings.last().expect("value binding").access,
        actus::semantic::AccessState::Mutable
    ));
}

#[test]
fn rejects_inference_for_mutable_and_owned_bindings() {
    let mutable = analyze_source(
        "verb read(abs value: Int) { } verb main() { erg value = 1; read(value: value); }",
    )
    .expect_err("an erg binding must not be inferred as abs");
    assert!(matches!(mutable.kind, SemanticErrorKind::InvalidArgumentRole { .. }));

    let owned = analyze_source(
        "verb update(ins value: Int) { } verb main() { erg value = 1; update(value: value); }",
    )
    .expect_err("an erg binding must not be inferred as ins");
    assert!(matches!(owned.kind, SemanticErrorKind::InvalidArgumentRole { .. }));
}

#[test]
fn rejects_inference_for_buffers_and_external_calls() {
    let buffer = analyze_source(
        "verb update(ins value: Buffer) { } verb main() { ins value = Buffer[1]; update(value: value); }",
    )
    .expect_err("buffer inference must remain explicit");
    assert!(matches!(buffer.kind, SemanticErrorKind::InvalidArgumentRole { .. }));

    let external = analyze_source(
        "unsafe extern \"C\" verb update(ins value: Int); verb main() { ins value = 0; update(value: value); }",
    )
    .expect_err("external ABI calls must not infer ownership roles");
    assert!(matches!(external.kind, SemanticErrorKind::InvalidArgumentRole { .. }));

    let aggregate = analyze_source(
        "struct Packet { value: Int, } verb inspect(abs value: Packet) { } verb main() { erg value = Packet { value: 1, }; inspect(value: value); }",
    )
    .expect_err("aggregate inference must remain explicit");
    assert!(matches!(aggregate.kind, SemanticErrorKind::InvalidArgumentRole { .. }));

    let dat = analyze_source(
        "verb inspect(abs value: Int) { } verb main(dat value: Int) { inspect(value: value); }",
    )
    .expect_err("dat bindings must not be inferred as abs");
    assert!(matches!(dat.kind, SemanticErrorKind::InvalidArgumentRole { .. }));
}

#[test]
fn preserves_inferred_roles_across_branch_loop_and_cleanup_boundaries() {
    let model = analyze_source(
        "verb observe(abs value: Int) { } verb main() { erg source = 1; if true { { abs view = ref source; observe(value: view); } } loop { break; } }",
    )
    .expect("inferred abs role should survive branch and loop cleanup planning");
    assert!(model.argument_roles.iter().any(|fact| fact.source == ArgumentRoleSource::Inferred));
}

#[test]
fn records_explicit_roles_without_rewriting_them() {
    let model = analyze_source(
        "verb read(abs value: Int) -> Int { return 0; } verb main() -> Int { erg source = 1; abs value = ref source; return read(value: abs value); }",
    )
    .expect("explicit abs role should remain valid");
    let fact = model.argument_roles.last().expect("call role fact");
    assert_eq!(fact.source, ArgumentRoleSource::Explicit);
}
