use std::sync::Arc;

use bytes::Bytes;
use image::{ImageBuffer, ImageReader, Rgba};

use crate::{
    assets::{AssetError, BuiltAsset, Loader},
    renderer::{Texture, TextureDefinition, TextureDimension},
};

impl BuiltAsset for Texture {
    type Built = TextureInstance;
}

pub(crate) struct TextureInstance {
    pub(crate) view: wgpu::TextureView,
}

pub(crate) struct TextureLoader {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl TextureLoader {
    pub(crate) fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
    ) -> Self {
        Self { device, queue }
    }
}

impl Loader<Texture> for TextureLoader {
    type Desc = TextureDefinition;
    type Parsed = ImageBuffer<Rgba<u8>, Vec<u8>>;

    fn build(
        &self,
        desc: &TextureDefinition,
        parsed: Self::Parsed,
    ) -> Result<TextureInstance, AssetError> {
        let (width, height) = parsed.dimensions();

        let (dimension, depth_or_layers) = match desc.dimension {
            TextureDimension::D1 => (wgpu::TextureDimension::D1, 1),
            TextureDimension::D2 => (wgpu::TextureDimension::D2, 1),
            TextureDimension::D3 => (
                wgpu::TextureDimension::D3,
                src.depth.ok_or_else(|| AssetError::BadAsset {
                    reason: "No depth for D3 texture",
                })?,
            ),
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(src.source.to_str().unwrap_or("unknown_texture")),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: depth_or_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &parsed,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture.size(),
        );

        Ok(TextureInstance {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
        })
    }

    fn parse(
        &self,
        _: &TextureDefinition,
        bytes: Option<Bytes>,
    ) -> Result<Self::Parsed, AssetError> {
        match bytes {
            Some(bytes) => {
                let image = ImageReader::from(bytes).decode();
            }
            None => Err(AssetError::InvalidArgument {
                msg: "expected bytes",
            }),
        }
    }
}
