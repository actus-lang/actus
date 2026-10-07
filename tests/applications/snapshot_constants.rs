use super::support::{build, project, run};
use std::fs;

#[test]
fn nested_snapshot_module_can_lower_facade_constants_in_struct_and_predicate() {
    let source = "import aie; verb main() -> Int { return snapshot_is_valid(); }\n";
    let (root, input, output) = project("config-snapshot-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/aie/persistence")).expect("create nested module");
    fs::write(root.join("src/aie/aie.act"), "open persistence;\n").expect("write aie facade");
    fs::write(root.join("src/aie/persistence/persistence.act"), "open serialization;\n")
        .expect("write persistence facade");
    fs::write(
        root.join("src/aie/persistence/serialization.act"),
        r#"import config;

open struct Snapshot {
    erg magic: u32,
}

open verb snapshot_is_valid() -> Int {
    erg snapshot = Snapshot {
        magic: BRAIN_MAGIC,
    };
    return case snapshot.magic == BRAIN_MAGIC {
        true => 42,
        _ => snapshot.magic as Int,
    };
}
"#,
    )
    .expect("write serialization implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove snapshot configuration project");
}

#[test]
fn nested_facade_constants_lower_inside_indexed_snapshot_loops() {
    let source = "import feature; verb main() -> Int { return emit(); }\n";
    let (root, input, output) = project("constant-indexed-loop", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        "open const STRIDE: u16 = 24u16; open const COUNT: u32 = 4u32;\n",
    )
    .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

open verb emit() -> Int {
    erg bytes: Array[u8, 24] = Array[u8, 24]();
    erg index: u32 = 0u32;
    loop {
        if index >= COUNT {
            break;
        }
        bytes[index * (STRIDE as u32) / (STRIDE as u32)] = index as u8;
        index += 1u32;
    }
    return bytes[3u32] as Int + STRIDE as Int;
}
"#,
    )
    .expect("write feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(27));
    fs::remove_dir_all(root).expect("remove indexed constant project");
}
