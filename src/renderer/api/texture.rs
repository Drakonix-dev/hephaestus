use std::path::PathBuf;

use crate::define_handle;

// TextureHandle defines a handle for a specific texture.
define_handle!(TextureHandle);

pub struct TextureDefinition {
    pub depth: Option<u32>,
    pub dimension: TextureDimension,
    pub source: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}
