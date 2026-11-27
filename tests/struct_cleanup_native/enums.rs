//! Enum owners are released by their active variant: the payload of that
//! variant only, nothing for a variant without a resource, and nothing for an
//! enum that was moved away.

use super::harness::check;

#[test]
fn a_payload_of_an_option_is_released_at_scope_end() {
    check(
        "option-buffer",
        "erg src: Buffer = Buffer[1]; erg held: Option[Buffer] = Option[Buffer].Some(src); print(9);",
        "9\n",
    );
}

#[test]
fn a_payload_with_a_drop_role_is_cleaned_up_at_scope_end() {
    check(
        "option-tracker",
        "erg t = Tracker { id: 4, }; erg held: Option[Tracker] = Option[Tracker].Some(t); print(9);",
        "9\n4\n",
    );
}

#[test]
fn a_user_enum_is_cleaned_up_by_its_active_variant() {
    check(
        "user-enum",
        "erg src: Buffer = Buffer[1]; erg a: Msg = Msg.Data(src); erg b: Msg = Msg.Empty; print(9);",
        "9\n",
    );
}

#[test]
fn an_option_releases_the_payload_of_some_and_nothing_for_none() {
    check(
        "option-some-none",
        "erg src: Buffer = Buffer[1]; erg some: Option[Buffer] = Option[Buffer].Some(src); \
erg none: Option[Buffer] = Option[Buffer].None; print(9);",
        "9\n",
    );
}

#[test]
fn a_user_enum_runs_the_drop_of_its_active_payload() {
    check(
        "slot-held",
        "erg free: Slot = Slot.Free; erg held: Slot = Slot.Held(Mark.On); print(9);",
        "9\n4\n",
    );
}

#[test]
fn an_enum_dropped_explicitly_releases_its_payload_there() {
    check("slot-dropped", "erg held: Slot = Slot.Held(Mark.On); drop(held); print(9);", "4\n9\n");
}

#[test]
fn an_enum_moved_to_a_callee_is_released_by_the_callee_only() {
    check(
        "slot-moved",
        "erg held: Slot = Slot.Held(Mark.On); keep_slot(s: dat held); print(9);",
        "4\n9\n",
    );
}

#[test]
fn a_wildcard_arm_of_case_dat_releases_the_subject_by_its_variant() {
    check(
        "slot-wildcard",
        "erg held: Slot = Slot.Held(Mark.On); \
erg k: Int = case dat held { Slot.Free => 1, _ => 2, }; print(k);",
        "4\n2\n",
    );
}
