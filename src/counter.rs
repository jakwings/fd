use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;

pub struct Counter {
    signal: Option<Arc<AtomicUsize>>,
    limit: usize,
    count: usize,
}

impl Counter {
    pub fn new(limit: usize, signal: Option<Arc<AtomicUsize>>) -> Counter {
        Counter {
            signal,
            limit,
            count: 0,
        }
    }

    pub fn inc(&mut self) -> bool {
        if self.count < self.limit {
            self.count += 1;
            return false;
        } else {
            self.count = 0;
            if let Some(ref atom) = self.signal {
                return atom.load(atomic::Ordering::Relaxed) != 0;
            }
            return true;
        }
    }
}
