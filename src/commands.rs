use std::time;

use crate::{channels::define_channel, config::WindowMode, renderer::PresentMode};

pub enum EngineCommand {
    RequestExit,
    SetPresentMode(PresentMode),
    SetTickFreq(time::Duration),
    SetWindowMode(WindowMode),
}

define_channel!(EngineCommand, EngineCommand);
