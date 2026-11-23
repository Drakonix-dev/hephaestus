use core::fmt;
use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    env,
    error::Error,
    fs,
    hash::{Hash, Hasher},
    io,
    marker::PhantomData,
    panic::{self, AssertUnwindSafe},
    path::{Component, Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
};

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, select_biased};

mod errors;
pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

pub use errors::*;
pub(crate) use private::*;

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

pub enum AssetStatus {
    Failed(Arc<AssetError>),
    Pending,
    Ready,
}

#[non_exhaustive]
pub enum Priority {
    Critical,
    Idle,
    Streaming,
}

impl Priority {
    const PRIORITY_COUNT: usize = 3;
}

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
        *self
    }
}

impl<T: Asset> Copy for Handle<T> {}

impl<T: Asset> Eq for Handle<T> {}

impl<T: Asset> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.generation.hash(state);
        self.id.hash(state);
    }
}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.generation == other.generation
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath {
    pub scheme: Cow<'static, str>,
    pub path: Cow<'static, str>,
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.path)
    }
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
    fn read(&self, path: &str) -> io::Result<Bytes>;
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

    fn label(&self) -> Option<AssetLabel> {
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
        label: AssetLabel,
        bytes: Option<Bytes>,
    ) -> Result<BuildFn, AssetError>;
}

pub trait Loader<A: Asset>: Send + Sync + 'static {
    type Desc: Descriptor;
    type Error: Error + Send + Sync + 'static;
    type Parsed: Send + 'static;

    fn build(
        &self,
        desc: &Self::Desc,
        parsed: Self::Parsed,
    ) -> Result<A::Built, Self::Error>;
    fn parse(
        &self,
        desc: &Self::Desc,
        bytes: Option<Bytes>,
    ) -> Result<Self::Parsed, Self::Error>;
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
        label: AssetLabel,
        bytes: Option<Bytes>,
    ) -> Result<BuildFn, AssetError> {
        let desc = desc.downcast::<L::Desc>().expect(INVARIANT);
        let parsed =
            self.0
                .parse(&desc, bytes)
                .map_err(|source| AssetError::Parse {
                    asset: label.clone(),
                    source: Box::new(source),
                })?;

        Ok(Box::new(move |handle, store| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(&desc, parsed).map_err(|source| {
                AssetError::Build {
                    asset: label,
                    source: Box::new(source),
                }
            })?;

            store
                .as_any_mut()
                .downcast_mut::<Store<A>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built));

            Ok(())
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
    Failed(Arc<AssetError>),
    Pending,
    Ready(T),
}

impl<T> From<&SlotState<T>> for AssetStatus {
    fn from(state: &SlotState<T>) -> Self {
        match state {
            SlotState::Failed(err) => AssetStatus::Failed(err.clone()),
            SlotState::Pending => AssetStatus::Pending,
            SlotState::Ready(_) => AssetStatus::Ready,
        }
    }
}

type ApplyFn = Box<dyn FnOnce(&mut dyn ErasedStore) + Send>;

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
    fn new(cfg: &AssetConfig) -> Result<Self, InitError> {
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
    ) -> Result<Handle<A>, LoadError> {
        let asset_key = TypeId::of::<A>();
        let key = (asset_key, TypeId::of::<D>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(LoadError::UnhandledAsset {
                asset: type_name::<A>(),
                desc: type_name::<D>(),
            })?
            .clone();

        // TODO: lookup descriptor by key first and return handle if exists

        let store = self
            .stores
            .entry(asset_key)
            .or_insert_with(|| Box::new(Store::<A>::new()));

        let label = desc.label();
        let route = Route::resolve(desc.take_external(), &self.readers)?;

        let (id, generation) = store.insert_pending();
        let handle = Handle::new(id, generation);
        let label = label.unwrap_or_else(|| {
            AssetLabel::from(format!("{}#{}", type_name::<A>(), id))
        });

        let job = LoadJob {
            desc: Box::new(desc),
            label,
            loader,
            submit: self.get_submit_fn(handle),
        };

        self.pool.submit(route, job, priority);

        Ok(handle)
    }

    fn process_queued_assets(&mut self) {
        while let Ok(queued) = self.qrx.try_recv() {
            let store =
                self.stores.get_mut(&queued.type_id).unwrap_or_else(|| {
                    panic!("no store for {}: {INVARIANT}", queued.type_name)
                });
            (queued.apply)(store.as_mut())
        }
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
        let entry = self.loaders.entry((TypeId::of::<A>(), TypeId::of::<D>()));
        match entry {
            Entry::Occupied(_) => panic!(
                "duplicate loader for asset {} and descriptor {}",
                type_name::<A>(),
                type_name::<D>()
            ),
            Entry::Vacant(e) => {
                let _ = e.insert(Arc::new(LoaderCell::new(loader)));
            }
        }
    }

    // ------------------------------------------------------------------------

    fn get_submit_fn<A: Asset>(&self, handle: Handle<A>) -> SubmitFn {
        let tx = self.qtx.clone();

        Box::new(move |result| {
            let apply: ApplyFn = Box::new(move |store| {
                let store = store
                    .as_any_mut()
                    .downcast_mut::<Store<A>>()
                    .expect(INVARIANT);

                let state = match result.and_then(|build| build(&handle, store))
                {
                    Ok(()) => return,
                    Err(err) => SlotState::Failed(Arc::new(err)),
                };

                store.update(handle, state)
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
    ) -> Result<Self, LoadError> {
        Ok(match data {
            None => Route::Ready(None),
            Some(AssetData::Inline(bytes)) => Route::Ready(Some(bytes)),
            Some(AssetData::Path(path)) => {
                let reader = readers.get(&path.scheme).ok_or(
                    LoadError::UnknownScheme {
                        path: path.path.clone().into(),
                        scheme: path.scheme.clone().into(),
                    },
                )?;

                // TODO: actually add security maybe instead of just checking path string
                if path.scheme == "file" {
                    let rel = Path::new(path.path.as_ref());
                    if !rel.components().all(|c| {
                        matches!(c, Component::Normal(_) | Component::CurDir)
                    }) {
                        return Err(LoadError::PathOutsideRoot {
                            path: rel.to_path_buf(),
                        });
                    }
                }

                Route::Fetch(Box::new(move || {
                    reader
                        .read(&path.path)
                        .map_err(|source| AssetError::Read { path, source })
                }))
            }
        })
    }
}

struct Readers(HashMap<Cow<'static, str>, Arc<dyn AssetReader>>);

impl Readers {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn get(&self, scheme: &str) -> Option<Arc<dyn AssetReader>> {
        self.0.get(scheme).cloned()
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
    label: AssetLabel,
    loader: Arc<dyn ErasedLoader>,
    submit: SubmitFn,
}

impl LoadJob {
    fn run(self, bytes: Result<Option<Bytes>, AssetError>) {
        let LoadJob {
            desc,
            label,
            loader,
            submit,
        } = self;

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            bytes.and_then(|b| loader.parse(desc, label.clone(), b))
        }))
        .unwrap_or_else(|payload| {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic".into());
            Err(AssetError::Panicked {
                asset: label,
                message,
            })
        });

        submit(result);
    }
}

type Job = Box<dyn FnOnce() + Send>;

struct Pool {
    // ORDER MATTERS: the senders must drop before workers joins.
    io_tx: Sender<Job>,
    p_txs: [Sender<Job>; Priority::PRIORITY_COUNT],
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
                    let result = panic::catch_unwind(AssertUnwindSafe(|| {
                        fetch().map(Some)
                    }))
                    .unwrap_or_else(|payload| {
                        let message = payload
                            .downcast_ref::<&str>()
                            .map(|s| s.to_string())
                            .or_else(|| {
                                payload.downcast_ref::<String>().cloned()
                            })
                            .unwrap_or_else(|| "non-string panic".into());
                        Err(AssetError::Panicked {
                            asset: job.label.clone(),
                            message,
                        })
                    });
                    let _ = ptx.send(Box::new(|| job.run(result)));
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
        rxs: [Receiver<Job>; Priority::PRIORITY_COUNT],
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

    fn get(&self, handle: &Handle<A>) -> Option<&SlotState<A::Built>> {
        if handle.id >= self.items.len() {
            return None;
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return None;
        }

        Some(&slot.state)
    }

    fn insert(&mut self, state: SlotState<A::Built>) -> Handle<A> {
        if let Some(id) = self.free.pop() {
            let slot = &mut self.items[id];
            slot.state = state;
            Handle::new(id, slot.generation)
        } else {
            self.items.push(Slot::new(state));
            Handle::new(self.items.len() - 1, 0)
        }
    }

    fn update(&mut self, handle: Handle<A>, state: SlotState<A::Built>) {
        if self.get(&handle).is_some() {
            self.items[handle.id].state = state;
        }
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
    pub fn new(root: impl AsRef<Path>) -> Result<Self, InitError> {
        let root = resolve_root(root.as_ref());
        let root = root
            .canonicalize()
            .map_err(|source| InitError::Root { path: root, source })?;

        Ok(Self { root })
    }
}

impl AssetReader for FileReader {
    fn read(&self, path: &str) -> io::Result<Bytes> {
        let path = self.root.join(Path::new(path));
        fs::read(path).map(Bytes::from)
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
