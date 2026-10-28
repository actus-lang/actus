#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
#[derive(Clone, Copy)]
enum LockMode {
    Shared,
    Exclusive,
}

#[cfg(windows)]
#[derive(Default)]
struct LockState {
    shared: u32,
    exclusive: bool,
    handles: HashMap<i64, LockMode>,
}

#[cfg(windows)]
static LOCKS: OnceLock<Mutex<HashMap<(u64, u64), LockState>>> = OnceLock::new();

#[cfg(windows)]
pub(super) fn register(identity: (u64, u64), handle: i64, exclusive: bool) -> bool {
    let Ok(mut locks) = LOCKS.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return false;
    };
    let state = locks.entry(identity).or_default();
    if state.handles.contains_key(&handle)
        || (exclusive && (state.exclusive || state.shared != 0))
        || (!exclusive && state.exclusive)
    {
        return false;
    }
    if exclusive {
        state.exclusive = true;
        state.handles.insert(handle, LockMode::Exclusive);
    } else {
        state.shared += 1;
        state.handles.insert(handle, LockMode::Shared);
    }
    true
}

#[cfg(windows)]
pub(super) fn remove(identity: (u64, u64), handle: i64) {
    let Ok(mut locks) = LOCKS.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return;
    };
    let Some(state) = locks.get_mut(&identity) else { return };
    match state.handles.remove(&handle) {
        Some(LockMode::Shared) => state.shared -= 1,
        Some(LockMode::Exclusive) => state.exclusive = false,
        None => return,
    }
    if state.shared == 0 && !state.exclusive {
        locks.remove(&identity);
    }
}
