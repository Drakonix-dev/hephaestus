use std::{
    error::Error,
    sync::{
        Arc,
        mpsc::{self, Receiver as SReceiver, Sender as SSender},
    },
    thread::{self, JoinHandle},
};

use crossbeam_channel::{Receiver, Sender};

use crate::{
    assets::{
        Asset, AssetError, Handle, SourceFor, loader::ErasedLoader, registry::ErasedRegistry,
    },
    config::EngineConfig,
    events::EventBus,
};

pub enum Priority {
    Critical,
    Streaming,
    Idle,
}

struct MainThreadJob {
    err: Option<Box<dyn Error + Send + Sync>>,
    job: Option<Box<dyn FnOnce() + Send>>,
}

impl MainThreadJob {
    fn new(job: Box<dyn FnOnce() + Send>) -> Self {
        Self {
            err: None,
            job: Some(job),
        }
    }

    fn err(err: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            err: Some(err),
            job: None,
        }
    }
}

pub(crate) struct Pool {
    events: Arc<EventBus>,
    handles: Vec<JoinHandle<()>>,
    io_tx: Sender<IOJob>,
    rx: SReceiver<MainThreadJob>,
    tx: SSender<MainThreadJob>,
}

impl Pool {
    pub(crate) fn new(cfg: &EngineConfig, events: Arc<EventBus>) -> Self {
        let mut handles = Vec::new();
        let (tx, rx) = mpsc::channel();

        let (io_tx, io_rx) = crossbeam_channel::bounded(cfg.io_threads as usize);
        for _ in 0..cfg.io_threads {
            handles.push(IOWorker::spawn(io_rx.clone()));
        }

        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        for _ in 0..num_threads {
            let handle = thread::spawn(|| {});
            handles.push(handle);
        }

        Self {
            events,
            handles,
            io_tx,
            rx,
            tx,
        }
    }

    pub(crate) fn close(&mut self) {
        for h in self.handles.drain(..) {
            h.join().unwrap();
        }
    }

    pub(crate) fn load<A: Asset, S: SourceFor<A> + Send>(
        &mut self,
        handle: Handle<A>,
        src: S,
        _priority: Priority,
        loader: Arc<dyn ErasedLoader>,
        reg: &mut dyn ErasedRegistry,
    ) {
        let tx = self.tx.clone();

        let job = Box::new(move || match src.fetch() {
            Ok(raw) => {
                let parse_job = Box::new(move || {
                    let main_job = match loader.parse(Box::new(raw)) {
                        Ok(build) => MainThreadJob::new(Box::new(move || {
                            if let Err(err) = (build)(&handle, reg) {
                                tx.send(MainThreadJob::err(Box::new(err)))
                                    .expect("Failed to emit error");
                            }
                        })),
                        Err(err) => MainThreadJob::err(Box::new(err)),
                    };

                    tx.send(main_job).expect("Failed to emit error");
                });
                todo!("send parse job");
            }
            Err(err) => tx
                .send(MainThreadJob::err(Box::new(err)))
                .expect("Failed to emit error"),
        });
        self.io_tx.send(job);

        // let raw = src.fetch()?;
        // let build = loader.parse(Box::new(raw));
        // (build)(&handle, reg)
    }
}

type IOJob = Box<dyn FnOnce() + Send>;

struct IOWorker {
    rx: Receiver<IOJob>,
}

impl IOWorker {
    fn spawn(rx: Receiver<IOJob>) -> JoinHandle<()> {
        let worker = Self { rx };
        thread::spawn(|| worker.work())
    }

    fn work(self) {
        while let Ok(job) = self.rx.recv() {
            (job)()
        }
    }
}

type ParseJob = Box<dyn FnOnce() -> MainThreadJob + Send>;

struct ParseWorker {
    rx: Receiver<ParseJob>,
    tx: Sender<MainThreadJob>,
}

impl ParseWorker {
    fn spawn(rx: Receiver<ParseJob>, tx: Sender<MainThreadJob>) -> JoinHandle<()> {
        let worker = Self { rx, tx };
        thread::spawn(|| worker.work())
    }

    fn work(self) {
        while let Ok(job) = self.rx.recv() {
            let main_job = (job)();
            if let Err(err) = self.tx.send(main_job) {}
        }
    }
}
