mod result_test {
    use thread_manager::ThreadManager;

    #[test]
    fn result_test() {
        println!("[RESULT TEST]");
        let thread_manager = ThreadManager::<usize>::new(4);

        for idx in 0..5 {
            thread_manager.execute(move || super::function(idx));
        }

        for result in thread_manager.yield_results() {
            println!("Result: {}", result);
        }
    }
}

mod asymmetric_result_test {
    use thread_manager::ThreadManager;

    #[test]
    fn asymmetric_result_test() {
        println!("[ASYMMETRIC RESULT TEST]");
        let thread_manager = ThreadManager::<usize>::new_asymmetric(4, 2);

        for idx in 0..5 {
            thread_manager.execute(move || super::function(idx));
        }

        for result in thread_manager.yield_results() {
            println!("Result: {}", result);
        }
    }
}

fn function(idx: usize) -> usize {
    idx
}
