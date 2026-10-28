use std::sync::atomic::{AtomicU8, Ordering};

const FAULT_NONE: u8 = 0;
pub(crate) const FAULT_STAGING_OPEN_FAILURE: u8 = 1;
pub(crate) const FAULT_WRITE_FAILURE: u8 = 2;
pub(crate) const FAULT_SHORT_WRITE: u8 = 3;
pub(crate) const FAULT_FLUSH_FAILURE: u8 = 4;
pub(crate) const FAULT_CLOSE_FAILURE: u8 = 5;
pub(crate) const FAULT_CLEANUP_FAILURE: u8 = 6;
pub(crate) const FAULT_RENAME_FAILURE: u8 = 7;
pub(crate) const FAULT_INTERRUPTED_BEFORE_RENAME: u8 = 8;
pub(crate) const FAULT_PARENT_OPEN_FAILURE: u8 = 9;
pub(crate) const FAULT_PARENT_SYNC_FAILURE: u8 = 10;
pub(crate) const FAULT_INTERRUPTED_PUBLICATION: u8 = 11;

static PENDING_FAULT: AtomicU8 = AtomicU8::new(FAULT_NONE);

#[unsafe(no_mangle)]
pub extern "C" fn actus_test_publication_fault(code: i32) -> i32 {
    let code = u8::try_from(code).unwrap_or(FAULT_NONE);
    if matches!(
        code,
        FAULT_STAGING_OPEN_FAILURE
            | FAULT_WRITE_FAILURE
            | FAULT_SHORT_WRITE
            | FAULT_FLUSH_FAILURE
            | FAULT_CLOSE_FAILURE
            | FAULT_CLEANUP_FAILURE
            | FAULT_RENAME_FAILURE
            | FAULT_INTERRUPTED_BEFORE_RENAME
            | FAULT_PARENT_OPEN_FAILURE
            | FAULT_PARENT_SYNC_FAILURE
            | FAULT_INTERRUPTED_PUBLICATION
    ) {
        PENDING_FAULT.store(code, Ordering::Release);
        return 0;
    }
    -1
}

pub(crate) fn take_fault(code: u8) -> bool {
    PENDING_FAULT.compare_exchange(code, FAULT_NONE, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

pub(crate) fn has_fault(code: u8) -> bool {
    PENDING_FAULT.load(Ordering::Acquire) == code
}
