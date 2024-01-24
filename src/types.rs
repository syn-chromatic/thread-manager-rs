pub type FnType<T> = Box<dyn Fn() -> T + Send + 'static>;
