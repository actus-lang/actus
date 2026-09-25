use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::configuration::{ArcaLock, CompilerConfiguration};
use actus::lexer::scan;
use actus::modules::{ModuleResolver, resolve_imports};
use actus::parser::parse;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-dependencies-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        fs::write(path, source).expect("write fixture");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn dependency_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "app/Arca.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = { path = \"../foo\" }\n",
    );
    fixture.write(
        "app/src/main.act",
        "import foo; verb main() -> Int { return add(left: 40, right: 2); }\n",
    );
    fixture.write("foo/Arca.toml", "[package]\nname = \"foo\"\nversion = \"1.2.0\"\n");
    fixture.write("foo/src/foo/foo.act", "open ops;");
    fixture.write(
        "foo/src/foo/ops.act",
        "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }",
    );
    fixture
}

#[test]
fn resolves_a_local_package_namespace_from_arca_manifest() {
    let fixture = dependency_fixture();
    let manifest = fixture.root.join("app/Arca.toml");
    let configuration = CompilerConfiguration::from_manifest(&manifest).expect("load manifest");
    let source = fs::read_to_string(fixture.root.join("app/src/main.act")).expect("read source");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("parse source");
    let expanded = resolve_imports(
        &program,
        &ModuleResolver::with_dependencies(
            configuration.source_root(),
            configuration.dependency_roots(),
        ),
    )
    .expect("resolve local package import");
    assert!(expanded.declarations.iter().any(|declaration| {
        matches!(declaration, actus::ast::TopLevelDecl::Verb(verb) if verb.name == "add")
    }));
}

#[test]
fn records_local_dependency_path_and_content_checksum_in_lockfile() {
    let fixture = dependency_fixture();
    let manifest = fixture.root.join("app/Arca.toml");
    let lock = ArcaLock::generate_from_manifest(&manifest).expect("generate lockfile");
    let package =
        lock.packages.iter().find(|package| package.name == "foo").expect("foo lock entry");
    assert_eq!(package.source.as_deref(), Some("path"));
    assert_eq!(package.path.as_deref(), Some("../foo"));
    assert!(package.checksum.as_deref().is_some_and(|checksum| checksum.len() == 16));
}

#[test]
fn rejects_a_missing_local_dependency_manifest() {
    let fixture = Fixture::new();
    fixture.write(
        "app/Arca.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmissing = { path = \"../missing\" }\n",
    );
    let error = CompilerConfiguration::from_manifest(&fixture.root.join("app/Arca.toml"))
        .expect_err("missing dependency must fail");
    assert!(error.to_string().contains("InvalidDependencyPath"));
}

#[test]
fn enforces_a_local_dependency_version_constraint() {
    let fixture = dependency_fixture();
    fixture.write(
        "app/Arca.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = { path = \"../foo\", version = \"^1.0.0\" }\n",
    );
    CompilerConfiguration::from_manifest(&fixture.root.join("app/Arca.toml"))
        .expect("compatible local dependency should resolve");
    fixture.write(
        "app/Arca.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = { path = \"../foo\", version = \"^2.0.0\" }\n",
    );
    let error = CompilerConfiguration::from_manifest(&fixture.root.join("app/Arca.toml"))
        .expect_err("incompatible local dependency must fail");
    assert!(error.to_string().contains("VersionConflict"));
}

#[test]
fn rejects_conflicting_dependency_declarations() {
    let fixture = dependency_fixture();
    fixture.write(
        "app/Arca.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[package.dependencies]\nfoo = \"1.0.0\"\n\n[dependencies]\nfoo = \"2.0.0\"\n",
    );
    let error = CompilerConfiguration::from_manifest(&fixture.root.join("app/Arca.toml"))
        .expect_err("conflicting declarations must fail");
    assert!(error.to_string().contains("VersionConflict"));
}
