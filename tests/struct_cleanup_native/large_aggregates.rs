//! A large aggregate lives in a heap block of its own (ADR-0125, bug #127).
//! When its bytes are copied into the storage of another owner, because it
//! moved into a struct literal field, a field or an array element, its block
//! is freed after the copy; the destination owns its parts and releases its
//! own block at its exit.

use super::harness::run_on_every_backend;

/// A `Fabric` is 64 KiB and a `Brain` 128 KiB, both above the size from which
/// an aggregate is held in a heap block of its own.
const FABRIC: &str = "struct Cell { erg storage: Array[u8, 64], } \
struct Fabric { erg columns: Array[Cell, 1024], } struct Brain { erg fabrics: Array[Fabric, 2], } ";

fn check_large(label: &str, body: &str) {
    let source = format!("{FABRIC}verb main() -> Int {{ {body} return 42; }}");
    assert_eq!(run_on_every_backend(label, &source, "", 42), super::harness::BACKENDS_BUILT);
}

#[test]
fn copying_an_element_out_and_back_frees_every_block() {
    check_large(
        "large-self-copy",
        "erg brain: Brain = Brain { fabrics: Array[Fabric, 2](), }; \
erg local_fabric: Fabric = brain.fabrics[0u32]; brain.fabrics[0u32] = local_fabric;",
    );
}

#[test]
fn a_large_value_moved_into_a_literal_field_frees_its_block() {
    check_large("large-literal", "erg brain: Brain = Brain { fabrics: Array[Fabric, 2](), };");
}

#[test]
fn a_large_value_stored_into_a_field_or_an_element_frees_its_block() {
    check_large(
        "large-element",
        "erg brain: Brain = Brain { fabrics: Array[Fabric, 2](), }; \
erg fabric: Fabric = Fabric { columns: Array[Cell, 1024](), }; brain.fabrics[1u32] = fabric;",
    );
    check_large(
        "large-field",
        "erg fabric: Fabric = Fabric { columns: Array[Cell, 1024](), }; \
erg columns: Array[Cell, 1024] = Array[Cell, 1024](); fabric.columns = columns;",
    );
}
