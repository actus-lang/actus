use crate::lexer::SourceSpan;

use super::data::{EnumDef, EnumPayload, EnumVariant};
use super::types::{GenericParam, TypeName};

pub fn builtin_enum_definitions() -> Vec<EnumDef> {
    vec![option_definition(), result_definition()]
}

fn option_definition() -> EnumDef {
    EnumDef {
        is_open: false,
        doc: None,
        name: "Option".to_owned(),
        generic_parameters: vec![generic_parameter("T")],
        variants: vec![
            EnumVariant {
                doc: None,
                name: "Some".to_owned(),
                payload: EnumPayload::Tuple(vec![type_name("T")]),
                span: zero_span(),
            },
            EnumVariant {
                doc: None,
                name: "None".to_owned(),
                payload: EnumPayload::Unit,
                span: zero_span(),
            },
        ],
        span: zero_span(),
    }
}

fn result_definition() -> EnumDef {
    EnumDef {
        is_open: false,
        doc: None,
        name: "Result".to_owned(),
        generic_parameters: vec![generic_parameter("T"), generic_parameter("E")],
        variants: vec![
            EnumVariant {
                doc: None,
                name: "Ok".to_owned(),
                payload: EnumPayload::Tuple(vec![type_name("T")]),
                span: zero_span(),
            },
            EnumVariant {
                doc: None,
                name: "Err".to_owned(),
                payload: EnumPayload::Tuple(vec![type_name("E")]),
                span: zero_span(),
            },
        ],
        span: zero_span(),
    }
}

fn generic_parameter(name: &str) -> GenericParam {
    GenericParam { name: name.to_owned(), bound: None, bounds: Vec::new(), span: zero_span() }
}

fn type_name(name: &str) -> TypeName {
    TypeName {
        name: name.to_owned(),
        arguments: Vec::new(),
        reference_role: None,
        span: zero_span(),
    }
}

fn zero_span() -> SourceSpan {
    SourceSpan::new(0, 0)
}
