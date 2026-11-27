//! Struct owners and `dat` parameters: released at the end of their scope,
//! by an explicit `drop`, and partially after a field was moved out; a
//! value's own `Drop` runs before its fields, which run in reverse order.

use super::harness::{CFG, DEEP, DUO, GUARDED, HOLDER, NEST, PAIR, check};

#[test]
fn a_dat_parameter_nothing_consumed_releases_its_buffers() {
    check("dat-buffers", &format!("{CFG} ignore_cfg(c: dat c); print(9);"), "9\n");
}

#[test]
fn a_dat_parameter_with_a_drop_contract_runs_it_and_releases_its_fields() {
    check("dat-contract", &format!("{GUARDED} ignore_guarded(g: dat g); print(9);"), "7\n9\n");
}

#[test]
fn a_dat_parameter_with_drop_role_fields_runs_their_drops_in_reverse_order() {
    check("dat-drop-fields", &format!("{PAIR} ignore_pair(p: dat p); print(9);"), "2\n1\n9\n");
}

#[test]
fn a_dat_parameter_with_nested_buffers_releases_all_of_them() {
    check("dat-nested", &format!("{NEST} ignore_nest(n: dat n); print(9);"), "9\n");
}

#[test]
fn a_local_struct_with_two_buffers_is_released_at_return() {
    check("local-buffers", CFG, "");
}

#[test]
fn a_local_struct_with_drop_role_fields_is_released_at_scope_end() {
    check("local-drop-fields", &format!("{PAIR} print(9);"), "9\n2\n1\n");
}

#[test]
fn a_local_struct_with_a_drop_contract_runs_it_then_releases_its_buffer() {
    check("local-contract", &format!("{GUARDED} print(9);"), "9\n7\n");
}

#[test]
fn dropping_a_dat_parameter_with_a_buffer_field_releases_the_buffer() {
    check("drop-dat-buffer", &format!("{HOLDER} drop_holder(o: dat h); print(9);"), "9\n");
}

#[test]
fn dropping_a_dat_parameter_with_drop_role_fields_runs_their_drops() {
    check("drop-dat-drop-fields", &format!("{PAIR} drop_pair(p: dat p); print(9);"), "2\n1\n9\n");
}

#[test]
fn dropping_a_dat_parameter_with_a_drop_contract_runs_it_and_releases_the_buffer() {
    check("drop-dat-contract", &format!("{GUARDED} drop_guarded(g: dat g); print(9);"), "7\n9\n");
}

#[test]
fn an_explicit_drop_of_a_local_with_drop_role_fields_runs_their_drops() {
    check("drop-local-drop-fields", &format!("{PAIR} drop(p); print(9);"), "2\n1\n9\n");
}

#[test]
fn an_explicit_drop_of_a_local_with_a_drop_contract_runs_it_and_releases_the_buffer() {
    check("drop-local-contract", &format!("{GUARDED} drop(g); print(9);"), "7\n9\n");
}

#[test]
fn an_explicit_drop_of_a_local_struct_releases_every_buffer() {
    check("drop-local-buffers", &format!("{CFG} drop(c); {HOLDER} drop(h); print(9);"), "9\n");
}

#[test]
fn a_nested_struct_releases_the_buffer_of_the_struct_it_holds() {
    check("nested-buffers", &format!("{NEST} print(9);"), "9\n");
}

#[test]
fn a_nested_struct_cleans_up_in_reverse_declaration_order_recursively() {
    check("nested-drop-fields", &format!("{DEEP} print(9);"), "9\n3\n2\n1\n");
}

#[test]
fn a_partially_moved_local_releases_only_the_fields_it_still_owns() {
    check("partial-local", &format!("{PAIR} sink_tracker(t: dat p.a); print(9);"), "1\n9\n2\n");
}

#[test]
fn a_partially_moved_owner_with_a_drop_contract_skips_its_own_drop() {
    check("partial-contract", &format!("{DUO} sink_tracker(t: dat u.b); print(9);"), "2\n9\n1\n");
}

#[test]
fn an_owner_runs_its_own_drop_before_its_fields_in_reverse_order() {
    check("drop-order", &format!("{DUO} {PAIR} print(9);"), "9\n2\n1\n0\n2\n1\n");
}
