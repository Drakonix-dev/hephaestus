use crate::{channels::define_channel, config::WindowMode, renderer::PresentMode};

pub(crate) enum EngineCommand {
    RequestExit,
    SetPresentMode(PresentMode),
    SetTickRate(u32),
    SetWindowMode(WindowMode),
}

define_channel!(EngineCommand, EngineCommand);
