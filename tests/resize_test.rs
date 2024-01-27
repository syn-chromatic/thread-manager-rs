use std::thread::sleep;
use std::time::Duration;

use thread_manager::ThreadManager;

#[test]
fn resize_test() {
    let mut thread_manager = ThreadManager::<()>::new(4);

    for _ in 0..100 {
        thread_manager.execute(|| {
            sleep(Duration::from_millis(10));
        });
    }

    thread_manager.resize(12);

    for _ in 0..100 {
        thread_manager.execute(|| {
            sleep(Duration::from_millis(10));
        });
    }

    thread_manager.resize(6);

    for _ in 0..100 {
        thread_manager.execute(|| {
            sleep(Duration::from_millis(10));
        });
    }

    thread_manager.join();

    let job_dist: Vec<usize> = thread_manager.job_distribution();
    let dist_sum: usize = job_dist.iter().sum();
    println!("Distribution: {:?} | Sum: {}", job_dist, dist_sum);
}
