use std::sync::mpsc::{self, Receiver, Sender};

use crate::{config::WindowMode, renderer::PresentMode};

pub(crate) enum EngineCommand {
    SetPresentMode(PresentMode),
    SetWindowMode(WindowMode),
}

pub(crate) struct EngineCommandQueue;

impl EngineCommandQueue {
    pub(crate) fn new() -> (EngineCommandWriter, EngineCommandReader) {
        let (tx, rx) = mpsc::channel();
        (EngineCommandWriter { tx }, EngineCommandReader { rx })
    }
}

#[derive(Clone)]
pub(crate) struct EngineCommandWriter {
    tx: Sender<EngineCommand>,
}

impl EngineCommandWriter {
    pub(crate) fn push(&self, cmd: EngineCommand) {
        self.tx
            .send(cmd)
            .expect("pushed engine command after queue reader dropped");
    }
}

pub(crate) struct EngineCommandReader {
    rx: Receiver<EngineCommand>,
}

impl EngineCommandReader {
    pub(crate) fn drain(&self) -> Vec<EngineCommand> {
        let mut cmds = Vec::new();

        while let Ok(cmd) = self.rx.try_recv() {
            cmds.push(cmd);
        }

        cmds
    }
}
