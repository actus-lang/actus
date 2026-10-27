use std::collections::HashMap;
use std::fs::ReadDir;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::super::contract::{ABI_HANDLE_FAILURE, ABI_STATUS_END_OF_STREAM, ABI_STATUS_FAILURE};
use super::super::path::ActusPath;
use super::path_bridge::{replace_output_path, with_host_path};

const WALK_KIND_ENTER_DIRECTORY: i32 = 0;
const WALK_KIND_FILE: i32 = 1;
const WALK_KIND_SYMLINK: i32 = 2;
const WALK_KIND_LEAVE_DIRECTORY: i32 = 3;
const WALK_KIND_ERROR: i32 = 4;
const WALK_ERROR_FAILED: i32 = 0;
const WALK_ERROR_NOT_FOUND: i32 = 1;
const WALK_ERROR_PERMISSION_DENIED: i32 = 2;
const WALK_ERROR_UNSUPPORTED: i32 = 4;
const WALK_STATUS_NOT_FOUND: i32 = -4;
const WALK_STATUS_PERMISSION_DENIED: i32 = -5;
const WALK_STATUS_UNSUPPORTED: i32 = -6;
const WALK_MAX_DEPTH: u32 = 256;
const WALK_MAX_VISITED_DIRECTORIES: usize = 4096;
const WALK_MAX_ENTRIES_PER_DIRECTORY: usize = 4096;
const WALK_MAX_ENTRY_NAME_BYTES: usize = 4095;
const WALK_STATUS_BUFFER_TOO_SMALL: i32 = -3;

#[repr(C)]
pub struct ActusWalkEvent {
    pub depth: i32,
    pub kind: i32,
    pub error: i32,
}

struct WalkOptions {
    max_depth: u32,
    follow_symlinks: bool,
    include_hidden: bool,
    order: i32,
    error_policy: i32,
}

enum WalkFrame {
    Native { path: PathBuf, depth: u32, entries: Box<ReadDir> },
    Ordered { path: PathBuf, depth: u32, entries: Vec<PathBuf>, next: usize },
}

impl WalkFrame {
    fn path(&self) -> &Path {
        match self {
            Self::Native { path, .. } | Self::Ordered { path, .. } => path,
        }
    }

    fn depth(&self) -> u32 {
        match self {
            Self::Native { depth, .. } | Self::Ordered { depth, .. } => *depth,
        }
    }

    fn next_path(&mut self) -> std::io::Result<Option<PathBuf>> {
        match self {
            Self::Native { entries, .. } => {
                entries.next().map(|entry| entry.map(|item| item.path())).transpose()
            }
            Self::Ordered { entries, next, .. } => {
                let Some(path) = entries.get(*next).cloned() else { return Ok(None) };
                *next += 1;
                Ok(Some(path))
            }
        }
    }
}

struct PendingEvent {
    path: PathBuf,
    depth: u32,
    kind: i32,
    error: i32,
}

struct WalkerState {
    options: WalkOptions,
    frames: Vec<WalkFrame>,
    visited: Vec<PathBuf>,
    pending: Option<PendingEvent>,
}

static WALKERS: OnceLock<Mutex<HashMap<i32, WalkerState>>> = OnceLock::new();
static NEXT_WALKER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

fn walkers() -> &'static Mutex<HashMap<i32, WalkerState>> {
    WALKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn open_frame(path: PathBuf, depth: u32, order: i32) -> Result<WalkFrame, i32> {
    let directory = std::fs::read_dir(&path).map_err(|error| -error_code(&error))?;
    if order == 0 {
        return Ok(WalkFrame::Native { path, depth, entries: Box::new(directory) });
    }
    if order != 1 {
        return Err(-WALK_ERROR_FAILED);
    }
    let mut entries = Vec::new();
    for item in directory {
        if entries.len() >= WALK_MAX_ENTRIES_PER_DIRECTORY {
            return Err(WALK_STATUS_BUFFER_TOO_SMALL);
        }
        let entry = item.map_err(|error| -error_code(&error))?;
        if entry_name_key(&entry.file_name()).len() > WALK_MAX_ENTRY_NAME_BYTES {
            return Err(WALK_STATUS_BUFFER_TOO_SMALL);
        }
        entries.push(entry.path());
    }
    entries.sort_by(|left, right| {
        entry_name_key(left.file_name().unwrap_or_default())
            .cmp(&entry_name_key(right.file_name().unwrap_or_default()))
    });
    Ok(WalkFrame::Ordered { path, depth, entries, next: 0 })
}

fn error_code(error: &std::io::Error) -> i32 {
    match error.kind() {
        std::io::ErrorKind::NotFound => WALK_ERROR_NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => WALK_ERROR_PERMISSION_DENIED,
        std::io::ErrorKind::Unsupported => WALK_ERROR_UNSUPPORTED,
        _ => WALK_ERROR_FAILED,
    }
}

fn failure_status(error: i32) -> i32 {
    match error {
        WALK_ERROR_NOT_FOUND => WALK_STATUS_NOT_FOUND,
        WALK_ERROR_PERMISSION_DENIED => WALK_STATUS_PERMISSION_DENIED,
        WALK_ERROR_UNSUPPORTED => WALK_STATUS_UNSUPPORTED,
        _ => ABI_STATUS_FAILURE,
    }
}

fn status_error_code(status: i32) -> i32 {
    match status {
        WALK_STATUS_NOT_FOUND => WALK_ERROR_NOT_FOUND,
        WALK_STATUS_PERMISSION_DENIED => WALK_ERROR_PERMISSION_DENIED,
        WALK_STATUS_UNSUPPORTED => WALK_ERROR_UNSUPPORTED,
        _ => WALK_ERROR_FAILED,
    }
}

fn is_hidden(path: &Path) -> bool {
    let Some(name) = path.file_name() else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return name.as_bytes().first() == Some(&b'.');
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        return name.encode_wide().next() == Some('.' as u16);
    }
    #[allow(unreachable_code)]
    false
}

fn entry_name_key(name: &std::ffi::OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return name.as_bytes().to_vec();
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        return name.encode_wide().flat_map(u16::to_le_bytes).collect();
    }
    #[allow(unreachable_code)]
    name.to_string_lossy().as_bytes().to_vec()
}

fn allocate_walker(state: WalkerState) -> i32 {
    let handle = NEXT_WALKER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if handle <= 0 {
        return ABI_HANDLE_FAILURE as i32;
    }
    let Ok(mut table) = walkers().lock() else { return ABI_HANDLE_FAILURE as i32 };
    table.insert(handle, state);
    handle
}

fn prepare_root(path: &Path, options: WalkOptions) -> Result<WalkerState, i32> {
    if options.max_depth > WALK_MAX_DEPTH
        || !matches!(options.order, 0 | 1)
        || !matches!(options.error_policy, 0 | 1)
    {
        return Err(ABI_STATUS_FAILURE);
    }
    let snapshot = std::fs::symlink_metadata(path).map_err(|error| -error_code(&error))?;
    let mut state = WalkerState { options, frames: Vec::new(), visited: Vec::new(), pending: None };
    if snapshot.file_type().is_symlink() && !state.options.follow_symlinks {
        state.pending = Some(PendingEvent {
            path: path.to_owned(),
            depth: 0,
            kind: WALK_KIND_SYMLINK,
            error: WALK_ERROR_FAILED,
        });
        return Ok(state);
    }
    let target = if state.options.follow_symlinks {
        std::fs::canonicalize(path).map_err(|error| -error_code(&error))?
    } else {
        path.to_owned()
    };
    if snapshot.is_dir()
        || (state.options.follow_symlinks
            && std::fs::metadata(path).map_err(|error| -error_code(&error))?.is_dir())
    {
        state.visited.push(target);
        state.frames.push(open_frame(path.to_owned(), 0, state.options.order)?);
        state.pending = Some(PendingEvent {
            path: path.to_owned(),
            depth: 0,
            kind: WALK_KIND_ENTER_DIRECTORY,
            error: WALK_ERROR_FAILED,
        });
    } else {
        state.pending = Some(PendingEvent {
            path: path.to_owned(),
            depth: 0,
            kind: WALK_KIND_FILE,
            error: WALK_ERROR_FAILED,
        });
    }
    Ok(state)
}

enum FrameStep {
    Candidate { path: PathBuf, depth: u32 },
    Event(PendingEvent),
    End,
}

fn next_frame_step(state: &mut WalkerState) -> Result<FrameStep, i32> {
    let Some(frame) = state.frames.last_mut() else { return Ok(FrameStep::End) };
    let frame_path = frame.path().to_owned();
    let frame_depth = frame.depth();
    let Some(path) = frame.next_path().map_err(|error| -error_code(&error))? else {
        state.frames.pop();
        return Ok(FrameStep::Event(PendingEvent {
            path: frame_path,
            depth: frame_depth,
            kind: WALK_KIND_LEAVE_DIRECTORY,
            error: WALK_ERROR_FAILED,
        }));
    };
    Ok(FrameStep::Candidate { path, depth: frame_depth + 1 })
}

fn report_io_error(
    state: &WalkerState,
    path: PathBuf,
    depth: u32,
    error: &std::io::Error,
) -> Result<Option<PendingEvent>, i32> {
    let code = error_code(error);
    if state.options.error_policy == 0 {
        return Err(failure_status(code));
    }
    Ok(Some(PendingEvent { path, depth, kind: WALK_KIND_ERROR, error: code }))
}

fn report_status_error(
    state: &WalkerState,
    path: PathBuf,
    depth: u32,
    status: i32,
) -> Result<Option<PendingEvent>, i32> {
    let failure = if status == 0 {
        ABI_STATUS_FAILURE
    } else if status == -WALK_ERROR_NOT_FOUND {
        WALK_STATUS_NOT_FOUND
    } else if status == -WALK_ERROR_PERMISSION_DENIED {
        WALK_STATUS_PERMISSION_DENIED
    } else if status == -WALK_ERROR_UNSUPPORTED {
        WALK_STATUS_UNSUPPORTED
    } else {
        status
    };
    if state.options.error_policy == 0 {
        return Err(failure);
    }
    Ok(Some(PendingEvent { path, depth, kind: WALK_KIND_ERROR, error: status_error_code(status) }))
}

fn descend_candidate(
    state: &mut WalkerState,
    candidate: PathBuf,
    depth: u32,
) -> Result<Option<PendingEvent>, i32> {
    let identity = if state.options.follow_symlinks {
        std::fs::canonicalize(&candidate).map_err(|error| -error_code(&error))?
    } else {
        candidate.clone()
    };
    if state.visited.iter().any(|known| known == &identity) {
        return report_status_error(state, candidate, depth, -WALK_ERROR_FAILED);
    }
    if state.visited.len() >= WALK_MAX_VISITED_DIRECTORIES {
        return Err(WALK_STATUS_BUFFER_TOO_SMALL);
    }
    let child_frame = match open_frame(candidate.clone(), depth, state.options.order) {
        Ok(frame) => frame,
        Err(status) => return report_status_error(state, candidate, depth, status),
    };
    state.visited.push(identity);
    state.frames.push(child_frame);
    Ok(Some(PendingEvent {
        path: candidate,
        depth,
        kind: WALK_KIND_ENTER_DIRECTORY,
        error: WALK_ERROR_FAILED,
    }))
}

fn process_candidate(
    state: &mut WalkerState,
    candidate: PathBuf,
    depth: u32,
) -> Result<Option<PendingEvent>, i32> {
    let link_snapshot = match std::fs::symlink_metadata(&candidate) {
        Ok(snapshot) => snapshot,
        Err(error) => return report_io_error(state, candidate, depth, &error),
    };
    if depth > state.options.max_depth {
        return Ok(None);
    }
    let is_symlink = link_snapshot.file_type().is_symlink();
    if is_symlink && !state.options.follow_symlinks {
        return Ok(Some(PendingEvent {
            path: candidate,
            depth,
            kind: WALK_KIND_SYMLINK,
            error: WALK_ERROR_FAILED,
        }));
    }
    let target_snapshot = if state.options.follow_symlinks {
        match std::fs::metadata(&candidate) {
            Ok(snapshot) => snapshot,
            Err(error) => return report_io_error(state, candidate, depth, &error),
        }
    } else {
        link_snapshot
    };
    if target_snapshot.is_dir() {
        return descend_candidate(state, candidate, depth);
    }
    Ok(Some(PendingEvent {
        path: candidate,
        depth,
        kind: if is_symlink { WALK_KIND_SYMLINK } else { WALK_KIND_FILE },
        error: WALK_ERROR_FAILED,
    }))
}

fn next_event(state: &mut WalkerState) -> Result<Option<PendingEvent>, i32> {
    if let Some(event) = state.pending.take() {
        return Ok(Some(event));
    }
    loop {
        match next_frame_step(state)? {
            FrameStep::End => return Ok(None),
            FrameStep::Event(event) => return Ok(Some(event)),
            FrameStep::Candidate { path, depth } => {
                if !state.options.include_hidden && is_hidden(&path) {
                    continue;
                }
                if let Some(event) = process_candidate(state, path, depth)? {
                    return Ok(Some(event));
                }
            }
        }
    }
}

/// Opens a bounded streaming recursive walker.
///
/// # Safety
///
/// `path` must be null or point to a valid `ActusPath` for the duration of the
/// call. A null pointer is rejected without dereferencing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_walker_open(
    path: *const ActusPath,
    max_depth: i32,
    follow_symlinks: i32,
    include_hidden: i32,
    order: i32,
    error_policy: i32,
) -> i32 {
    let Some(result) = (unsafe {
        with_host_path(path, |native| {
            prepare_root(
                native,
                WalkOptions {
                    max_depth: max_depth.max(0) as u32,
                    follow_symlinks: follow_symlinks != 0,
                    include_hidden: include_hidden != 0,
                    order,
                    error_policy,
                },
            )
        })
    }) else {
        return ABI_HANDLE_FAILURE as i32;
    };
    match result {
        Ok(state) => allocate_walker(state),
        Err(status) => status,
    }
}

/// Produces one owned walk event and writes its path into caller-owned storage.
///
/// # Safety
///
/// `path` and `output` must be valid writable pointers for the duration of the
/// call, or the function must receive null pointers and reject the request
/// before dereferencing them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_walker_next(
    handle: i32,
    path: *mut ActusPath,
    output: *mut ActusWalkEvent,
) -> i32 {
    if path.is_null() || output.is_null() {
        return ABI_STATUS_FAILURE;
    }
    let Ok(mut table) = walkers().lock() else { return ABI_STATUS_FAILURE };
    let Some(state) = table.get_mut(&handle) else { return ABI_STATUS_FAILURE };
    let event = match next_event(state) {
        Ok(Some(event)) => event,
        Ok(None) => return ABI_STATUS_END_OF_STREAM,
        Err(status) => return status,
    };
    if unsafe { !replace_output_path(path, &event.path) } {
        return ABI_STATUS_FAILURE;
    }
    let output = unsafe { &mut *output };
    output.depth = event.depth as i32;
    output.kind = event.kind;
    output.error = event.error;
    0
}

/// Closes one walker token and releases all retained traversal state.
///
/// # Safety
///
/// The integer handle is treated only as an opaque capability token; no memory
/// pointer is dereferenced by this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn actus_walker_close(handle: i32) -> i32 {
    if handle < 0 {
        return ABI_STATUS_FAILURE;
    }
    let Ok(mut table) = walkers().lock() else { return ABI_STATUS_FAILURE };
    table.remove(&handle).map_or(ABI_STATUS_FAILURE, |_| 0)
}

#[cfg(test)]
#[path = "walk_tests.rs"]
mod tests;
