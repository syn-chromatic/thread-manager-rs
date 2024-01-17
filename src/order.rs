use std::sync::atomic::Ordering;

pub const LOAD_ORDER: Ordering = Ordering::Acquire;
pub const STORE_ORDER: Ordering = Ordering::Release;
pub const FETCH_ORDER: Ordering = Ordering::Release;
