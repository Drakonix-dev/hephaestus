use core::fmt;
use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use crossbeam_channel::Sender;

use crate::assets::{
    AssetError, INVARIANT,
    ticket::{Allocator, Ticket},
};

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

pub trait Asset: private::BuiltAsset {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssetLabel(Arc<str>);

impl AssetLabel {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for AssetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl From<&str> for AssetLabel {
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl From<String> for AssetLabel {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

#[derive(Default)]
pub(super) struct Stores(HashMap<TypeId, Box<dyn ErasedStore>>);

impl Stores {
    pub(super) fn get_built<A: Asset>(
        &self,
        key: &Key<A>,
    ) -> Option<&AssetState<A::Built>> {
        self.0
            .get(&TypeId::of::<A>())
            .expect("handle minted by different registry")
            .as_any()
            .downcast_ref::<Store<A>>()
            .expect(INVARIANT)
            .lookup(&key)
    }

    pub(super) fn get_erased(
        &mut self,
        id: &TypeId,
    ) -> Option<&mut Box<dyn ErasedStore>> {
        self.0.get_mut(&id)
    }

    pub(super) fn init<A: Asset>(&mut self, allocator: Arc<Allocator>) {
        let _ = self
            .0
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(Store::<A>::new(allocator)));
    }
}

pub(super) trait ErasedStore: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn free(&mut self, freed: Freed);
}

pub(super) struct Store<A: Asset> {
    allocator: Arc<Allocator>,
    items: Vec<Slot<A>>,
}

impl<A: Asset> Store<A> {
    fn new(allocator: Arc<Allocator>) -> Self {
        Self {
            allocator,
            items: Vec::new(),
        }
    }

    fn ensure(&mut self, id: usize) -> &mut Slot<A> {
        if id >= self.items.len() {
            self.items.resize_with(id + 1, || Slot::new());
        }

        &mut self.items[id]
    }

    fn free(&mut self, freed: Freed) {
        let slot = self.ensure(freed.id);
        if slot.generation <= freed.generation {
            slot.generation = freed.generation + 1;
            slot.state = None;
        }

        self.allocator.release(Ticket {
            id: freed.id,
            generation: freed.generation,
        })
    }

    pub(super) fn lookup(&self, key: &Key<A>) -> Option<&AssetState<A::Built>> {
        if let Some(slot) = self.items.get(key.id) {
            if slot.generation == key.generation {
                return Some(
                    slot.state.as_ref().unwrap_or(&AssetState::Pending),
                );
            } else if slot.generation > key.generation {
                return None;
            }
        }

        Some(&AssetState::Pending)
    }

    pub(super) fn set(&mut self, key: &Key<A>, state: AssetState<A::Built>) {
        let slot = self.ensure(key.id);

        debug_assert!(key.generation <= slot.generation);
        if key.generation < slot.generation {
            return;
        }

        slot.state = Some(state);
    }
}

impl<A: Asset> ErasedStore for Store<A> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn free(&mut self, freed: Freed) {
        self.free(freed)
    }
}

struct Slot<A: Asset> {
    generation: u64,
    state: Option<AssetState<A::Built>>,
}

impl<A: Asset> Slot<A> {
    fn new() -> Self {
        Self {
            generation: 0,
            state: None,
        }
    }
}

#[non_exhaustive]
pub enum AssetState<T> {
    Failed(Arc<AssetError>),
    Pending,
    Ready(T),
}

impl<T> AssetState<T> {
    pub fn status(&self) -> AssetStatus {
        match self {
            AssetState::Failed(err) => AssetStatus::Failed(err.clone()),
            AssetState::Pending => AssetStatus::Pending,
            AssetState::Ready(_) => AssetStatus::Ready,
        }
    }
}

#[non_exhaustive]
pub enum AssetStatus {
    Failed(Arc<AssetError>),
    Pending,
    Ready,
}

pub struct Handle<A: Asset>(pub(super) Arc<Guard<A>>);

impl<A: Asset> Handle<A> {
    pub fn downgrade(&self) -> WeakHandle<A> {
        WeakHandle(self.0.key)
    }

    pub(super) fn key(&self) -> Key<A> {
        self.0.key
    }
}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<A: Asset> Eq for Handle<A> {}

impl<A: Asset> Hash for Handle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.key.hash(state);
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.key.eq(&other.0.key)
    }
}

pub struct WeakHandle<A: Asset>(pub(super) Key<A>);

impl<A: Asset> WeakHandle<A> {
    pub(super) fn key(&self) -> Key<A> {
        self.0
    }
}

impl<A: Asset> Clone for WeakHandle<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Asset> Copy for WeakHandle<A> {}

impl<A: Asset> Eq for WeakHandle<A> {}

impl<A: Asset> Hash for WeakHandle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<A: Asset> PartialEq for WeakHandle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<A: Asset> From<&Handle<A>> for WeakHandle<A> {
    fn from(handle: &Handle<A>) -> Self {
        Self(handle.key())
    }
}

pub(super) struct Key<T> {
    pub(super) generation: u64,
    pub(super) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Key<T> {
    pub(super) fn new(id: usize, generation: u64) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Key<T> {}

impl<T> Eq for Key<T> {}

impl<T> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.generation.hash(state);
    }
}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.generation == other.generation
    }
}

pub(super) struct Guard<T> {
    id: TypeId,
    pub(super) key: Key<T>,
    tx: Sender<Freed>,
}

impl<T> Guard<T> {
    pub(super) fn new(id: TypeId, key: Key<T>, tx: Sender<Freed>) -> Self {
        Self { id, key, tx }
    }
}

impl<T> Drop for Guard<T> {
    fn drop(&mut self) {
        let _ = self.tx.send(Freed {
            type_id: self.id,
            type_name: type_name::<T>(),
            id: self.key.id,
            generation: self.key.generation,
        });
    }
}

pub(super) struct Freed {
    pub(super) type_id: TypeId,
    pub(super) type_name: &'static str,
    pub(super) id: usize,
    pub(super) generation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetLabel, GameAsset};

    struct TestAsset;
    impl GameAsset for TestAsset {
        type Built = u32;
    }

    struct OtherAsset;
    impl GameAsset for OtherAsset {
        type Built = u32;
    }

    /// Built form whose drop is observable through `Arc::strong_count`.
    struct DropAsset;
    impl GameAsset for DropAsset {
        type Built = Arc<()>;
    }

    fn store<A: Asset>() -> (Arc<Allocator>, Store<A>) {
        let alloc = Arc::new(Allocator::default());
        (Arc::clone(&alloc), Store::new(alloc))
    }

    /// Mints a key the same way `Registry::load` does.
    fn key<A: Asset>(alloc: &Allocator) -> Key<A> {
        let t = alloc.alloc();
        Key::new(t.id, t.generation)
    }

    fn freed<A: Asset>(k: &Key<A>) -> Freed {
        Freed {
            type_id: TypeId::of::<A>(),
            type_name: type_name::<A>(),
            id: k.id,
            generation: k.generation,
        }
    }

    fn failure() -> AssetError {
        AssetError::Panicked {
            asset: AssetLabel::from("test"),
            message: "boom".into(),
        }
    }

    // --- lookup ------------------------------------------------------------

    #[test]
    fn lookup_past_end_is_pending() {
        let (alloc, s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);

        assert!(s.items.is_empty(), "lookup must not need a slot");
        assert!(matches!(s.lookup(&k), Some(AssetState::Pending)));
    }

    #[test]
    fn existing_slot_with_no_state_is_pending() {
        let (alloc, mut s) = store::<TestAsset>();
        let k0 = key::<TestAsset>(&alloc);
        let k1 = key::<TestAsset>(&alloc);

        // Arriving for id 1 creates slot 0 as a side effect.
        s.set(&k1, AssetState::Ready(1));
        assert_eq!(s.items.len(), 2);
        assert!(matches!(s.lookup(&k0), Some(AssetState::Pending)));
    }

    #[test]
    fn set_ready_then_lookup() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);

        s.set(&k, AssetState::Ready(7));
        assert!(matches!(s.lookup(&k), Some(AssetState::Ready(7))));
    }

    #[test]
    fn set_failed_then_lookup() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);

        s.set(&k, AssetState::Failed(Arc::new(failure())));
        assert!(matches!(s.lookup(&k), Some(AssetState::Failed(_))));
    }

    #[test]
    fn set_is_lazy_and_grows_to_fit() {
        let (_alloc, mut s) = store::<TestAsset>();

        s.set(&Key::new(4, 0), AssetState::Ready(4));
        assert_eq!(s.items.len(), 5);
        assert!(matches!(
            s.lookup(&Key::new(4, 0)),
            Some(AssetState::Ready(4))
        ));
    }

    // --- free --------------------------------------------------------------

    #[test]
    fn free_makes_old_key_not_found() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);
        s.set(&k, AssetState::Ready(1));

        s.free(freed(&k));
        assert!(s.lookup(&k).is_none());
    }

    #[test]
    fn free_releases_id_with_next_generation() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);
        s.set(&k, AssetState::Ready(1));

        s.free(freed(&k));
        let next = key::<TestAsset>(&alloc);
        assert_eq!(next.id, k.id);
        assert_eq!(next.generation, k.generation + 1);
    }

    #[test]
    fn free_before_any_result_creates_slot_and_releases() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);

        s.free(freed(&k));
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].generation, k.generation + 1);
        assert!(s.lookup(&k).is_none());

        let next = key::<TestAsset>(&alloc);
        assert_eq!((next.id, next.generation), (k.id, k.generation + 1));
    }

    #[test]
    fn free_drops_built_value() {
        let (alloc, mut s) = store::<DropAsset>();
        let k = key::<DropAsset>(&alloc);
        let marker = Arc::new(());

        s.set(&k, AssetState::Ready(Arc::clone(&marker)));
        assert_eq!(Arc::strong_count(&marker), 2);

        s.free(freed(&k));
        assert_eq!(Arc::strong_count(&marker), 1, "built value leaked");
    }

    #[test]
    fn double_free_is_noop() {
        let (alloc, mut s) = store::<TestAsset>();
        let k = key::<TestAsset>(&alloc);

        s.free(freed(&k));
        s.free(freed(&k));
        assert_eq!(s.items[0].generation, k.generation + 1);

        // If the id had been released twice, both allocs would get it.
        let a = key::<TestAsset>(&alloc);
        let b = key::<TestAsset>(&alloc);
        assert_ne!(a.id, b.id);
    }

    // --- stale messages and reuse -----------------------------------------

    #[test]
    fn result_after_free_is_ignored() {
        let (alloc, mut s) = store::<DropAsset>();
        let k = key::<DropAsset>(&alloc);
        let marker = Arc::new(());

        s.free(freed(&k));
        s.set(&k, AssetState::Ready(Arc::clone(&marker)));

        assert!(s.lookup(&k).is_none());
        assert!(s.items[0].state.is_none());
        assert_eq!(Arc::strong_count(&marker), 1, "stale result was kept");
    }

    #[test]
    fn reused_id_new_key_pending_old_key_gone() {
        let (alloc, mut s) = store::<TestAsset>();
        let old = key::<TestAsset>(&alloc);
        s.set(&old, AssetState::Ready(1));
        s.free(freed(&old));

        let new = key::<TestAsset>(&alloc);
        assert_eq!(new.id, old.id);
        assert!(matches!(s.lookup(&new), Some(AssetState::Pending)));
        assert!(s.lookup(&old).is_none());
    }

    #[test]
    fn reused_id_ready_is_not_visible_through_old_key() {
        let (alloc, mut s) = store::<TestAsset>();
        let old = key::<TestAsset>(&alloc);
        s.free(freed(&old));

        let new = key::<TestAsset>(&alloc);
        s.set(&new, AssetState::Ready(2));
        assert!(matches!(s.lookup(&new), Some(AssetState::Ready(2))));
        assert!(s.lookup(&old).is_none());
    }

    #[test]
    fn stale_free_does_not_clear_reused_slot() {
        let (alloc, mut s) = store::<TestAsset>();
        let old = key::<TestAsset>(&alloc);
        s.free(freed(&old));

        let new = key::<TestAsset>(&alloc);
        s.set(&new, AssetState::Ready(2));

        s.free(freed(&old));
        assert!(matches!(s.lookup(&new), Some(AssetState::Ready(2))));

        // And the live id was not handed out again.
        let other = key::<TestAsset>(&alloc);
        assert_ne!(other.id, new.id);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn set_ahead_of_slot_generation_panics_in_debug() {
        let (_alloc, mut s) = store::<TestAsset>();
        // Generation 1 for an id the allocator never released: impossible
        // in practice, so the debug assertion must catch it.
        s.set(&Key::new(0, 1), AssetState::Ready(1));
    }

    // --- guard -> freed -> store round trip -------------------------------

    #[test]
    fn guard_sends_freed_only_when_last_handle_drops() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let k = Key::<TestAsset>::new(3, 7);
        let h = Handle(Arc::new(Guard::new(TypeId::of::<TestAsset>(), k, tx)));
        let h2 = h.clone();

        drop(h);
        assert!(rx.try_recv().is_err(), "freed sent while a clone was alive");

        drop(h2);
        let f = rx.try_recv().expect("no freed message");
        assert_eq!(f.type_id, TypeId::of::<TestAsset>());
        assert_eq!((f.id, f.generation), (3, 7));
        assert!(rx.try_recv().is_err(), "freed sent more than once");
    }

    #[test]
    fn dropped_handle_frees_and_recycles_through_store() {
        let (alloc, mut s) = store::<TestAsset>();
        let (tx, rx) = crossbeam_channel::unbounded();
        let k = key::<TestAsset>(&alloc);
        let h = Handle(Arc::new(Guard::new(TypeId::of::<TestAsset>(), k, tx)));
        s.set(&k, AssetState::Ready(1));

        drop(h);
        s.free(rx.try_recv().expect("no freed message"));

        assert!(s.lookup(&k).is_none());
        let next = key::<TestAsset>(&alloc);
        assert_eq!((next.id, next.generation), (k.id, k.generation + 1));
    }

    // --- Stores ------------------------------------------------------------

    fn store_mut<A: Asset>(stores: &mut Stores) -> &mut Store<A> {
        stores
            .get_erased(&TypeId::of::<A>())
            .expect("store not initialised")
            .as_any_mut()
            .downcast_mut::<Store<A>>()
            .expect(INVARIANT)
    }

    #[test]
    fn stores_route_by_type() {
        let mut stores = Stores::default();
        let a = Arc::new(Allocator::default());
        let b = Arc::new(Allocator::default());
        stores.init::<TestAsset>(Arc::clone(&a));
        stores.init::<OtherAsset>(Arc::clone(&b));

        let ka = key::<TestAsset>(&a);
        let kb = key::<OtherAsset>(&b);
        assert_eq!(ka.id, kb.id, "separate types share id values");

        store_mut::<TestAsset>(&mut stores).set(&ka, AssetState::Ready(1));
        store_mut::<OtherAsset>(&mut stores).set(&kb, AssetState::Ready(2));

        assert!(matches!(stores.get_built(&ka), Some(AssetState::Ready(1))));
        assert!(matches!(stores.get_built(&kb), Some(AssetState::Ready(2))));
    }

    #[test]
    fn stores_init_twice_keeps_existing_store() {
        let mut stores = Stores::default();
        let alloc = Arc::new(Allocator::default());
        stores.init::<TestAsset>(Arc::clone(&alloc));

        let k = key::<TestAsset>(&alloc);
        store_mut::<TestAsset>(&mut stores).set(&k, AssetState::Ready(5));

        stores.init::<TestAsset>(Arc::new(Allocator::default()));
        assert!(matches!(stores.get_built(&k), Some(AssetState::Ready(5))));
    }

    #[test]
    fn stores_erased_free_reaches_typed_store() {
        let mut stores = Stores::default();
        let alloc = Arc::new(Allocator::default());
        stores.init::<TestAsset>(Arc::clone(&alloc));

        let k = key::<TestAsset>(&alloc);
        store_mut::<TestAsset>(&mut stores).set(&k, AssetState::Ready(1));

        stores
            .get_erased(&TypeId::of::<TestAsset>())
            .unwrap()
            .free(freed(&k));
        assert!(stores.get_built(&k).is_none());
    }

    #[test]
    #[should_panic(expected = "different registry")]
    fn stores_get_built_for_unregistered_type_panics() {
        let stores = Stores::default();
        let _ = stores.get_built(&Key::<TestAsset>::new(0, 0));
    }
}
