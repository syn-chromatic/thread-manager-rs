use thread_manager::ThreadManager;

use std::time::Instant;

#[test]
fn benchmark_test() {
    let x: usize = 40;
    let n: usize = 100;
    let values: Vec<usize> = vec![x; n];

    let threads: usize = 12;
    thread_manager_benchmark(&values, threads);
}

fn fibonacci(n: usize) -> usize {
    if n <= 1 {
        return n;
    }
    let value: usize = fibonacci(n - 1) + fibonacci(n - 2);
    value
}

fn write_thread_metrics(thread_manager: &ThreadManager) {
    let busy_threads: usize = thread_manager.get_busy_threads();
    let job_queue: usize = thread_manager.get_job_queue();

    print!(
        "\x1B[2KThreads: {} | Job Queue: {}\r",
        busy_threads, job_queue
    );
}

fn add_thread_jobs(thread_manager: &ThreadManager, values: &Vec<usize>) {
    for (idx, value) in values.iter().enumerate() {
        if idx % 50 == 0 {
            write_thread_metrics(thread_manager);
        }

        let value: usize = *value;
        thread_manager.execute(move || {
            fibonacci(value);
        });
    }
}

fn pending_thread_metrics(thread_manager: &ThreadManager) {
    let mut counter: usize = 0;
    loop {
        counter += 1;
        if counter % 50 == 0 {
            write_thread_metrics(thread_manager);
        }

        if thread_manager.get_job_queue() == 0 {
            break;
        }
    }
}

fn write_elapsed_time(now: &Instant) {
    let elapsed: u128 = now.elapsed().as_millis();
    println!("\nTime: {}", elapsed);
}

fn thread_manager_benchmark(values: &Vec<usize>, threads: usize) {
    println!("Benchmarking..\n");
    let thread_manager: ThreadManager = ThreadManager::new(threads);

    let now: Instant = Instant::now();

    add_thread_jobs(&thread_manager, values);
    pending_thread_metrics(&thread_manager);
    thread_manager.join();
    write_elapsed_time(&now);
}
