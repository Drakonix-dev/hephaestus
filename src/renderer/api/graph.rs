// RenderPhase defines a phase of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPhase {
    pub domain: RenderDomain,
    pub subphase: Option<SubPhase>,
}

impl RenderPhase {
    pub fn new(domain: RenderDomain, subphase: Option<SubPhase>) -> Self {
        Self { domain, subphase }
    }
}

// RenderDomain defines the different domains of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDomain {
    UI,
    World2D,
    World3D,
    Other(u32),
}

// SubPhase defines the different sub-phases of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubPhase {
    Opaque,
    Transparent,
    Other(u32),
}
