use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{DynamicRoleType, FatPointerLayout, SemanticErrorKind, analyze};

fn analyze_source(
    source: &str,
) -> Result<actus::semantic::SemanticModel, actus::semantic::SemanticError> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program)
}

const WRITER_SOURCE: &str = "
    struct File { value: Int, }
    role Writer { verb write(abs self: File) -> Int; }
    perform Writer for File {
        verb write(abs self: File) -> Int { return self.value; }
    }
";

#[test]
fn accepts_dynamic_abs_parameter_for_a_performed_type() {
    let source = format!(
        "{WRITER_SOURCE}
         verb send(abs writer: dynamic Writer) -> Int {{ return 0; }}
         verb main() {{ erg file = File {{ value: 1, }}; send(writer: ref file); }}"
    );
    let model = analyze_source(&source).expect("performed type should satisfy dynamic role");

    assert_eq!(
        model.dynamic_roles,
        vec![DynamicRoleType {
            role_name: "Writer".to_owned(),
            layout: FatPointerLayout::DYNAMIC_ROLE,
        }]
    );
}

#[test]
fn rejects_dynamic_parameter_without_abs_role() {
    let error = analyze_source("verb send(erg writer: dynamic Writer) { }")
        .expect_err("dynamic roles must be borrowed");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::DynamicRequiresAbs { parameter } if parameter == "writer"
    ));
}

#[test]
fn rejects_unknown_dynamic_role() {
    let error = analyze_source("verb send(abs writer: dynamic Missing) { }")
        .expect_err("unknown dynamic roles must fail");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::UnknownDynamicRole { name } if name == "Missing"
    ));
}

#[test]
fn rejects_dynamic_argument_without_matching_performance() {
    let source = format!(
        "{WRITER_SOURCE}
         struct Other {{ value: Int, }}
         verb send(abs writer: dynamic Writer) -> Int {{ return 0; }}
         verb main() {{ erg other = Other {{ value: 1, }}; send(writer: ref other); }}"
    );
    let error = analyze_source(&source).expect_err("a dynamic role requires a performance");
    assert!(matches!(
        error.kind,
        SemanticErrorKind::DynamicRoleMismatch { role, found }
            if role == "Writer" && found == "Other"
    ));
}

#[test]
fn models_dynamic_roles_as_data_and_vtable_pointer_words() {
    let layout = FatPointerLayout::DYNAMIC_ROLE;
    assert_eq!(layout.data_ptr_word, 0);
    assert_eq!(layout.vtable_ptr_word, 1);
    assert_eq!(layout.word_count, 2);
    assert_eq!(layout.alignment_words, 1);
}
