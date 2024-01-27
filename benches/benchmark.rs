mod algorithms;
use algorithms::estimate_pi_leibniz;

use std::hint::black_box;
use std::time::Instant;
use thread_manager::ThreadManager;

#[test]
fn benchmark_test() {
    let pi_terms: usize = 50_000_000;
    let jobs: usize = 1_000;

    let threads: usize = 12;
    thread_manager_benchmark(pi_terms, jobs, threads);
    println!();
    single_thread_benchmark(pi_terms, jobs);
}

fn write_thread_metrics(thread_manager: &ThreadManager<f64>) {
    let busy_threads: usize = thread_manager.busy_threads();
    let job_queue: usize = thread_manager.job_queue();

    print!(
        "\x1B[2KThreads: {} | Job Queue: {}\r",
        busy_threads, job_queue
    );
}

fn add_thread_jobs(thread_manager: &ThreadManager<f64>, pi_terms: usize, jobs: usize) {
    for idx in 0..jobs {
        if idx % 100 == 0 {
            write_thread_metrics(thread_manager);
        }

        thread_manager.execute(move || {
            let pi: f64 = estimate_pi_leibniz(pi_terms);
            pi
        });
    }
}

fn pending_thread_metrics(thread_manager: &ThreadManager<f64>) {
    let mut counter: usize = 0;
    loop {
        counter += 1;
        if counter % 1000 == 0 {
            write_thread_metrics(thread_manager);
        }

        if thread_manager.job_queue() == 0 {
            break;
        }
    }
    write_thread_metrics(thread_manager);
}

fn write_elapsed_time(now: &Instant) {
    let elapsed: u128 = now.elapsed().as_millis();
    println!("\nTime: {}", elapsed);
}

fn single_thread_benchmark(pi_terms: usize, jobs: usize) {
    println!("[SINGLE THREAD BENCHMARK]");
    let now: Instant = Instant::now();

    for _ in 0..jobs {
        let pi: f64 = estimate_pi_leibniz(pi_terms);
        black_box(pi);
    }

    write_elapsed_time(&now);
}

fn thread_manager_benchmark(pi_terms: usize, jobs: usize, threads: usize) {
    println!("[THREAD MANAGER BENCHMARK]");
    let thread_manager: ThreadManager<f64> = ThreadManager::new(threads);
    let now: Instant = Instant::now();

    add_thread_jobs(&thread_manager, pi_terms, jobs);
    pending_thread_metrics(&thread_manager);
    thread_manager.join();

    write_elapsed_time(&now);
    write_thread_metrics(&thread_manager);
    println!("\nDistribution: {:?}", thread_manager.job_distribution());
}
