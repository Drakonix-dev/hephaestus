use std::path::Path;

use crate::{define_handle, define_wrapper};

// TextureHandle defines a handle for a specific texture.
define_handle!(TextureHandle);

define_wrapper!(TextureManager, TextureDevice, {
    fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle;
});

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
