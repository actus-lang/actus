use std::sync::atomic::{AtomicU8, Ordering};

const FAULT_NONE: u8 = 0;
pub(crate) const FAULT_SHORT_WRITE: u8 = 1;
pub(crate) const FAULT_FLUSH_FAILURE: u8 = 2;
pub(crate) const FAULT_RENAME_FAILURE: u8 = 3;
pub(crate) const FAULT_PARENT_SYNC_FAILURE: u8 = 4;
pub(crate) const FAULT_INTERRUPTED_PUBLICATION: u8 = 5;

static PENDING_FAULT: AtomicU8 = AtomicU8::new(FAULT_NONE);

#[unsafe(no_mangle)]
pub extern "C" fn actus_test_publication_fault(code: i32) -> i32 {
    let code = u8::try_from(code).unwrap_or(FAULT_NONE);
    if matches!(
        code,
        FAULT_SHORT_WRITE
            | FAULT_FLUSH_FAILURE
            | FAULT_RENAME_FAILURE
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
