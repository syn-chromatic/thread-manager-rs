## `⌽` Thread Manager
Thread Manager is a Rust library that provides a simple and efficient way to manage a pool of threads for executing jobs in parallel.

It is designed to abstract away the complexities of thread management and provides a convenient interface for parallelizing workloads with zero dependencies.

___
### `➢` Features
* **Dynamic Thread Management:** ThreadManager allows dynamic resizing of the thread pool, enabling efficient resource utilization based on the workload.

* **Job Execution:** Submit jobs for execution, and ThreadManager will distribute them among the available worker threads.

* **Thread Status Monitoring:** ThreadManager provides various methods to monitor the status of the thread pool, including active threads, busy threads, waiting threads, job distribution among threads, and more.

* **Graceful Termination:** The library supports graceful termination of threads, ensuring that all pending jobs are completed before shutting down.

___
### `➢` Usage

#### `⤷` Basic Usage
```rust
fn main() {
    // Create ThreadManager with 4 worker threads
    let mut thread_manager = ThreadManager::new(4);

    // Submit job for execution
    thread_manager.execute(|| {
        // Your job logic here
    });


    // Optional ways to proceed after executing a job.
    // ...
    // Increase the number of threads dynamically
    thread_manager.set_thread_size(6);

    // Terminate all threads gracefully and join
    thread_manager.terminate_all();

    // Join and wait for all threads to complete
    thread_manager.join();
}
```

#### `⤷` Monitoring Status And Job Information
```rust
fn main() {
    // ... Create thread manager and execute jobs.

    // Threads that are spawned that could be busy or waiting.
    let active_threads: usize = thread_manager.get_active_threads();

    // Threads that are busy and currently executing a job.
    let busy_threads: usize = thread_manager.get_busy_threads();

    // Threads that are waiting to receive a job.
    let waiting_threads: usize = thread_manager.get_waiting_threads();

    // The amount of jobs left in the queue.
    let job_queue: usize = thread_manager.get_job_queue();

    // The job distribution that are executed among threads
    // Example distribution of 4 threads:
    // [3, 3, 3, 4] => each value is the amount of jobs executed for each thread.
    let job_distribution: Vec<usize> = thread_manager.get_job_distribution();

    // The amount of jobs received among all threads.
    let received_jobs: usize = thread_manager.get_received_jobs();

    // The amount of jobs sent to all threads.
    let sent_jobs: usize = thread_manager.get_sent_jobs();

    // The amount of jobs completed among all threads.
    let completed_jobs: usize = thread_manager.get_completed_jobs();
}
```
