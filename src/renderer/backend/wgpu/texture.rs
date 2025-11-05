use std::collections::HashMap;

use image::ImageReader;

use crate::renderer::{TextureDefinition, TextureDimension, TextureHandle};

pub(crate) struct TextureManager {
    textures: HashMap<TextureHandle, TextureInstance>,
}

pub(crate) struct TextureInstance {
    pub(crate) view: wgpu::TextureView,
}

impl TextureManager {
    pub(crate) fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub(crate) fn create_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        handle: TextureHandle,
        def: &TextureDefinition,
    ) {
        let image = ImageReader::open(def.source.as_path())
            .expect("failed to open image")
            .decode()
            .expect("failed to read image");
        let rgba = image.to_rgb8();

        use image::GenericImageView;
        let (width, height) = image.dimensions();

        let (dimension, depth_or_layers) = match def.dimension {
            TextureDimension::D1 => (wgpu::TextureDimension::D1, 1),
            TextureDimension::D2 => (wgpu::TextureDimension::D2, 1),
            TextureDimension::D3 => (wgpu::TextureDimension::D3, def.depth.unwrap()),
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture"), 
            size: wgpu::Extent3d {
                width: width,
                height: height,
                depth_or_array_layers: depth_or_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: dimension,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,  
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
              offset: 0,
              bytes_per_row: Some(4 * width),
              rows_per_image: Some(height),
            },
            texture.size(),
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.textures.insert(handle, TextureInstance {
            view: texture_view, 
        });
    }

    pub(crate) fn get_texture(&self, handle: &TextureHandle) -> Option<&TextureInstance> {
        self.textures.get(handle)
    }
}
