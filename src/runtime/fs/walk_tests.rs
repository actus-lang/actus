use super::*;

fn test_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("actus-walk-{name}-{}", std::process::id()))
}

fn options(max_depth: u32, follow_symlinks: bool, error_policy: i32) -> WalkOptions {
    WalkOptions { max_depth, follow_symlinks, include_hidden: false, order: 1, error_policy }
}

#[cfg(unix)]
fn take_event(state: &mut WalkerState) -> PendingEvent {
    next_event(state).expect("walker should not fail").expect("event should exist")
}

#[test]
fn depth_limit_keeps_each_directory_frame_bounded() {
    let root = test_root("depth");
    std::fs::create_dir_all(root.join("nested")).expect("create nested directory");
    std::fs::write(root.join("alpha"), b"").expect("create alpha");
    std::fs::write(root.join("nested").join("inside"), b"").expect("create inside");
    std::fs::write(root.join("zeta"), b"").expect("create zeta");
    let mut state = prepare_root(&root, options(1, false, 0)).expect("open root");
    let mut kinds = Vec::new();
    while let Some(event) = next_event(&mut state).expect("walk should succeed") {
        kinds.push(event.kind);
    }
    assert_eq!(
        kinds,
        vec![
            WALK_KIND_ENTER_DIRECTORY,
            WALK_KIND_FILE,
            WALK_KIND_ENTER_DIRECTORY,
            WALK_KIND_LEAVE_DIRECTORY,
            WALK_KIND_FILE,
            WALK_KIND_LEAVE_DIRECTORY,
        ]
    );
    std::fs::remove_dir_all(root).expect("remove depth fixture");
}

#[test]
fn ordered_walker_rejects_a_directory_over_the_entry_bound() {
    let root = test_root("wide");
    std::fs::create_dir_all(&root).expect("create wide root");
    for index in 0..=WALK_MAX_ENTRIES_PER_DIRECTORY {
        std::fs::write(root.join(format!("entry-{index}")), b"").expect("create entry");
    }
    match prepare_root(&root, options(1, false, 0)) {
        Err(status) => assert_eq!(status, WALK_STATUS_BUFFER_TOO_SMALL),
        Ok(_) => panic!("wide directory should exceed the bounded entry limit"),
    }
    std::fs::remove_dir_all(root).expect("remove wide fixture");
}

#[cfg(unix)]
#[test]
fn followed_symlink_cycle_is_fail_fast() {
    use std::os::unix::fs::symlink;

    let root = test_root("cycle-fail");
    std::fs::create_dir_all(&root).expect("create cycle root");
    symlink(&root, root.join("loop")).expect("create cycle link");
    let mut state = prepare_root(&root, options(4, true, 0)).expect("open root");
    let entered = take_event(&mut state);
    assert_eq!(entered.kind, WALK_KIND_ENTER_DIRECTORY);
    match next_event(&mut state) {
        Err(status) => assert_eq!(status, ABI_STATUS_FAILURE),
        Ok(_) => panic!("cycle should fail fast"),
    }
    std::fs::remove_dir_all(root).expect("remove cycle fixture");
}

#[cfg(unix)]
#[test]
fn followed_symlink_cycle_can_be_emitted_and_skipped() {
    use std::os::unix::fs::symlink;

    let root = test_root("cycle-event");
    std::fs::create_dir_all(&root).expect("create cycle root");
    symlink(&root, root.join("loop")).expect("create cycle link");
    let mut state = prepare_root(&root, options(4, true, 1)).expect("open root");
    let entered = take_event(&mut state);
    assert_eq!(entered.kind, WALK_KIND_ENTER_DIRECTORY);
    let error = take_event(&mut state);
    assert_eq!(error.kind, WALK_KIND_ERROR);
    let leave = take_event(&mut state);
    assert_eq!(leave.kind, WALK_KIND_LEAVE_DIRECTORY);
    assert!(next_event(&mut state).expect("walk should finish").is_none());
    std::fs::remove_dir_all(root).expect("remove cycle fixture");
}

#[cfg(unix)]
#[test]
fn non_followed_symlink_is_reported_without_target_lookup() {
    use std::os::unix::fs::symlink;

    let root = test_root("symlink");
    std::fs::create_dir_all(&root).expect("create symlink root");
    symlink("missing-target", root.join("link")).expect("create link");
    let mut state = prepare_root(&root, options(1, false, 0)).expect("open root");
    assert_eq!(take_event(&mut state).kind, WALK_KIND_ENTER_DIRECTORY);
    assert_eq!(take_event(&mut state).kind, WALK_KIND_SYMLINK);
    assert_eq!(take_event(&mut state).kind, WALK_KIND_LEAVE_DIRECTORY);
    assert!(next_event(&mut state).expect("walk should finish").is_none());
    std::fs::remove_dir_all(root).expect("remove symlink fixture");
}

#[cfg(unix)]
#[test]
fn followed_broken_symlink_can_be_emitted_as_error() {
    use std::os::unix::fs::symlink;

    let root = test_root("broken-link");
    std::fs::create_dir_all(&root).expect("create broken-link root");
    symlink("missing-target", root.join("broken")).expect("create broken link");
    let mut state = prepare_root(&root, options(1, true, 1)).expect("open root");
    assert_eq!(take_event(&mut state).kind, WALK_KIND_ENTER_DIRECTORY);
    assert_eq!(take_event(&mut state).kind, WALK_KIND_ERROR);
    assert_eq!(take_event(&mut state).kind, WALK_KIND_LEAVE_DIRECTORY);
    assert!(next_event(&mut state).expect("walk should finish").is_none());
    std::fs::remove_dir_all(root).expect("remove broken-link fixture");
}

#[test]
fn permission_denied_is_preserved_in_error_events() {
    let state = WalkerState {
        options: options(1, false, 1),
        frames: Vec::new(),
        visited: Vec::new(),
        pending: None,
    };
    let native_error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let event = report_io_error(&state, PathBuf::from("protected"), 1, &native_error)
        .expect("emit-error policy should continue")
        .expect("permission failure should produce an event");
    assert_eq!(event.kind, WALK_KIND_ERROR);
    assert_eq!(event.error, WALK_ERROR_PERMISSION_DENIED);
}

#[test]
fn permission_denied_fail_fast_does_not_become_eof() {
    let state = WalkerState {
        options: options(1, false, 0),
        frames: Vec::new(),
        visited: Vec::new(),
        pending: None,
    };
    let native_error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let result = report_io_error(&state, PathBuf::from("protected"), 1, &native_error);
    assert!(matches!(result, Err(status) if status == WALK_STATUS_PERMISSION_DENIED));
    assert_ne!(WALK_STATUS_PERMISSION_DENIED, ABI_STATUS_END_OF_STREAM);
}
