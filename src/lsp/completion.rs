use serde_json::json;

pub(super) fn primitive_items() -> serde_json::Value {
    let mut labels = vec!["f32".to_owned(), "f64".to_owned(), "Void".to_owned()];
    labels.extend((1..=128).map(|width| format!("u{width}")));
    labels.extend((1..=128).map(|width| format!("i{width}")));
    serde_json::Value::Array(
        labels.into_iter().map(|label| json!({"label": label, "kind": 25})).collect(),
    )
}
