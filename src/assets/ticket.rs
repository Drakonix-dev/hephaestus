use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(super) struct Allocators(HashMap<TypeId, Arc<Allocator>>);

impl Allocators {
    pub(super) fn get<A: 'static>(&self) -> Option<&Arc<Allocator>> {
        self.0.get(&TypeId::of::<A>())
    }

    pub(super) fn register<A: 'static>(&mut self) -> Arc<Allocator> {
        Arc::clone(self.0.entry(TypeId::of::<A>()).or_default())
    }
}

#[derive(Default)]
pub(super) struct Allocator {
    inner: Mutex<Inner>,
}

impl Allocator {
    pub(super) fn alloc(&self) -> Ticket {
        let mut a = self.inner.lock().unwrap();
        match a.free.pop() {
            Some(id) => Ticket {
                generation: a.generations[id],
                id,
            },
            None => {
                let id = a.generations.len();
                a.generations.push(0);
                Ticket { generation: 0, id }
            }
        }
    }

    pub(super) fn release(&self, t: Ticket) {
        let mut guard = self.inner.lock().unwrap();
        let a = &mut *guard;
        let g = &mut a.generations[t.id];
        if *g == t.generation {
            a.free.push(t.id);
            *g += 1;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Ticket {
    pub(super) generation: u64,
    pub(super) id: usize,
}

#[derive(Default)]
struct Inner {
    free: Vec<usize>,
    generations: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    fn snapshot(a: &Allocator) -> (Vec<usize>, Vec<u64>) {
        let inner = a.inner.lock().unwrap();
        (inner.free.clone(), inner.generations.clone())
    }

    #[test]
    fn fresh_allocs_are_sequential_with_generation_zero() {
        let a = Allocator::default();
        for expected in 0..5 {
            let t = a.alloc();
            assert_eq!(t.id, expected);
            assert_eq!(t.generation, 0);
        }
        let (free, gens) = snapshot(&a);
        assert!(free.is_empty());
        assert_eq!(gens, vec![0; 5]);
    }

    #[test]
    fn release_bumps_generation_and_frees_id() {
        let a = Allocator::default();
        let t = a.alloc();
        a.release(t);
        let (free, gens) = snapshot(&a);
        assert_eq!(free, vec![0]);
        assert_eq!(gens, vec![1]);
    }

    #[test]
    fn realloc_reuses_id_with_new_generation() {
        let a = Allocator::default();
        let t0 = a.alloc();
        let _t1 = a.alloc();
        a.release(t0);

        let t = a.alloc();
        assert_eq!(t.id, 0);
        assert_eq!(t.generation, 1);
        // Free list drained; nothing new pushed to generations.
        let (free, gens) = snapshot(&a);
        assert!(free.is_empty());
        assert_eq!(gens.len(), 2);
    }

    #[test]
    fn freed_ids_are_reused_before_growing() {
        let a = Allocator::default();
        let ts: Vec<_> = (0..4).map(|_| a.alloc()).collect();
        for t in ts {
            a.release(t);
        }
        let ids: HashSet<_> = (0..4).map(|_| a.alloc().id).collect();
        assert_eq!(ids, (0..4).collect());
        // Only once the free list is exhausted does a new id appear.
        assert_eq!(a.alloc().id, 4);
    }

    #[test]
    fn many_cycles_keep_incrementing_generation() {
        let a = Allocator::default();
        for g in 0..100u64 {
            let t = a.alloc();
            assert_eq!(t.id, 0);
            assert_eq!(t.generation, g);
            a.release(t);
        }
        assert_eq!(snapshot(&a).1, vec![100]);
    }

    #[test]
    fn double_release_is_ignored() {
        let a = Allocator::default();
        let t = a.alloc();
        let copy = t.clone();
        a.release(t);
        a.release(copy);

        let (free, gens) = snapshot(&a);
        assert_eq!(free, vec![0], "id must only be on the free list once");
        assert_eq!(gens, vec![1], "generation must only be bumped once");

        // If the id had been pushed twice, two allocs would both get id 0.
        let x = a.alloc();
        let y = a.alloc();
        assert_ne!(x.id, y.id);
    }

    #[test]
    fn stale_ticket_cannot_free_reused_slot() {
        let a = Allocator::default();
        let old = a.alloc();
        let stale = old.clone();
        a.release(old);

        let current = a.alloc();
        assert_eq!(current.id, stale.id);
        assert_ne!(current.generation, stale.generation);

        a.release(stale);
        let (free, gens) = snapshot(&a);
        assert!(free.is_empty(), "stale release must not free a live slot");
        assert_eq!(gens, vec![1]);

        // The real owner can still release it.
        a.release(current);
        let (free, gens) = snapshot(&a);
        assert_eq!(free, vec![0]);
        assert_eq!(gens, vec![2]);
    }

    #[test]
    fn ticket_from_future_generation_is_ignored() {
        let a = Allocator::default();
        let t = a.alloc();
        a.release(Ticket {
            generation: t.generation + 5,
            id: t.id,
        });
        let (free, gens) = snapshot(&a);
        assert!(free.is_empty());
        assert_eq!(gens, vec![0]);
    }

    #[test]
    #[should_panic]
    fn release_of_never_allocated_id_panics() {
        let a = Allocator::default();
        a.release(Ticket {
            generation: 0,
            id: 42,
        });
    }

    #[test]
    fn concurrent_allocs_hand_out_unique_ids() {
        const THREADS: usize = 8;
        const PER_THREAD: usize = 1_000;

        let a = Arc::new(Allocator::default());
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let a = Arc::clone(&a);
                thread::spawn(move || {
                    (0..PER_THREAD).map(|_| a.alloc().id).collect::<Vec<_>>()
                })
            })
            .collect();

        let mut seen = HashSet::new();
        for h in handles {
            for id in h.join().unwrap() {
                assert!(seen.insert(id), "id {id} handed out twice");
            }
        }
        assert_eq!(seen.len(), THREADS * PER_THREAD);
        assert_eq!(snapshot(&a).1.len(), THREADS * PER_THREAD);
    }

    #[test]
    fn concurrent_alloc_release_churn_keeps_invariants() {
        const THREADS: usize = 8;
        const ITERS: usize = 2_000;

        let a = Arc::new(Allocator::default());
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let a = Arc::clone(&a);
                thread::spawn(move || {
                    for _ in 0..ITERS {
                        let t = a.alloc();
                        a.release(t);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let (free, gens) = snapshot(&a);
        // Every slot is released, exactly once on the free list.
        assert_eq!(free.len(), gens.len());
        let unique: HashSet<_> = free.iter().copied().collect();
        assert_eq!(unique.len(), free.len());
        // At most one slot per thread can have been live at a time.
        assert!(gens.len() <= THREADS);
        // Each alloc/release pair bumps exactly one generation.
        assert_eq!(gens.iter().sum::<u64>(), (THREADS * ITERS) as u64);
    }
}
