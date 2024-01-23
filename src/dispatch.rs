use std::sync::atomic::AtomicUsize;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct DispatchCycle {
    id: AtomicUsize,
    max: usize,
}

impl DispatchCycle {
    pub fn new(max: usize) -> Self {
        let id: AtomicUsize = AtomicUsize::new(0);
        Self { id, max }
    }

    pub fn set_id(&self, id: usize) {
        self.id.store(id, STORE_ORDER);
    }

    pub fn set_max(&mut self, max: usize) {
        self.max = max;
        let id: usize = self.fetch();
        if id >= (self.max - 1) {
            self.set_id(0);
        }
    }

    pub fn fetch(&self) -> usize {
        self.id.load(LOAD_ORDER)
    }

    pub fn fetch_max(&self) -> usize {
        self.max
    }

    pub fn fetch_and_update(&self) -> usize {
        let id: usize = self.fetch();
        if id >= (self.max - 1) {
            self.set_id(0);
        } else {
            self.set_id(id + 1);
        }
        id
    }
}
