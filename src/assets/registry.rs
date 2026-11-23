use std::{
    any::{TypeId, type_name},
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use crossbeam_channel::{Receiver, Sender};

use crate::assets::{
    Asset, AssetConfig, AssetLabel, AssetReader, Descriptor, ErasedLoader,
    ErasedStore, FileReader, Handle, INVARIANT, InitError, LoadError, LoadJob,
    Loader, LoaderCell, Pool, Priority, Readers, Route, SlotState, Store,
    SubmitFn,
};

pub struct Registry {
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
                    panic!("no store for {}: {}", queued.type_name, INVARIANT)
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

type ApplyFn = Box<dyn FnOnce(&mut dyn ErasedStore) + Send>;

struct QueuedAsset {
    apply: ApplyFn,
    type_id: TypeId,
    type_name: &'static str,
}
