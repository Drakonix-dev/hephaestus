use std::path::PathBuf;

use image::{ImageBuffer, ImageReader, Rgba};

use crate::assets::{Asset, AssetError, SourceFor};

pub struct Texture;
impl Asset for Texture {}

pub struct TextureDefinition {
    pub depth: Option<u32>,
    pub dimension: TextureDimension,
    pub source: PathBuf,
}

impl SourceFor<Texture> for TextureDefinition {
    type Raw = ImageBuffer<Rgba<u8>, Vec<u8>>;

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        let image = ImageReader::open(self.source.as_path())?
            .decode()
            .map_err(|err| AssetError::OperationFailed {
                source: Box::new(err),
            })?;
        Ok(image.to_rgba8())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}
