//! A method call resolves to the verb of its receiver's own type. A performance
//! of the same method name for another type (`File` performing `seek`) must not
//! capture the call while that receiver has no performance of its own.

use std::fs;
use std::process::Command;

const MAIN: &str = "import { open cursor, open file };
verb main() -> Int {
    erg c: Cursor = Cursor { position: 1 };
    erg f: File = File { descriptor: 0 };
    erg offset = 2;
    erg moved = c.seek(offset: offset);
    erg origin = 4;
    erg other = f.seek(from: dat origin);
    return moved + other;
}
";
const CURSOR: &str = "open struct Cursor { position: Int, }
open verb seek(ins self: Cursor, erg offset: Int) -> Int { return self.position + offset; }
";
const FILE: &str = "open struct File { descriptor: Int, }
open role Seeker { verb seek(ins self: File, dat from: Int) -> Int; }
open perform Seeker for File { verb seek(ins self: File, dat from: Int) -> Int { return self.descriptor * 10 + from; } }
";

fn run(name: &str, main: &str, cursor: &str, file: &str) -> (Option<i32>, String) {
    let root =
        std::env::temp_dir().join(format!("actus-method-lookup-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/cursor")).expect("create fixture");
    fs::create_dir_all(root.join("src/file")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"method-lookup\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    fs::write(root.join("src/main.act"), main).expect("write main");
    fs::write(root.join("src/cursor/cursor.act"), "open api;\n").expect("write facade");
    fs::write(root.join("src/cursor/api.act"), cursor).expect("write cursor");
    fs::write(root.join("src/file/file.act"), "open api;\n").expect("write facade");
    fs::write(root.join("src/file/api.act"), file).expect("write file");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["run", "src/main.act"])
        .current_dir(&root)
        .output()
        .expect("run actus");
    (output.status.code(), String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn verb_of_one_type_is_not_captured_by_a_performance_of_another() {
    // 1 + 2 from the cursor verb, 0 * 10 + 4 from the file performance; a
    // captured call gives 1 * 10 + 2.
    let (code, stderr) = run("static", MAIN, CURSOR, FILE);
    assert_eq!(code, Some(7), "{stderr}");
}

#[test]
fn dynamic_role_call_still_reaches_its_performance_beside_a_same_named_verb() {
    let main = "import { open cursor, open file };
verb send(abs writer: dynamic Seeker) -> Int {
    erg origin = 4;
    return writer.seek(from: dat origin);
}
verb main() -> Int {
    erg f: File = File { descriptor: 1 };
    return send(writer: ref f);
}
";
    let file = "open struct File { descriptor: Int, }
open role Seeker { verb seek(abs self: File, dat from: Int) -> Int; }
open perform Seeker for File { verb seek(abs self: File, dat from: Int) -> Int { return self.descriptor * 10 + from; } }
";
    let (code, stderr) = run("dynamic", main, CURSOR, file);
    assert_eq!(code, Some(14), "{stderr}");
}
