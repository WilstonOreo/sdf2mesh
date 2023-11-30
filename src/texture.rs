// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::png;

pub struct Rgba32FloatTextureStorage {
    data: Vec<f32>,
    dims: (u32, u32),
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    binding_id: u32,
}

impl Rgba32FloatTextureStorage {
    pub fn new(device: &wgpu::Device, dims: (u32, u32), binding_id: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: dims.0,
                height: dims.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let data = vec![0.0_f32; (dims.0 as usize) * (dims.1 as usize) * 4];
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of_val(&data[..]) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            data,
            dims,
            texture,
            view,
            buffer,
            binding_id,
        }
    }

    pub fn get_rgba(&self, x: u32, y: u32) -> (f32, f32, f32, f32) {
        let idx = ((y * self.dims.0 + x) * 4) as usize;
        (
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        )
    }

    pub fn bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding_id,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba32Float,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: self.binding_id,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }

    pub fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    // This needs to be padded to 256.
                    bytes_per_row: Some(self.dims.0 * 16),
                    rows_per_image: Some(self.dims.1),
                },
            },
            wgpu::Extent3d {
                width: self.dims.0,
                height: self.dims.1,
                depth_or_array_layers: 1,
            },
        );
    }

    pub async fn map_buffer(&mut self, device: &wgpu::Device) {
        let buffer_slice = self.buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
        device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.unwrap().unwrap();
        {
            let view = buffer_slice.get_mapped_range();
            let byte_slice = &view[..];
            self.data = Vec::from(bytemuck::cast_slice(byte_slice));
        }
        self.buffer.unmap();
    }
}

impl png::ToPngFile for Rgba32FloatTextureStorage {
    fn to_png_file(&self, path: impl AsRef<std::path::Path>) {
        let image_data = self
            .data
            .iter()
            .map(|f| (*f * 127.0 + 128.0).clamp(0.0, 255.0) as u8)
            .collect();
        png::image_data_to_file(image_data, self.dims, path);
    }
}
