use std::{fs, path::PathBuf};

use crate::assets::{Asset, AssetError, SourceFor};

pub struct Texture;
impl Asset for Texture {}

pub struct TextureDefinition {
    pub depth: Option<u32>,
    pub dimension: TextureDimension,
    pub source: PathBuf,
}

impl SourceFor<Texture> for TextureDefinition {
    type Raw = (PathBuf, Option<u32>, TextureDimension, Vec<u8>);

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        fs::read(self.source.as_path())
            .map(|v| (self.source, self.depth, self.dimension, v))
            .map_err(|err| AssetError::Other {
                source: Box::new(err),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}
