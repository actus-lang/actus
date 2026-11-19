//! Shared lowering of control-flow joins and literal operand types.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, source: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-control-flow-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create control flow fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"control-flow-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write control flow manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write control flow lockfile");
    fs::write(root.join("src/main.act"), source).expect("write control flow source");
    root
}

fn actus(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("run actus")
}

/// Checks, builds and runs `source`; returns the exit code.
fn run(name: &str, source: &str) -> Option<i32> {
    let root = project(name, source);
    let check = actus(&root, &["check", "--strict", "src/main.act"]);
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build =
        actus(&root, &["build", "--strict", "--emit", "exe", "-o", "main.bin", "src/main.act"]);
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
    let code = Command::new(root.join("main.bin")).output().expect("run program").status.code();
    fs::remove_dir_all(root).expect("remove control flow fixture");
    code
}

/// A loop inside a branch inside a loop: the outer binding assigned in the
/// branch keeps its own value after the inner loop, which only carries its
/// own state. The inner `if` joins carried `count` as a block parameter that
/// does not dominate the inner loop exit.
#[test]
fn inner_loop_in_a_branch_keeps_outer_bindings() {
    let source = "verb main() -> Int {\n    erg marks: Array[u8, 10] = Array[u8, 10]();\n    erg count = 0;\n    erg outer = 0;\n    loop {\n        if outer >= 3 {\n            break;\n        }\n        if outer < 9 {\n            count = count + 1;\n            erg inner = 5;\n            loop {\n                if inner >= 7 {\n                    break;\n                }\n                marks[inner] = 1;\n                inner = inner + 1;\n            }\n        }\n        outer = outer + 1;\n    }\n    return count;\n}\n";
    assert_eq!(run("inner-loop", source), Some(3));
}

/// The sieve of Eratosthenes below 100 counts 25 primes.
#[test]
fn sieve_with_an_inner_loop_in_a_branch_counts_primes() {
    let source = "verb main() -> Int {\n    erg composite: Array[u8, 100] = Array[u8, 100]();\n    erg count = 0;\n    erg candidate = 2;\n    loop {\n        if candidate >= 100 {\n            break;\n        }\n        if composite[candidate] == 0 {\n            count = count + 1;\n            erg multiple = candidate * candidate;\n            loop {\n                if multiple >= 100 {\n                    break;\n                }\n                composite[multiple] = 1;\n                multiple = multiple + candidate;\n            }\n        }\n        candidate = candidate + 1;\n    }\n    return count;\n}\n";
    assert_eq!(run("sieve", source), Some(25));
}

/// Unsuffixed literals assigned in both branches of an `if` take the `u64`
/// type of the binding, so the join merges values of one type.
#[test]
fn branch_literals_take_the_type_of_the_assigned_binding() {
    let source = "verb main() -> Int {\n    erg outer: u64 = 0;\n    erg total: u64 = 0;\n    erg x: u64 = 0;\n    loop {\n        if outer >= 3 {\n            break;\n        }\n        if outer == 1 {\n            x = 5;\n        } else {\n            x = 2;\n        }\n        total = total + x;\n        outer = outer + 1;\n    }\n    return total as Int;\n}\n";
    assert_eq!(run("branch-literals", source), Some(9));
}

/// A literal divisor and remainder take the `u64` type of the dividend.
#[test]
fn literal_divisors_take_the_type_of_the_dividend() {
    let source = "verb steps(erg start: u64) -> u64 {\n    erg value = start;\n    erg count: u64 = 0;\n    loop {\n        if value == 1 {\n            break;\n        }\n        if value % 2 == 0 {\n            value = value / 2;\n        } else {\n            value = value * 3 + 1;\n        }\n        count = count + 1;\n    }\n    return count;\n}\n\nverb main() -> Int {\n    erg start: u64 = 27;\n    return steps(start: erg start) as Int;\n}\n";
    assert_eq!(run("divisors", source), Some(111));
}

/// Only a signed division overflows: an unsigned divisor with every bit set
/// divides normally instead of trapping.
#[test]
fn unsigned_division_by_all_ones_does_not_trap() {
    let source = "verb main() -> Int {\n    erg dividend: u64 = 9223372036854775808;\n    erg divisor: u64 = 18446744073709551615;\n    erg quotient = dividend / divisor;\n    return (quotient as Int) + 5;\n}\n";
    assert_eq!(run("unsigned-division", source), Some(5));
}

/// An unsuffixed literal above `i64::MAX` initializes a `u64` binding with its
/// full value instead of a truncated one.
#[test]
fn large_unsigned_literals_keep_their_value() {
    let source = "verb main() -> Int {\n    erg dividend: u64 = 9223372036854775808;\n    erg divisor: u64 = 3;\n    erg quotient = dividend / divisor;\n    return (quotient % 7) as Int;\n}\n";
    assert_eq!(run("large-literal", source), Some(2));
}

/// A struct built in every iteration of a long loop reuses one stack slot.
/// Each temporary is a static slot of the verb's frame (ADR-0114); a slot
/// allocated where the value is built would grow the stack on every
/// iteration and overflow it long before the loop ends.
#[test]
fn temporaries_built_in_a_loop_reuse_one_stack_slot() {
    let source = "struct Point {\n    x: Int,\n    y: Int,\n}\n\nverb sum(abs self: Point) -> Int {\n    return self.x + self.y;\n}\n\nverb main() -> Int {\n    erg total = 0;\n    erg i = 0;\n    loop {\n        if i == 50000000 {\n            break;\n        }\n        erg p = Point { x: i % 7, y: 1 };\n        abs view = ref p;\n        total = total + view.sum();\n        i = i + 1;\n    }\n    return total % 100;\n}\n";
    assert_eq!(run("loop-temporaries", source), Some(97));
}
