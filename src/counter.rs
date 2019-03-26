use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;

pub struct Counter {
    signal: Option<Arc<AtomicBool>>,
    limit: usize,
    count: usize,
}

impl Counter {
    pub fn new(limit: usize, signal: Option<Arc<AtomicBool>>) -> Counter {
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
                return atom.load(atomic::Ordering::Relaxed);
            }
            return true;
        }
    }
}
