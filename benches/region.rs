use std::hint::black_box;
use std::time::Instant;

use actus::runtime::InMemoryRegion;

const ITERATIONS: u32 = 10_000;
const BULK_ITERATIONS: u32 = 100;

fn measure(iterations: u32, mut operation: impl FnMut()) -> u128 {
    let started = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    started.elapsed().as_nanos() / u128::from(iterations)
}

fn measure_bulk(bytes: usize) -> (u128, u128) {
    let mut region =
        InMemoryRegion::new(7, 1, bytes as u64, 0, bytes as u64).expect("bulk region should open");
    let source = vec![1u8; bytes];
    let mut destination = vec![0u8; bytes];
    let read_ns = measure(BULK_ITERATIONS, || {
        let generation = region.descriptor().generation;
        let view = region.borrow_abs(7, generation).expect("bulk read view should open");
        view.read_range(0, bytes as u64, &mut destination).expect("bulk read should succeed");
        black_box(&destination);
    });
    let write_ns = measure(BULK_ITERATIONS, || {
        let generation = region.descriptor().generation;
        let mut view = region.borrow_ins(7, generation).expect("bulk write view should open");
        view.write_range(0, bytes as u64, &source).expect("bulk write should succeed");
    });
    (read_ns, write_ns)
}

fn main() {
    let open_ns = measure(ITERATIONS, || {
        black_box(InMemoryRegion::new(7, 1, 1, 0, 1).expect("region should open"));
    });

    let mut region = InMemoryRegion::new(7, 1, 1, 0, 1).expect("region should open");
    let read_ns = measure(ITERATIONS, || {
        let generation = region.descriptor().generation;
        let view = region.borrow_abs(7, generation).expect("read view should open");
        let mut destination = [0u8; 1];
        view.read(0, &mut destination).expect("read should succeed");
        black_box(destination);
    });

    let mut source = [1u8; 1];
    let write_ns = measure(ITERATIONS, || {
        let generation = region.descriptor().generation;
        let mut view = region.borrow_ins(7, generation).expect("write view should open");
        view.write(0, &source).expect("write should succeed");
        source[0] = source[0].wrapping_add(1);
    });

    let publish_ns = measure(ITERATIONS, || {
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("write view should open");
            view.write(0, &source).expect("write should succeed");
        }
        region.publish(generation).expect("publish should succeed");
    });

    let cancel_ns = measure(ITERATIONS, || {
        let generation = region.descriptor().generation;
        {
            let mut view = region.borrow_ins(7, generation).expect("write view should open");
            view.write(0, &source).expect("write should succeed");
        }
        region.cancel(generation).expect("cancel should succeed");
    });

    let remap_ns = measure(ITERATIONS, || {
        let generation = region.descriptor().generation;
        region.remap(generation, vec![0], 0, 1).expect("remap should succeed");
    });

    let (read_64k_ns, write_64k_ns) = measure_bulk(64 * 1024);
    let (read_128k_ns, write_128k_ns) = measure_bulk(128 * 1024);
    println!(
        "REGION_OPS iterations={ITERATIONS} avg_ns open={open_ns} read={read_ns} write={write_ns} publish={publish_ns} cancel={cancel_ns} remap={remap_ns} close=native_fixture"
    );
    println!(
        "REGION_BULK iterations={BULK_ITERATIONS} bytes=65536 avg_ns read={read_64k_ns} write={write_64k_ns}"
    );
    println!(
        "REGION_BULK iterations={BULK_ITERATIONS} bytes=131072 avg_ns read={read_128k_ns} write={write_128k_ns}"
    );
}
