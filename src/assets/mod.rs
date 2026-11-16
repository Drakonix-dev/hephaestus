use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    error::Error,
    marker::PhantomData,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

use crossbeam_channel::select_biased;

use crate::config::EngineConfig;
pub(crate) use private::*;

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

const INVARIANT: &str = "asset type-erasure invariant violated";

pub trait Asset: private::BuiltAsset {}

pub trait GameAsset: 'static {
    type Built: 'static;
}

impl<T: GameAsset> private::BuiltAsset for T {
    type Built = <T as GameAsset>::Built;
}

impl<T: GameAsset> Asset for T {}

pub enum AssetStatus {
    Failed(String),
    Pending,
    Ready,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: &'static str },

    #[error("asset {t} cannot be handled for source {s}")]
    UnhandledAsset { t: &'static str, s: &'static str },

    #[error("asset type unknown: {t}")]
    UnknownAsset { t: &'static str },
}

impl AssetError {
    fn not_found<A: Asset>(handle: &Handle<A>) -> Self {
        Self::NotFound {
            id: handle.id,
            t: type_name::<A>(),
        }
    }
}

pub enum Priority {
    Critical,
    Idle,
    Streaming,
}

const TOTAL_PRIORITIES: usize = Priority::Streaming as usize + 1;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T: Asset> {
    pub(crate) generation: u64,
    pub(crate) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Handle<T> {
    pub(crate) fn new(id: usize, generation: u64) -> Self {
        Self {
            generation,
            id,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

pub struct SourceId<'a>(pub &'a str);

pub trait Source: 'static {
    type Output;

    fn id(&self) -> SourceId;
    fn read(&self) -> Result<Self::Output, AssetError>;
}

#[macro_export]
macro_rules! asset_source {
    ($t:ty, $id_body:block) => {
        impl crate::assets::Source for $t {
            type Output = ();
            fn id(&self) -> crate::assets::SourceId $id_body
            fn read(&self) -> Result<Self::Output, crate::assets::AssetError> {
                Ok(())
            }
        }
    };
}

type BuildFn =
    Box<dyn FnOnce(&dyn Any, &mut dyn ErasedStore) -> Result<(), AssetError>>;

trait ErasedLoader {
    fn parse(self: Self, raw: Box<dyn Any>) -> Result<BuildFn, AssetError>;
}

pub trait Loader<A: Asset>: 'static {
    type Parsed: 'static;
    type Raw;

    fn build(&self, parsed: Self::Parsed) -> Result<A::Built, AssetError>;
    fn parse(&self, raw: Self::Raw) -> Result<Self::Parsed, AssetError>;
}

struct LoaderCell<A, L>(L, PhantomData<fn() -> A>);

impl<A, L> LoaderCell<A, L> {
    fn new(l: L) -> Self {
        Self(l, PhantomData)
    }
}

impl<A, L> ErasedLoader for LoaderCell<A, L>
where
    A: Asset,
    L: Loader<A>,
{
    fn parse(self: Self, raw: Box<dyn Any>) -> Result<BuildFn, AssetError> {
        let raw = raw.downcast::<L::Raw>().expect(INVARIANT);
        let parsed = self.0.parse(*raw)?;

        Ok(Box::new(move |handle, store| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(parsed)?;

            store
                .as_any_mut()
                .downcast_mut::<Store<A>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built))
        }))
    }
}

struct Slot<T> {
    generation: u64,
    state: SlotState<T>,
}

impl<T> Slot<T> {
    fn new(state: SlotState<T>) -> Self {
        Self {
            generation: 0,
            state,
        }
    }
}

enum SlotState<T> {
    Failed(Box<dyn Error + Send + Sync>),
    Pending,
    Ready(T),
}

impl<T> From<&SlotState<T>> for AssetStatus {
    fn from(state: &SlotState<T>) -> Self {
        match state {
            SlotState::Failed(err) => AssetStatus::Failed(err.to_string()),
            SlotState::Pending => AssetStatus::Pending,
            SlotState::Ready(_) => AssetStatus::Ready,
        }
    }
}

type ApplyFn = Box<dyn FnOnce(&mut dyn ErasedStore) -> Result<(), AssetError>>;

struct QueuedAsset {
    apply: ApplyFn,
    type_id: TypeId,
    type_name: &'static str,
}

struct Registry {
    loaders: HashMap<(TypeId, TypeId), Arc<dyn ErasedLoader>>,
    pool: Pool,
    qrx: Receiver<QueuedAsset>,
    qtx: Sender<QueuedAsset>,
    stores: HashMap<TypeId, Box<dyn ErasedStore>>,
}

impl Registry {
    fn new(cfg: &EngineConfig) -> Self {
        let (qtx, qrx) = mpsc::channel();

        Self {
            loaders: HashMap::new(),
            pool: Pool::new(cfg),
            qrx,
            qtx,
            stores: HashMap::new(),
        }
    }

    fn load<A: Asset, S: Source>(
        &mut self,
        src: S,
    ) -> Result<Handle<A>, AssetError> {
        let asset_key = TypeId::of::<A>();
        let key = (asset_key, TypeId::of::<S::Output>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<A>(),
                s: type_name::<S>(),
            })?
            .clone();

        let store = self
            .stores
            .entry(asset_key)
            .or_insert_with(|| Box::new(Store::<A>::new()));

        let (id, generation) = store.insert_pending();
        let handle = Handle::new(id, generation);

        let raw = src.read()?;
        let build = loader.parse(Box::new(raw))?;
        build(&handle, store.as_mut())?;
        Ok(handle)
    }

    fn process_queued_assets(&mut self) -> Result<(), AssetError> {
        while let Ok(queued) = self.qrx.try_recv() {
            let store = self.stores.get_mut(&queued.type_id).ok_or(
                AssetError::UnknownAsset {
                    t: queued.type_name,
                },
            )?;

            (queued.apply)(store.as_mut())?;
        }

        Ok(())
    }

    // ------------------------------------------------------------------------

    fn prepare_submit_fn<A: Asset>(&self, handle: Handle<A>) -> SubmitFn {
        let tx = self.qtx.clone();
        let get_apply = Box::new(|result| {
            Box::new(|store| {
                if let Ok(build) = result {
                    build(&handle, store)
                }
            })
        });

        Box::new(|result| {
            let _ = tx.send(QueuedAsset {
                apply: get_apply(result),
                type_id: TypeId::of::<A>(),
                type_name: type_name::<A>(),
            });
        })
    }
}

type Job = Box<dyn FnOnce()>;

type SubmitFn = Box<dyn FnOnce(Result<BuildFn, AssetError>)>;

struct Pool {
    handles: Vec<JoinHandle<()>>,
    io_tx: crossbeam_channel::Sender<Job>,
    p_txs: [crossbeam_channel::Sender<Job>; TOTAL_PRIORITIES],
}

impl Pool {
    fn new(cfg: &EngineConfig) -> Self {
        let mut handles = Vec::new();

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        for _ in 0..cfg.io_threads {
            handles.push(Pool::spawn_io_worker(io_rx.clone()));
        }

        let (pc_tx, pc_rx) = crossbeam_channel::unbounded();
        let (pi_tx, pi_rx) = crossbeam_channel::unbounded();
        let (ps_tx, ps_rx) = crossbeam_channel::unbounded();
        let p_txs = [pc_tx, pi_tx, ps_tx];

        let num_threads = thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);

        for _ in 0..num_threads {
            let p_rxs = [pc_rx.clone(), pi_rx.clone(), ps_rx.clone()];
            handles.push(Pool::spawn_parse_worker(p_rxs));
        }

        Self {
            handles,
            io_tx,
            p_txs,
        }
    }

    fn load<S: Source>(
        &self,
        src: &S,
        priority: Priority,
        loader: Arc<dyn ErasedLoader>,
        submit: SubmitFn,
    ) {
        let ptx = self.p_txs[priority as usize].clone();
        let _ = self.io_tx.send(Box::new(|| match src.read() {
            Ok(raw) => {
                let _ =
                    ptx.send(Box::new(|| submit(loader.parse(Box::new(raw)))));
            }
            Err(e) => submit(Err(e)),
        }));
    }

    // ------------------------------------------------------------------------

    fn spawn_io_worker(rx: crossbeam_channel::Receiver<Job>) -> JoinHandle<()> {
        thread::spawn(|| {
            while let Ok(job) = rx.recv() {
                (job)()
            }
        })
    }

    fn spawn_parse_worker(
        rxs: [crossbeam_channel::Receiver<Job>; TOTAL_PRIORITIES],
    ) -> JoinHandle<()> {
        thread::spawn(|| {
            loop {
                select_biased!(
                    recv(rxs[Priority::Critical as usize]) -> job => match job {
                        Ok(job) => job(),
                        Err(_) => break,
                    },
                    recv(rxs[Priority::Streaming as usize]) -> job => match job {
                        Ok(job) => job(),
                        Err(_) => break,
                    },
                    recv(rxs[Priority::Idle as usize]) -> job => match job {
                        Ok(job) => job(),
                        Err(_) => break,
                    },
                )
            }
        })
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        drop(self.io_tx);
        drop(self.p_txs);

        for h in self.handles {
            h.join().unwrap();
        }
    }
}

trait ErasedStore: Any {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn insert_pending(&mut self) -> (usize, u64);
}

struct Store<A: Asset> {
    free: Vec<usize>,
    items: Vec<Slot<A::Built>>,
}

impl<A: Asset> Store<A> {
    fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
        }
    }

    fn get(
        &self,
        handle: &Handle<A>,
    ) -> Result<&SlotState<A::Built>, AssetError> {
        if handle.id >= self.items.len() {
            return Err(AssetError::not_found(handle));
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return Err(AssetError::not_found(handle));
        }

        Ok(&slot.state)
    }

    fn insert(&mut self, state: SlotState<A::Built>) -> Handle<A> {
        if let Some(id) = self.free.pop() {
            Handle::new(id, self.items[id].generation)
        } else {
            self.items.push(Slot::new(state));
            Handle::new(self.items.len() - 1, 0)
        }
    }

    fn update(
        &mut self,
        handle: Handle<A>,
        state: SlotState<A::Built>,
    ) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;
        self.items[handle.id].state = state;
        Ok(())
    }
}

impl<A: Asset> ErasedStore for Store<A> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn insert_pending(&mut self) -> (usize, u64) {
        let handle = self.insert(SlotState::Pending);
        (handle.id, handle.generation)
    }
}
