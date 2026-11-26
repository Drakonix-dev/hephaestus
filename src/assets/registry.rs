use std::{
    any::{TypeId, type_name},
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use crossbeam_channel::{Receiver, Sender};

use crate::assets::{
    Asset, AssetConfig, AssetData, AssetLabel, AssetReader, AssetState,
    AssetStatus, Dependencies, Descriptor, ErasedLoader, ErasedStore,
    FileReader, Freed, GameAsset, Handle, INVARIANT, InitError, Key, LoadError,
    LoadJob, Loader, LoaderCell, Pool, Priority, Readers, Route, Store, Stores,
    SubmitFn, WeakHandle, catch_panic,
};

pub struct Registry {
    deps: Dependencies,
    frx: Receiver<Freed>,
    ftx: Sender<Freed>,
    loaders: HashMap<(TypeId, TypeId), Arc<dyn ErasedLoader>>,
    pool: Pool,
    qrx: Receiver<QueuedAsset>,
    qtx: Sender<QueuedAsset>,
    readers: Readers,
    stores: Stores,
}

impl Registry {
    pub(crate) fn new(cfg: &AssetConfig) -> Result<Self, InitError> {
        let (ftx, frx) = crossbeam_channel::unbounded();
        let (qtx, qrx) = crossbeam_channel::unbounded();

        let mut readers = Readers::new();
        readers.register("file", FileReader::new(&cfg.root)?);

        Ok(Self {
            deps: Dependencies::default(),
            frx,
            ftx,
            loaders: HashMap::new(),
            pool: Pool::new(cfg),
            qrx,
            qtx,
            readers,
            stores: Stores::default(),
        })
    }

    pub fn get<A: GameAsset>(
        &self,
        handle: impl Into<WeakHandle<A>>,
    ) -> Option<&AssetState<A::Built>> {
        self.get_built(handle)
    }

    pub(crate) fn get_built<A: Asset>(
        &self,
        handle: impl Into<WeakHandle<A>>,
    ) -> Option<&AssetState<A::Built>> {
        self.stores.get_built(&handle.into().key())
    }

    pub fn load<A: Asset, D: Descriptor>(
        &mut self,
        mut desc: D,
        priority: Priority,
    ) -> Result<Handle<A>, LoadError> {
        let key = (TypeId::of::<A>(), TypeId::of::<D>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(LoadError::UnhandledAsset {
                asset: type_name::<A>(),
                desc: type_name::<D>(),
            })?
            .clone();
        let store = self.stores.get_mut();

        let label = desc.label();
        let data = desc.take_external();
        let label = label.or_else(|| match &data {
            Some(AssetData::Path(p)) => Some(AssetLabel::from(p.to_string())),
            _ => None,
        });

        let route = Route::resolve(desc.take_external(), &self.readers)?;

        let handle = store.insert(self.ftx.clone());
        let label = label.unwrap_or_else(|| {
            let key = handle.key();
            AssetLabel::from(format!(
                "{}#{}.{}",
                type_name::<A>(),
                key.id,
                key.generation,
            ))
        });

        let submit = self.get_submit_fn(handle.key(), label.clone());
        let job = LoadJob {
            desc: Box::new(desc),
            label,
            loader,
            submit,
        };

        self.pool.submit(route, job, priority);

        Ok(handle)
    }

    pub fn status<A: Asset>(
        &self,
        handle: impl Into<WeakHandle<A>>,
    ) -> Option<AssetStatus> {
        self.get_built(handle).map(|s| s.status())
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
        self.stores.init::<A>();

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

    pub(crate) fn process_queued_assets(&mut self) {
        while let Ok(freed) = self.frx.try_recv() {
            self.stores
                .get_erased(&freed.type_id)
                .unwrap_or_else(|| {
                    panic!("no store for {}: {}", freed.type_name, INVARIANT)
                })
                .free(freed);
        }

        while let Ok(queued) = self.qrx.try_recv() {
            let store =
                self.stores.get_erased(&queued.type_id).unwrap_or_else(|| {
                    panic!("no store for {}: {}", queued.type_name, INVARIANT)
                });
            (queued.apply)(store.as_mut())
        }
    }

    // ------------------------------------------------------------------------

    fn get_submit_fn<A: Asset>(
        &self,
        key: Key<A>,
        label: AssetLabel,
    ) -> SubmitFn {
        let tx = self.qtx.clone();

        Box::new(move |result| {
            let apply: ApplyFn = Box::new(move |store| {
                let store = store
                    .as_any_mut()
                    .downcast_mut::<Store<A>>()
                    .expect(INVARIANT);

                if !matches!(store.lookup(&key), Some(AssetState::Pending)) {
                    return;
                }

                let state =
                    match result.and_then(|build| catch_panic(label, build)) {
                        Ok(built) => AssetState::Ready(
                            *built.downcast::<A::Built>().expect(INVARIANT),
                        ),
                        Err(err) => AssetState::Failed(Arc::new(err)),
                    };

                store.update(&key, state)
            });

            let _ = tx.send(QueuedAsset {
                apply,
                type_id: TypeId::of::<A>(),
                type_name: type_name::<A>(),
            });
        })
    }
}

type ApplyFn = Box<dyn FnOnce(&mut dyn ErasedStore) + Send>;

struct QueuedAsset {
    apply: ApplyFn,
    type_id: TypeId,
    type_name: &'static str,
}
