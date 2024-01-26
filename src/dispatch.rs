use std::sync::atomic::AtomicUsize;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct DispatchCycle {
    id: AtomicUsize,
    size: usize,
}

impl DispatchCycle {
    pub fn new(size: usize) -> Self {
        let id: AtomicUsize = AtomicUsize::new(0);
        Self { id, size }
    }

    pub fn set_id(&self, id: usize) {
        self.id.store(id, STORE_ORDER);
    }

    pub fn set_size(&mut self, size: usize) {
        self.size = size;
        let id: usize = self.fetch_id();
        if id >= (self.size - 1) {
            self.set_id(0);
        }
    }

    pub fn fetch_id(&self) -> usize {
        self.id.load(LOAD_ORDER)
    }

    pub fn fetch_size(&self) -> usize {
        self.size
    }

    pub fn fetch_and_update(&self) -> usize {
        let id: usize = self.fetch_id();
        if id >= (self.size - 1) {
            self.set_id(0);
        } else {
            self.set_id(id + 1);
        }
        id
    }
}
