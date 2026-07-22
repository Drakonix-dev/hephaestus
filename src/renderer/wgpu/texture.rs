use std::sync::Arc;

use crate::{
    assets::{AssetError, AssetLoader, SourceFor},
    renderer::{Texture, TextureDefinition, TextureDimension},
};

pub(crate) struct TextureInstance {
    pub(crate) view: wgpu::TextureView,
}

pub(crate) struct TextureLoader {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl TextureLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self { device, queue }
    }
}

impl AssetLoader<Texture, TextureDefinition> for TextureLoader {
    type Built = TextureInstance;
    type Parsed = <TextureDefinition as SourceFor<Texture>>::Raw;

    fn parse(
        &self,
        _: &TextureDefinition,
        raw: <TextureDefinition as SourceFor<Texture>>::Raw,
    ) -> Result<Self::Parsed, AssetError> {
        Ok(raw)
    }

    fn build(
        &self,
        src: &TextureDefinition,
        parsed: Self::Parsed,
    ) -> Result<Self::Built, AssetError> {
        use image::GenericImageView;
        let (width, height) = parsed.dimensions();

        let (dimension, depth_or_layers) = match src.dimension {
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
}
