use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct TripleBuffer<T: Clone> {
    buffers: [T; 3],
    read_idx: AtomicUsize,
    ready_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T: Clone> TripleBuffer<T> {
    pub(crate) fn new(initial: T) -> Self {
        Self {
            buffers: [initial.clone(), initial.clone(), initial],
            read_idx: AtomicUsize::new(1),
            ready_idx: AtomicUsize::new(2),
            write_idx: AtomicUsize::new(0),
        }
    }

    pub(crate) fn write(&mut self, data: T) {
        let idx = self.write_idx.load(Ordering::Relaxed);
        self.buffers[idx] = data;

        let old_ready = self.ready_idx.swap(idx, Ordering::Release);
        let next_write = (0..3).find(|&i|
            i != old_ready && i != self.read_idx.load(Ordering::Acquire)).unwrap();

        self.write_idx.store(next_write, Ordering::Relaxed);
    }

    pub fn read(&self) -> T {
        let new_read = self.ready_idx.swap(self.read_idx.load(Ordering::Acquire), Ordering::Acquire);
        self.read_idx.store(new_read, Ordering::Release);
        self.buffers[new_read].clone()
    }
}
