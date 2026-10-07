#![no_std]
#![no_main]

use actus_freestanding_region_provider::Provider;
use core::panic::PanicInfo;

core::arch::global_asm!(
    r#"
    .section .vector_table,"a",%progbits
    .align 2
    .global __vector_table
__vector_table:
    .word __stack_top
    .word Reset
    .rept 14
    .word DefaultHandler
    .endr
"#
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Evidence {
    magic: u32,
    state: u32,
    iterations: u32,
    reads: u32,
    writes: u32,
    publishes: u32,
    cancels: u32,
    last_value: u32,
}

#[unsafe(no_mangle)]
extern "C" fn DefaultHandler() -> ! {
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn Reset() -> ! {
    rust_main()
}

#[unsafe(link_section = ".noinit")]
static mut EVIDENCE: Evidence = Evidence {
    magic: 0u32,
    state: 0u32,
    iterations: 0u32,
    reads: 0u32,
    writes: 0u32,
    publishes: 0u32,
    cancels: 0u32,
    last_value: 0u32,
};

const EVIDENCE_MAGIC: u32 = 0x41494531u32;
const STATE_RUNNING: u32 = 1u32;
const STATE_FAILURE: u32 = 0xEEu32;

fn publish_evidence(evidence: Evidence) {
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(EVIDENCE), evidence);
    }
}

fn rust_main() -> ! {
    let mut resident = [0u8; 8];
    let mut published = [0u8; 8];
    let mut destination = [0u8; 1];
    let mut provider = Provider::<1>::new();

    let descriptor =
        match provider.open(&mut resident, &mut published, 1u64, 8u64, 0u64, 8u64, 32u8, 32u8) {
            Ok(descriptor) => descriptor,
            Err(_error) => {
                publish_evidence(Evidence {
                    magic: EVIDENCE_MAGIC,
                    state: STATE_FAILURE,
                    iterations: 0u32,
                    reads: 0u32,
                    writes: 0u32,
                    publishes: 0u32,
                    cancels: 0u32,
                    last_value: 0u32,
                });
                loop {}
            }
        };

    let mut evidence = Evidence {
        magic: EVIDENCE_MAGIC,
        state: STATE_RUNNING,
        iterations: 0u32,
        reads: 0u32,
        writes: 0u32,
        publishes: 0u32,
        cancels: 0u32,
        last_value: 0u32,
    };
    publish_evidence(evidence);

    loop {
        let source = [42u8];
        if provider.write(descriptor, 7u64, &source).is_err() {
            evidence.state = STATE_FAILURE;
            publish_evidence(evidence);
            loop {}
        }
        evidence.writes += 1u32;

        let published_descriptor = match provider.publish(descriptor) {
            Ok(next) => next,
            Err(_error) => {
                evidence.state = STATE_FAILURE;
                publish_evidence(evidence);
                loop {}
            }
        };
        evidence.publishes += 1u32;

        if provider.read(published_descriptor, 7u64, &mut destination).is_err() {
            evidence.state = STATE_FAILURE;
            publish_evidence(evidence);
            loop {}
        }
        evidence.reads += 1u32;
        evidence.last_value = destination[0] as u32;

        if provider.cancel(published_descriptor).is_err() {
            evidence.state = STATE_FAILURE;
            publish_evidence(evidence);
            loop {}
        }
        evidence.cancels += 1u32;
        evidence.iterations += 1u32;
        publish_evidence(evidence);
    }
}
