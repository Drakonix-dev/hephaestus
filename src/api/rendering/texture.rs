use std::path::Path;

use crate::define_handle;

// TextureHandle defines a handle for a specific texture.
define_handle!(TextureHandle);

pub struct TextureDefinition<'a> {
    pub depth: Option<u32>,
    pub dimension: TextureDimension,
    pub source: &'a Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}
