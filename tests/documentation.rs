use std::fs;
use std::path::{Path, PathBuf};

use actus::documentation::{DocumentationSection, validate_public_documentation};
use actus::lexer::scan;
use actus::parser::parse;

fn parse_source(source: &str) -> actus::ast::Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("documentation fixture should parse")
}

fn standard_library_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_actus_files(Path::new("library/std/src"), &mut files);
    files.sort();
    files
}

fn collect_actus_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("standard-library directory should exist") {
        let path = entry.expect("directory entry should be readable").path();
        if path.is_dir() {
            collect_actus_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "act") {
            files.push(path);
        }
    }
}

#[test]
fn standard_library_public_documentation_is_complete() {
    let mut failures = Vec::new();
    for path in standard_library_sources() {
        let source = fs::read_to_string(&path).expect("standard-library source should be readable");
        let (tokens, errors) = scan(&source);
        if !errors.is_empty() {
            failures.push(format!("{}: lexical errors: {errors:?}", path.display()));
            continue;
        }
        let program = match parse(tokens) {
            Ok(program) => program,
            Err(error) => {
                failures.push(format!("{}: parse error: {error:?}", path.display()));
                continue;
            }
        };
        for issue in validate_public_documentation(&program) {
            failures.push(format!(
                "{}:{}-{} {} {}: {}",
                path.display(),
                issue.span.start,
                issue.span.end,
                issue.code,
                issue.declaration,
                issue.message
            ));
        }
    }
    assert!(failures.is_empty(), "documentation violations:\n{}", failures.join("\n"));
}

#[test]
fn accepts_complete_contracts_for_each_public_declaration_category() {
    let program = parse_source(
        r#"
        """A storage type owns bytes and releases them during cleanup."""
        open struct Packet {
            """Owned erg payload storage; the field remains with Packet and is dropped with it."""
            erg payload: Buffer,
        }
        """A status type reports successful completion and typed failure variants."""
        open enum Status {
            """Success reports a completed operation and carries no owned storage."""
            Done,
            """Failure reports an invalid operation and carries an error payload."""
            Failed,
        }
        """A role defines a borrowed stream operation and its result contract."""
        open role Reader {
            """Read through ins exclusive loans, fill caller buffer storage, and return a count or typed error."""
            verb read(ins self: Self, ins buffer: Buffer) -> Result[Int, IoError];
        }
        """A performance binds the reader behavior to an owned packet resource."""
        open perform Reader for Packet {
            """Read through ins loans, mutate caller storage, and return count or error."""
            verb read(ins self: Packet, ins buffer: Buffer) -> Result[Int, IoError] { return 0; }
        }
        """Read a borrowed raw path through the native C ABI runtime bridge without allocation; return a result or failure."""
        open unsafe extern "C" verb read_path(abs path: Path) -> Result[Int, IoError];
        """Consume owned dat Buffer bytes, use their allocation, and return a result or typed error after cleanup."""
        open verb consume(dat bytes: Buffer) -> Result[Int, IoError] { return 0; }
        """Re-export the public stream implementation and its documented contract."""
        open stream;
        "#,
    );
    let issues = validate_public_documentation(&program);
    assert!(issues.is_empty(), "accepted fixture issues: {issues:?}");
}

#[test]
fn rejects_missing_and_incomplete_contract_sections_at_declaration_span() {
    let program = parse_source(
        r#"
        open verb missing(abs input: Buffer) -> Result[Int, IoError] { return 0; }
        """Short summary.""" open verb short(erg input: Int) -> Int { return input; }
        """Return a value but omit ownership details.""" open verb partial(abs input: Buffer) -> Result[Int, IoError] { return 0; }
        "#,
    );
    let issues = validate_public_documentation(&program);
    assert!(issues.iter().any(|issue| issue.code == "E1840" && issue.span.start > 0));
    assert!(issues.iter().any(|issue| issue.code == "E1842"));
    assert!(issues.iter().any(|issue| issue.section == DocumentationSection::Ownership));
    assert!(issues.iter().any(|issue| issue.section == DocumentationSection::Errors));
}

#[test]
fn rejects_contradictory_and_misrepresenting_contracts() {
    let program = parse_source(
        r#"
        """This borrowed abs input takes ownership and always succeeds without errors."""
        open verb contradictory(abs input: Buffer) -> Result[Int, IoError] { return 0; }
        "#,
    );
    let issues = validate_public_documentation(&program);
    assert!(issues.iter().any(|issue| issue.code == "E1844"));
    assert!(issues.iter().any(|issue| issue.code == "E1845"));
}
