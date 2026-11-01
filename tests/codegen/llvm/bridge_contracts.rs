use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use actus::cli::run_with_args;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExternalContract {
    arity: usize,
    aggregate_return: bool,
}

fn collect_external_arities(
    directory: &Path,
    extension: &str,
    marker: &str,
    declarations: bool,
    signatures: &mut BTreeMap<String, ExternalContract>,
) {
    for entry in fs::read_dir(directory).expect("ABI directory should be readable") {
        let path = entry.expect("ABI entry should be readable").path();
        if path.is_dir() {
            collect_external_arities(&path, extension, marker, declarations, signatures);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        let source = fs::read_to_string(path).expect("ABI source should be readable");
        let mut remaining = source.as_str();
        while let Some(offset) = remaining.find(marker) {
            let declaration = &remaining[offset + marker.len()..];
            let name_end = declaration
                .find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .expect("ABI declaration should have a name boundary");
            let name = declaration[..name_end].to_owned();
            let open = declaration[name_end..]
                .find('(')
                .map(|index| index + name_end)
                .expect("ABI declaration should have parameters");
            let mut depth = 0usize;
            let mut close = None;
            for (index, character) in declaration[open..].char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + index);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.expect("ABI declaration should close its parameters");
            let parameter_text = declaration[open + 1..close].trim();
            let parameter_text = parameter_text.strip_suffix(',').unwrap_or(parameter_text);
            let mut nested = 0usize;
            let arity = parameter_text
                .chars()
                .filter(|character| match character {
                    '[' | '<' | '{' => {
                        nested += 1;
                        false
                    }
                    ']' | '>' | '}' => {
                        nested -= 1;
                        false
                    }
                    ',' => nested == 0,
                    _ => false,
                })
                .count()
                + usize::from(!parameter_text.is_empty());
            let aggregate_return = if declarations {
                let return_tail = &declaration[close + 1..];
                let return_type = return_tail
                    .split(';')
                    .next()
                    .and_then(|value| value.split_once("->"))
                    .map(|(_, value)| value.trim())
                    .unwrap_or_default();
                !matches!(
                    return_type,
                    "Bool"
                        | "Int"
                        | "String"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                )
            } else {
                false
            };
            let contract = ExternalContract { arity, aggregate_return };
            if let Some(previous) = signatures.insert(name.clone(), contract) {
                assert_eq!(previous, contract, "inconsistent ABI contract for {name}");
            }
            remaining = &declaration[close + 1..];
        }
    }
}

fn assert_no_missing_runtime_bridges(
    expected: &BTreeMap<String, ExternalContract>,
    runtime: &BTreeMap<String, ExternalContract>,
) {
    let missing = expected.keys().filter(|name| !runtime.contains_key(*name)).collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "standard-library declarations without runtime providers: {missing:?}"
    );
    let mismatched = expected
        .iter()
        .filter_map(|(name, arity)| {
            runtime
                .get(name)
                .filter(|runtime_contract| {
                    runtime_contract.arity != arity.arity
                        && !(arity.aggregate_return && runtime_contract.arity == arity.arity + 1)
                })
                .map(|runtime_contract| (name, arity, runtime_contract))
        })
        .collect::<Vec<_>>();
    assert!(mismatched.is_empty(), "runtime bridge arity mismatches: {mismatched:?}");
}

#[test]
fn all_hosted_standard_library_facades_and_runtime_bridge_contracts_pass_llvm() {
    let root =
        std::env::temp_dir().join(format!("actus-llvm-stdlib-bridge-audit-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("LLVM bridge audit source directory should exist");
    let source = root.join("src/main.act");
    let output = root.join(format!("bridge-audit{}", std::env::consts::EXE_SUFFIX));
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm_stdlib_bridge_audit\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nbackend = \"llvm\"\nverify_no_float_ir = true\n",
    )
    .expect("LLVM bridge audit manifest should be written");
    fs::write(
        &source,
        "import { open std::io, open std::fs, open std::path, open std::process, open std::region, open std::wire, open std::charmap, open std::string, open std::time }; verb main() -> Int { return 0; }\n",
    )
    .expect("LLVM bridge audit source should be written");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source.display().to_string(),
            "--strict".to_owned(),
            "--backend".to_owned(),
            "llvm".to_owned(),
            "--emit".to_owned(),
            "obj".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0, "all hosted standard-library bridges must lower through LLVM");
    assert!(output.exists(), "LLVM bridge audit object should be emitted");

    let mut expected = BTreeMap::new();
    collect_external_arities(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src"),
        "act",
        "unsafe extern \"C\" verb ",
        true,
        &mut expected,
    );
    let mut runtime = BTreeMap::new();
    collect_external_arities(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/runtime"),
        "rs",
        "extern \"C\" fn ",
        false,
        &mut runtime,
    );
    assert_no_missing_runtime_bridges(&expected, &runtime);
    let _ = fs::remove_dir_all(root);
}
