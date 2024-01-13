mod algorithms;
use algorithms::estimate_pi_leibniz;

use std::hint::black_box;
use std::time::Instant;
use thread_manager::ThreadManager;

#[test]
fn benchmark_test() {
    let pi_terms: usize = 50_000_000;
    let jobs: usize = 100;

    let threads: usize = 12;
    thread_manager_benchmark(pi_terms, jobs, threads);
}

fn write_thread_metrics(thread_manager: &ThreadManager) {
    let busy_threads: usize = thread_manager.get_busy_threads();
    let job_queue: usize = thread_manager.get_job_queue();

    print!(
        "\x1B[2KThreads: {} | Job Queue: {}\r",
        busy_threads, job_queue
    );
}

fn add_thread_jobs(thread_manager: &ThreadManager, pi_terms: usize, jobs: usize) {
    for idx in 0..jobs {
        if idx % 100 == 0 {
            write_thread_metrics(thread_manager);
        }

        thread_manager.execute(move || {
            let pi: f64 = estimate_pi_leibniz(pi_terms);
            black_box(pi);
        });
    }
}

fn pending_thread_metrics(thread_manager: &ThreadManager) {
    let mut counter: usize = 0;
    loop {
        counter += 1;
        if counter % 1000 == 0 {
            write_thread_metrics(thread_manager);
        }

        if thread_manager.get_job_queue() == 0 {
            break;
        }
    }
    write_thread_metrics(thread_manager);
}

fn write_elapsed_time(now: &Instant) {
    let elapsed: u128 = now.elapsed().as_millis();
    println!("\nTime: {}", elapsed);
}

fn thread_manager_benchmark(pi_terms: usize, jobs: usize, threads: usize) {
    println!("Benchmarking..\n");
    let thread_manager: ThreadManager = ThreadManager::new(threads);

    let now: Instant = Instant::now();

    add_thread_jobs(&thread_manager, pi_terms, jobs);
    pending_thread_metrics(&thread_manager);
    thread_manager.join();

    write_thread_metrics(&thread_manager);
    write_elapsed_time(&now);
    println!("Distribution: {:?}", thread_manager.get_job_distribution());
}
