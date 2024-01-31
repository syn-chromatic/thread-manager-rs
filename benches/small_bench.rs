use std::hint::black_box;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use thread_manager::ThreadManager;
use thread_manager::ThreadManagerStack;

const ITERATIONS: usize = 1_000_000;
const THREADS: usize = 4;
const WPC: usize = 4;

#[test]
fn small_bench() {
    sleep(Duration::from_millis(500));

    tm_bench();
    tm_asymmetric_bench();
    tm_stack_bench();
    tm_stack_asymmetric_bench();

    st_bench();
}

fn st_bench() {
    println!("[SINGLE THREAD BENCH]");
    let now: Instant = Instant::now();
    for idx in 0..ITERATIONS {
        black_box(small(idx));
    }

    let elapsed: Duration = now.elapsed();
    println!(
        "Time {}ms | Time {}us",
        elapsed.as_millis(),
        elapsed.as_micros()
    );
}

fn tm_bench() {
    println!("[THREAD MANAGER BENCH]");
    let thread_manager: ThreadManager<()> = ThreadManager::<()>::new(THREADS);
    let now: Instant = Instant::now();
    for idx in 0..ITERATIONS {
        thread_manager.execute(move || black_box(small(idx)));
    }

    let elapsed: Duration = now.elapsed();
    println!(
        "Time {}ms | Time {}us",
        elapsed.as_millis(),
        elapsed.as_micros()
    );
}

fn tm_asymmetric_bench() {
    println!("[THREAD MANAGER ASYMMETRIC BENCH]");
    let thread_manager: ThreadManager<()> = ThreadManager::<()>::new_asymmetric(THREADS, WPC);
    let now: Instant = Instant::now();
    for idx in 0..ITERATIONS {
        thread_manager.execute(move || black_box(small(idx)));
    }

    let elapsed: Duration = now.elapsed();
    println!(
        "Time {}ms | Time {}us",
        elapsed.as_millis(),
        elapsed.as_micros()
    );
}

fn tm_stack_bench() {
    println!("[THREAD MANAGER STACK BENCH]");
    let thread_manager: ThreadManagerStack<_, ()> = ThreadManagerStack::<_, ()>::new(THREADS);
    let now: Instant = Instant::now();
    for idx in 0..ITERATIONS {
        thread_manager.execute(move || black_box(small(idx)));
    }

    let elapsed: Duration = now.elapsed();
    println!(
        "Time {}ms | Time {}us",
        elapsed.as_millis(),
        elapsed.as_micros()
    );
}

fn tm_stack_asymmetric_bench() {
    println!("[THREAD MANAGER STACK ASYMMETRIC BENCH]");
    let thread_manager: ThreadManagerStack<_, ()> =
        ThreadManagerStack::<_, ()>::new_asymmetric(THREADS, WPC);
    let now: Instant = Instant::now();
    for idx in 0..ITERATIONS {
        thread_manager.execute(move || black_box(small(idx)));
    }

    let elapsed: Duration = now.elapsed();
    println!(
        "Time {}ms | Time {}us",
        elapsed.as_millis(),
        elapsed.as_micros()
    );
}

fn small(idx: usize) {
    let mut result: f32 = 0.0;
    for _ in 0..1 {
        let idx: f32 = idx as f32;
        let pow: f32 = idx.powf(16.0f32).sqrt();
        result += pow;
    }
    black_box(result);
}
