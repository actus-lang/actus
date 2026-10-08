use std::hint::black_box;
use std::time::Instant;

use actus::runtime::InMemoryRegion;

const ITERATIONS: u32 = 10_000;

fn measure(mut operation: impl FnMut()) -> u128 {
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        operation();
    }
    started.elapsed().as_nanos() / u128::from(ITERATIONS)
}

fn main() {
    let open_ns = measure(|| {
        black_box(InMemoryRegion::new(7, 1, 1, 0, 1).expect("region should open"));
    });

    let mut region = InMemoryRegion::new(7, 1, 1, 0, 1).expect("region should open");
    let read_ns = measure(|| {
        let generation = region.descriptor().generation;
        let view = region.borrow_abs(7, generation).expect("read view should open");
        let mut destination = [0u8; 1];
        view.read(0, &mut destination).expect("read should succeed");
        black_box(destination);
    });

    let mut source = [1u8; 1];
    let write_ns = measure(|| {
        let generation = region.descriptor().generation;
        let mut view = region.borrow_ins(7, generation).expect("write view should open");
        view.write(0, &source).expect("write should succeed");
        source[0] = source[0].wrapping_add(1);
    });

    let publish_ns = measure(|| {
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("write view should open");
            view.write(0, &source).expect("write should succeed");
        }
        region.publish(generation).expect("publish should succeed");
    });

    let cancel_ns = measure(|| {
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("write view should open");
            view.write(0, &source).expect("write should succeed");
        }
        region.cancel(generation).expect("cancel should succeed");
    });

    let remap_ns = measure(|| {
        let generation = region.descriptor().generation;
        region.remap(generation, vec![0], 0, 1).expect("remap should succeed");
    });

    println!(
        "REGION_OPS iterations={ITERATIONS} avg_ns open={open_ns} read={read_ns} write={write_ns} publish={publish_ns} cancel={cancel_ns} remap={remap_ns} close=native_fixture"
    );
}
