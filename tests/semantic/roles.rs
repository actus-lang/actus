use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, SemanticModel, analyze};

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
        "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return 1; } } verb main() -> Int { erg file = File { value: 1, }; return file.write(); }",
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
