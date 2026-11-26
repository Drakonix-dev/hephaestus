use std::{
    any::Any,
    sync::Arc,
    thread::{self, JoinHandle},
};

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, select_biased};

use crate::assets::{
    AssetConfig, AssetData, AssetError, AssetLabel, LoadError, Readers,
    catch_panic,
    loader::{BuildFn, ErasedLoader},
};

type Job = Box<dyn FnOnce() + Send>;

pub(super) struct Pool {
    // ORDER MATTERS: the senders must drop before workers joins.
    shutdown: Sender<()>,
    io_tx: Sender<Job>,
    p_txs: [Sender<Job>; Priority::PRIORITY_COUNT],
    workers: Workers,
}

impl Pool {
    pub(super) fn new(cfg: &AssetConfig) -> Self {
        let mut handles = Vec::new();
        let (shutdown, shutdown_rx) = crossbeam_channel::bounded::<()>(0);

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        for _ in 0..cfg.io_threads.max(1) {
            handles.push(Pool::spawn_io_worker(
                shutdown_rx.clone(),
                io_rx.clone(),
            ));
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
            handles.push(Pool::spawn_parse_worker(shutdown_rx.clone(), p_rxs));
        }

        Self {
            shutdown,
            io_tx,
            p_txs,
            workers: Workers(handles),
        }
    }

    pub(super) fn submit(
        &self,
        route: Route,
        job: LoadJob,
        priority: Priority,
    ) {
        let ptx = self.p_txs[priority as usize].clone();

        match route {
            Route::Fetch(fetch) => {
                let _ = self.io_tx.send(Box::new(move || {
                    let result =
                        catch_panic(job.label.clone(), || fetch().map(Some));
                    let _ = ptx.send(Box::new(|| job.run(result)));
                }));
            }
            Route::Ready(bytes) => {
                let _ = ptx.send(Box::new(|| job.run(Ok(bytes))));
            }
        }
    }

    // ------------------------------------------------------------------------

    fn spawn_io_worker(
        stop: Receiver<()>,
        rx: Receiver<Job>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                select_biased!(
                    recv(stop) -> _ => break,
                    recv(rx) -> job => match job {
                        Ok(job) => job(),
                        Err(_) => break,
                    },
                )
            }
        })
    }

    fn spawn_parse_worker(
        stop: Receiver<()>,
        rxs: [Receiver<Job>; Priority::PRIORITY_COUNT],
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                select_biased!(
                    recv(stop) -> _ => break,
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

type FetchFn = Box<dyn FnOnce() -> Result<Bytes, AssetError> + Send>;

pub(super) enum Route {
    Fetch(FetchFn),
    Ready(Option<Bytes>),
}

impl Route {
    pub(super) fn resolve(
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

                Route::Fetch(Box::new(move || {
                    reader
                        .read(&path.path)
                        .map_err(|source| AssetError::Read { path, source })
                }))
            }
        })
    }
}

pub(super) type SubmitFn = Box<dyn FnOnce(Result<BuildFn, AssetError>) + Send>;

pub(super) struct LoadJob {
    pub(super) desc: Box<dyn Any + Send>,
    pub(super) label: AssetLabel,
    pub(super) loader: Arc<dyn ErasedLoader>,
    pub(super) submit: SubmitFn,
}

impl LoadJob {
    fn run(self, bytes: Result<Option<Bytes>, AssetError>) {
        let LoadJob {
            desc,
            label,
            loader,
            submit,
        } = self;

        let result = catch_panic(label.clone(), || {
            bytes.and_then(|b| loader.parse(desc, label, b))
        });

        submit(result);
    }
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
