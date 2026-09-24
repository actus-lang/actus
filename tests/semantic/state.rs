use actus::semantic::{AccessState, OwnershipState, ResourceState};

#[test]
fn active_owner_has_one_live_ownership_path() {
    let state = OwnershipState::Active;
    assert!(state.is_live());
    assert!(state.can_be_read());
}

#[test]
fn move_invalidates_the_source() {
    let state = OwnershipState::Moved;
    assert!(!state.is_live());
    assert!(!state.can_be_read());
}

#[test]
fn partial_move_remains_explicitly_live() {
    let state = OwnershipState::PartiallyMoved { fields: vec!["payload".to_owned()] };
    assert!(state.is_live());
    assert!(state.can_be_cleaned());
}

#[test]
fn dropped_owner_cannot_be_cleaned_twice() {
    assert!(!OwnershipState::Dropped.can_be_cleaned());
}

#[test]
fn frozen_access_is_separate_from_ownership() {
    let access = AccessState::Frozen { borrow_ids: vec![0] };
    assert!(access.is_frozen());
    assert!(OwnershipState::Active.is_live());
}

#[test]
fn mutable_access_is_restorable_without_consuming_owner() {
    let access = AccessState::Mutable;
    assert!(!access.is_frozen());
    assert!(OwnershipState::Active.can_be_read());
}

#[test]
fn moved_owner_cannot_become_mutable_by_access_change() {
    let access = AccessState::Mutable;
    assert!(!access.is_frozen());
    assert!(!OwnershipState::Moved.is_live());
}

#[test]
fn cleanup_requires_a_live_ownership_state() {
    assert!(OwnershipState::Active.can_be_cleaned());
    assert!(OwnershipState::PartiallyMoved { fields: Vec::new() }.can_be_cleaned());
    assert!(!OwnershipState::Moved.can_be_cleaned());
    assert!(!OwnershipState::Dropped.can_be_cleaned());
}

#[test]
fn transition_matrix_preserves_owner_during_temporary_abs_access() {
    let mut state = ResourceState::active();
    assert!(state.freeze(0));
    assert_eq!(state.ownership, OwnershipState::Active);
    assert!(!state.move_owner());
    assert!(state.thaw());
    assert_eq!(state.access, AccessState::Mutable);
}

#[test]
fn transition_matrix_allows_exactly_one_dat_destination() {
    let mut state = ResourceState::active();
    assert!(state.move_owner());
    assert!(!state.move_owner());
    assert!(!state.thaw());
}

#[test]
fn transition_matrix_tracks_partial_move_and_remaining_cleanup() {
    let mut state = ResourceState::active();
    assert!(state.partial_move("payload"));
    assert_eq!(
        state.ownership,
        OwnershipState::PartiallyMoved { fields: vec!["payload".to_owned()] }
    );
    assert!(state.drop_owner());
    assert!(!state.drop_owner());
}

#[test]
fn transition_matrix_rejects_failed_cleanup_and_ambiguous_access() {
    let mut state = ResourceState::active();
    assert!(state.freeze(1));
    assert!(!state.drop_owner());
    assert!(!state.partial_move("field"));
    assert!(state.thaw());
    assert!(state.drop_owner());
    assert!(!state.thaw());
}
