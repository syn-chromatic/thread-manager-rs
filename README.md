## `⌽` Thread Manager
Thread Manager is a Rust library that provides a simple and efficient way to manage a pool of threads for executing jobs in parallel.

It is designed to abstract away the complexities of thread management and provides a convenient interface for parallelizing workloads.


#### Add to `Cargo.toml`
```
thread-manager = "*"
```


___
### `➢` Features
* **Job Submission:** Easily submit jobs for parallel execution, which are then efficiently distributed among worker threads for optimal performance.

* **Result Retrieval:** You can retrieve results during execution by either immediately fetching the available results or yielding them as each job completes. This process also allows for the submission of additional jobs while iterating over the results!

* **Pool Resizing:** Offers the capability to resize the thread manager during execution, to optimize resource allocation according to the current workload.

* **Thread Monitoring:** Keep track of your thread manager with detailed insights, including thread activity, workload distribution, and more.

* **Graceful Termination:** Supports graceful termination of worker threads, ensuring that currently executing jobs are concluded before shutting down.


___
### `➢` Usage

#### `⤷` Basic Usage
```rust
fn main() {
    // Create ThreadManager with 4 worker threads
    // ::<T> specifies return type for jobs
    let mut thread_manager = ThreadManager::<()>::new(4);

    // Submit job for execution
    thread_manager.execute(|| {
        // Your job logic here
    });

    // Optional ways to proceed after executing a job
    //
    // Resize the number of worker threads
    thread_manager.resize(6);

    // Wait for all worker threads to complete
    thread_manager.join();

    // Terminate all worker threads gracefully and join
    thread_manager.terminate_all();
}
```


#### `⤷` Retrieving Results
```rust
fn main() {
    // Create ThreadManager with 4 worker threads
    // ::<T> specifies return type for jobs
    let mut thread_manager = ThreadManager::<f32>::new(4);

    // Submit job for execution
    thread_manager.execute(|| {
        return 50.0 / 32.0;
    });

    // The ResultIter retrieves all the available results without blocking
    for result in thread_manager.results() {
        println!("{}", result);
    }

    // The YieldResultIter blocks if there are jobs in the queue
    // This way the 'for loop' only completes when all jobs are executed
    for result in thread_manager.yield_results() {
        println!("{}", result);
        // You can execute jobs while iterating over the results
        // Beware that it will run indefinitely if there is no condition for execution
        // As it will execute and yield a result in the same loop
    }
}
```


#### `⤷` Monitoring Status And Job Information
```rust
fn main() {
    // ... Create thread manager and execute jobs

    // Worker threads that could be busy or waiting
    let active_threads: usize = thread_manager.active_threads();

    // Worker threads that are busy and executing a job
    let busy_threads: usize = thread_manager.busy_threads();

    // Worker threads that are waiting to receive a job
    let waiting_threads: usize = thread_manager.waiting_threads();

    // The amount of jobs left in the queue
    let job_queue: usize = thread_manager.job_queue();

    // The job distribution of execution across worker threads
    // Example distribution of 4 worker threads:
    // [4, 3, 3, 3] => each value is the amount of jobs executed for each worker
    let job_distribution: Vec<usize> = thread_manager.job_distribution();

    // The total amount of jobs received across worker threads
    let received_jobs: usize = thread_manager.received_jobs();

    // The total amount of jobs sent across worker threads
    let sent_jobs: usize = thread_manager.sent_jobs();

    // The total amount of jobs concluded across worker threads
    let concluded_jobs: usize = thread_manager.concluded_jobs();
}
```


___
### `➢` To-Do
- [ ] — Add documentation


___
### `➢` License
```
This project is licensed under the MIT License.
See the LICENSE file for more information.
```