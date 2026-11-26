use std::collections::BTreeMap;

use super::DiagnosticCode;

#[test]
fn registry_values_are_unique() {
    let mut owners: BTreeMap<&str, usize> = BTreeMap::new();
    for code in DiagnosticCode::ALL {
        *owners.entry(code.as_str()).or_default() += 1;
    }
    let shared: Vec<_> = owners.iter().filter(|(_, count)| **count > 1).collect();
    assert!(shared.is_empty(), "codes listed twice: {shared:?}");
}

#[test]
fn registry_values_have_the_e_and_four_digits_shape() {
    for code in DiagnosticCode::ALL {
        let text = code.as_str();
        let digits = text.strip_prefix('E').unwrap_or("");
        assert!(
            digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit()),
            "`{text}` is not E followed by four digits"
        );
    }
}

#[test]
fn registry_is_sorted_by_value() {
    let values: Vec<_> = DiagnosticCode::ALL.iter().map(DiagnosticCode::as_str).collect();
    let mut sorted = values.clone();
    sorted.sort_unstable();
    assert_eq!(values, sorted);
}
