use serde_json::json;

use crate::ast::TopLevelDecl;
use crate::lexer::scan;
use crate::parser::parse;

pub(super) fn items(source: Option<&str>) -> serde_json::Value {
    let mut labels = vec![
        "pack".to_owned(),
        "layout".to_owned(),
        "fields".to_owned(),
        "at".to_owned(),
        "little".to_owned(),
        "big".to_owned(),
        "f32".to_owned(),
        "f64".to_owned(),
        "Void".to_owned(),
        "Int".to_owned(),
        "Bool".to_owned(),
        "Char".to_owned(),
        "String".to_owned(),
        "Buffer".to_owned(),
        "Array".to_owned(),
        "Arena".to_owned(),
        "Option".to_owned(),
        "Result".to_owned(),
        "Map".to_owned(),
        "Usize".to_owned(),
        "as".to_owned(),
        "<".to_owned(),
        "<=".to_owned(),
        ">".to_owned(),
        ">=".to_owned(),
        "==".to_owned(),
        "!=".to_owned(),
        "%".to_owned(),
        "&&".to_owned(),
        "||".to_owned(),
        "&".to_owned(),
        "|".to_owned(),
        "^".to_owned(),
        "~".to_owned(),
        "<<".to_owned(),
        ">>".to_owned(),
    ];
    labels.extend((1..=128).map(|width| format!("u{width}")));
    labels.extend((1..=128).map(|width| format!("i{width}")));
    if let Some(source) = source
        && let Ok(program) = parse(scan(source).0)
    {
        for declaration in program.declarations {
            if let TopLevelDecl::Pack(pack) = declaration {
                labels.push(pack.name);
                labels.extend(pack.fields.into_iter().map(|field| field.name));
            }
        }
    }
    serde_json::Value::Array(
        labels.into_iter().map(|label| json!({"label": label, "kind": 25})).collect(),
    )
}
