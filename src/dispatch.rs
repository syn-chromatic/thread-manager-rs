use std::sync::atomic::AtomicUsize;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct DispatchID {
    id: AtomicUsize,
    max: usize,
}

impl DispatchID {
    pub fn new(max: usize) -> Self {
        let id: AtomicUsize = AtomicUsize::new(0);
        Self { id, max }
    }

    pub fn fetch_and_update(&self) -> usize {
        let dispatch: usize = self.id.load(LOAD_ORDER);
        if dispatch >= (self.max - 1) {
            self.id.store(0, STORE_ORDER);
        } else {
            self.id.store(dispatch + 1, STORE_ORDER);
        }
        dispatch
    }
}
