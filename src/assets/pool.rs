use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

use crossbeam_channel::{Receiver, Sender, select_biased};

use crate::{
    assets::{
        Asset, AssetError, Priority, SourceFor, TOTAL_PRIORITIES,
        graph::Deps,
        loader::{BuildFn, ErasedLoader},
    },
    config::EngineConfig,
};

pub(crate) type SubmitFn = Box<dyn FnOnce(Result<(Deps, BuildFn), AssetError>) + Send>;

pub(crate) struct Pool {
    handles: Vec<JoinHandle<()>>,
    io_tx: Sender<IOJob>,
    p_txs: [Sender<ParseJob>; TOTAL_PRIORITIES],
}

impl Pool {
    pub(crate) fn new(cfg: &EngineConfig) -> Self {
        let mut handles = Vec::new();

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        for _ in 0..cfg.io_threads {
            handles.push(IOWorker::spawn(io_rx.clone()));
        }

        let (pc_tx, pc_rx) = crossbeam_channel::unbounded();
        let (ps_tx, ps_rx) = crossbeam_channel::unbounded();
        let (pi_tx, pi_rx) = crossbeam_channel::unbounded();
        let p_txs = [pc_tx, ps_tx, pi_tx];

        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        for _ in 0..num_threads {
            let p_rxs = [pc_rx.clone(), ps_rx.clone(), pi_rx.clone()];
            handles.push(ParseWorker::spawn(p_rxs));
        }

        Self {
            handles,
            io_tx,
            p_txs,
        }
    }

    pub(crate) fn close(self) {
        drop(self.io_tx);
        drop(self.p_txs);

        for h in self.handles {
            h.join().unwrap();
        }
    }

    pub(crate) fn load<A: Asset, S: SourceFor<A>>(
        &mut self,
        src: Arc<S>,
        priority: Priority,
        loader: Arc<dyn ErasedLoader>,
        submit: SubmitFn,
    ) {
        let ptx = self.p_txs[priority as usize].clone();

        let job = Box::new(move || match src.fetch() {
            Ok(raw) => {
                let _ = ptx.send(Box::new(move || {
                    let mut deps = Deps::new();
                    let res = loader
                        .parse(src, Box::new(raw), &mut deps)
                        .map(|build| (deps, build));
                    submit(res)
                }));
            }
            Err(e) => submit(Err(e)),
        });
        let _ = self.io_tx.send(job);
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

type ParseJob = Box<dyn FnOnce() + Send>;

struct ParseWorker {
    rxs: [Receiver<ParseJob>; TOTAL_PRIORITIES],
}

impl ParseWorker {
    fn spawn(rxs: [Receiver<ParseJob>; TOTAL_PRIORITIES]) -> JoinHandle<()> {
        let worker = Self { rxs };
        thread::spawn(|| worker.work())
    }

    fn work(self) {
        loop {
            select_biased!(
                recv(self.rxs[Priority::Critical as usize]) -> job => match job {
                    Ok(job) => job(),
                    Err(_) => break,
                },
                recv(self.rxs[Priority::Streaming as usize]) -> job => match job {
                    Ok(job) => job(),
                    Err(_) => break,
                },
                recv(self.rxs[Priority::Idle as usize]) -> job => match job {
                    Ok(job) => job(),
                    Err(_) => break,
                },
            );
        }
    }
}
