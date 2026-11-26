use std::fs;
use std::path::Path;

fn collect_sources(directory: &Path, sources: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory should be readable") {
        let path = entry.expect("directory entry should be readable").path();
        if path.is_dir() {
            collect_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

/// Whether `line` holds a string literal that starts like a diagnostic code.
fn spells_a_code(line: &str) -> bool {
    line.match_indices("\"E").any(|(index, _)| {
        let digits = &line.as_bytes()[index + 2..];
        digits.len() >= 4 && digits[..4].iter().all(u8::is_ascii_digit)
    }) || line.contains("\"E000")
}

#[test]
fn no_diagnostic_code_is_spelled_outside_the_registry() {
    let mut sources = Vec::new();
    collect_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut sources);
    let mut offenders = Vec::new();
    for path in sources {
        if path.ends_with("diagnostics/code/registry.rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("source file should be readable");
        for (number, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") && spells_a_code(line) {
                offenders.push(format!("{}:{}: {}", path.display(), number + 1, line.trim()));
            }
        }
    }
    assert!(offenders.is_empty(), "code literals outside the registry:\n{}", offenders.join("\n"));
}
