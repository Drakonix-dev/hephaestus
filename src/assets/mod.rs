use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
    collections::HashMap,
    env,
    error::Error,
    fs,
    marker::PhantomData,
    path::{Component, Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
};

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, select_biased};

pub(crate) use private::*;

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

const INVARIANT: &str = "asset type-erasure invariant violated";

pub struct AssetConfig {
    /// Root for the default `file://` reader. Relative paths resolve against
    /// CARGO_MANIFEST_DIR in debug builds and the executable's dir in release.
    pub root: PathBuf,
    /// Threads that run blocking reads.
    pub io_threads: u8,
    /// Threads that run CPU-side parsing, `None` = available_parallelism - 1 (min 1).
    pub parse_threads: Option<usize>,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("assets"),
            io_threads: 4,
            parse_threads: None,
        }
    }
}

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
    #[error("invalid argument: {msg}")]
    InvalidArgument { msg: &'static str },

    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: &'static str },

    #[error("operation failed")]
    OperationFailed {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("asset {t} cannot be handled for descriptor {d}")]
    UnhandledAsset { d: &'static str, t: &'static str },

    #[error("asset type unknown: {t}")]
    UnknownAsset { t: &'static str },

    #[error("asset path scheme unknown: {s}")]
    UnknownScheme { s: String },
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

impl<T: Asset> Copy for Handle<T> {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath {
    pub scheme: Cow<'static, str>,
    pub path: Cow<'static, str>,
}

impl From<&'static str> for AssetPath {
    fn from(s: &'static str) -> Self {
        match s.split_once("://") {
            Some((scheme, path)) => Self {
                scheme: scheme.into(),
                path: path.into(),
            },
            None => Self {
                scheme: "file".into(),
                path: s.into(),
            },
        }
    }
}

pub trait AssetReader: Send + Sync + 'static {
    fn read(&self, path: &str) -> Result<Bytes, AssetError>;
}

pub enum AssetData {
    Inline(Bytes),
    Path(AssetPath),
}

pub struct AssetKey(u64);

pub trait Descriptor: Send + 'static {
    fn take_external(&mut self) -> Option<AssetData> {
        None
    }

    fn key(&self) -> Option<AssetKey> {
        None
    }

    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

type BuildFn = Box<
    dyn FnOnce(&dyn Any, &mut dyn ErasedStore) -> Result<(), AssetError> + Send,
>;

trait ErasedLoader: Send + Sync + 'static {
    fn parse(
        self: Arc<Self>,
        desc: Box<dyn Any + Send>,
        bytes: Option<Bytes>,
    ) -> Result<BuildFn, AssetError>;
}

pub trait Loader<A: Asset>: Send + Sync + 'static {
    type Desc: Descriptor;
    type Parsed: Send + 'static;

    fn build(
        &self,
        desc: &Self::Desc,
        parsed: Self::Parsed,
    ) -> Result<A::Built, AssetError>;
    fn parse(
        &self,
        desc: &Self::Desc,
        bytes: Option<Bytes>,
    ) -> Result<Self::Parsed, AssetError>;
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
    fn parse(
        self: Arc<Self>,
        desc: Box<dyn Any + Send>,
        bytes: Option<Bytes>,
    ) -> Result<BuildFn, AssetError> {
        let desc = desc.downcast::<L::Desc>().expect(INVARIANT);
        let parsed = self.0.parse(&desc, bytes)?;

        Ok(Box::new(move |handle, store| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(&desc, parsed)?;

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

type ApplyFn =
    Box<dyn FnOnce(&mut dyn ErasedStore) -> Result<(), AssetError> + Send>;

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
    readers: Readers,
    stores: HashMap<TypeId, Box<dyn ErasedStore>>,
}

impl Registry {
    fn new(cfg: &AssetConfig) -> Result<Self, AssetError> {
        let (qtx, qrx) = crossbeam_channel::unbounded();

        let mut readers = Readers::new();
        readers.register("file", FileReader::new(&cfg.root)?);

        Ok(Self {
            loaders: HashMap::new(),
            pool: Pool::new(cfg),
            qrx,
            qtx,
            readers,
            stores: HashMap::new(),
        })
    }

    fn load<A: Asset, D: Descriptor>(
        &mut self,
        mut desc: D,
        priority: Priority,
    ) -> Result<Handle<A>, AssetError> {
        let asset_key = TypeId::of::<A>();
        let key = (asset_key, TypeId::of::<D>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(AssetError::UnhandledAsset {
                d: type_name::<D>(),
                t: type_name::<A>(),
            })?
            .clone();

        // TODO: lookup descriptor by key first and return handle if exists

        let store = self
            .stores
            .entry(asset_key)
            .or_insert_with(|| Box::new(Store::<A>::new()));

        let (id, generation) = store.insert_pending();
        let handle = Handle::new(id, generation);

        let route = Route::resolve(desc.take_external(), &self.readers)?;
        let job = LoadJob {
            desc: Box::new(desc),
            loader,
            submit: self.get_submit_fn(handle),
        };

        self.pool.submit(route, job, priority);

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

    pub fn register_reader(
        &mut self,
        scheme: impl Into<Cow<'static, str>>,
        reader: impl AssetReader,
    ) {
        self.readers.register(scheme, reader)
    }

    pub fn register_loader<A, D, L>(&mut self, loader: L)
    where
        A: Asset,
        D: Descriptor,
        L: Loader<A, Desc = D>,
    {
        self.loaders
            .entry((TypeId::of::<A>(), TypeId::of::<D>()))
            .or_insert_with(|| Arc::new(LoaderCell::<A, L>::new(loader)));
    }

    // ------------------------------------------------------------------------

    fn get_submit_fn<A: Asset>(&self, handle: Handle<A>) -> SubmitFn {
        let tx = self.qtx.clone();

        Box::new(move |result| {
            let apply: ApplyFn = Box::new(move |store| match result {
                Err(err) => Err(err),
                Ok(build) => build(&handle, store),
            });

            let _ = tx.send(QueuedAsset {
                apply,
                type_id: TypeId::of::<A>(),
                type_name: type_name::<A>(),
            });
        })
    }
}

type FetchFn = Box<dyn FnOnce() -> Result<Bytes, AssetError> + Send>;

enum Route {
    Fetch(FetchFn),
    Ready(Option<Bytes>),
}

impl Route {
    fn resolve(
        data: Option<AssetData>,
        readers: &Readers,
    ) -> Result<Self, AssetError> {
        Ok(match data {
            None => Route::Ready(None),
            Some(AssetData::Inline(bytes)) => Route::Ready(Some(bytes)),
            Some(AssetData::Path(path)) => {
                let reader = readers.get(&path.scheme)?;
                Route::Fetch(Box::new(move || reader.read(&path.path)))
            }
        })
    }
}

struct Readers(HashMap<Cow<'static, str>, Arc<dyn AssetReader>>);

impl Readers {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn get(&self, scheme: &str) -> Result<Arc<dyn AssetReader>, AssetError> {
        self.0
            .get(scheme)
            .cloned()
            .ok_or_else(|| AssetError::UnknownScheme {
                s: scheme.to_owned(),
            })
    }

    fn register(
        &mut self,
        scheme: impl Into<Cow<'static, str>>,
        reader: impl AssetReader,
    ) {
        self.0.insert(scheme.into(), Arc::new(reader));
    }
}

type SubmitFn = Box<dyn FnOnce(Result<BuildFn, AssetError>) + Send>;

struct LoadJob {
    desc: Box<dyn Any + Send>,
    loader: Arc<dyn ErasedLoader>,
    submit: SubmitFn,
}

impl LoadJob {
    fn run(self, bytes: Result<Option<Bytes>, AssetError>) {
        let LoadJob {
            desc,
            loader,
            submit,
        } = self;
        submit(bytes.and_then(|b| loader.parse(desc, b)));
    }
}

type Job = Box<dyn FnOnce() + Send>;

struct Pool {
    // ORDER MATTERS: the senders must drop before workers joins.
    io_tx: Sender<Job>,
    p_txs: [Sender<Job>; TOTAL_PRIORITIES],
    workers: Workers,
}

impl Pool {
    fn new(cfg: &AssetConfig) -> Self {
        let mut handles = Vec::new();

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        for _ in 0..cfg.io_threads {
            handles.push(Pool::spawn_io_worker(io_rx.clone()));
        }

        let (pc_tx, pc_rx) = crossbeam_channel::unbounded();
        let (pi_tx, pi_rx) = crossbeam_channel::unbounded();
        let (ps_tx, ps_rx) = crossbeam_channel::unbounded();
        let p_txs = [pc_tx, pi_tx, ps_tx];

        let num_threads = cfg.parse_threads.unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(1)
        });

        for _ in 0..num_threads {
            let p_rxs = [pc_rx.clone(), pi_rx.clone(), ps_rx.clone()];
            handles.push(Pool::spawn_parse_worker(p_rxs));
        }

        Self {
            io_tx,
            p_txs,
            workers: Workers(handles),
        }
    }

    fn submit(&self, route: Route, job: LoadJob, priority: Priority) {
        let ptx = self.p_txs[priority as usize].clone();

        match route {
            Route::Fetch(fetch) => {
                let _ = self.io_tx.send(Box::new(move || {
                    let bytes = fetch().map(Some);
                    let _ = ptx.send(Box::new(|| job.run(bytes)));
                }));
            }
            Route::Ready(bytes) => {
                let _ = ptx.send(Box::new(|| job.run(Ok(bytes))));
            }
        }
    }

    // ------------------------------------------------------------------------

    fn spawn_io_worker(rx: Receiver<Job>) -> JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                (job)()
            }
        })
    }

    fn spawn_parse_worker(
        rxs: [Receiver<Job>; TOTAL_PRIORITIES],
    ) -> JoinHandle<()> {
        thread::spawn(move || {
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

struct Workers(Vec<JoinHandle<()>>);

impl Drop for Workers {
    fn drop(&mut self) {
        for h in self.0.drain(..) {
            let _ = h.join();
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

struct FileReader {
    root: PathBuf,
}

impl FileReader {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, AssetError> {
        let root = resolve_root(root.as_ref());
        let root =
            root.canonicalize()
                .map_err(|e| AssetError::OperationFailed {
                    source: format!("asset root {}: {e}", root.display())
                        .into(),
                })?;

        Ok(Self { root })
    }
}

impl AssetReader for FileReader {
    fn read(&self, path: &str) -> Result<Bytes, AssetError> {
        let rel = Path::new(path);
        if !rel
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(AssetError::InvalidArgument {
                msg: "asset path must be relative and stay instide the asset root",
            });
        }

        fs::read(self.root.join(rel)).map(Bytes::from).map_err(|e| {
            AssetError::OperationFailed {
                source: format!("{path}: {e}").into(),
            }
        })
    }
}

fn resolve_root(root: &Path) -> PathBuf {
    if root.is_absolute() {
        return root.to_path_buf();
    }

    if let Some(dir) = option_env!("CARGO_MANIFEST_DIR") {
        return Path::new(dir).join(root);
    }

    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(root)))
        .unwrap_or_else(|| root.to_path_buf())
}
