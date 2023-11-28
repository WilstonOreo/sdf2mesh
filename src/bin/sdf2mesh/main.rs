use std::io::Write;
extern crate sdf2mesh;

use encase::ShaderType;
use sdf2mesh::{
    mesh::{TriangleMesh, VertexList},
    Bounds3D, Vec3D,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author = "Michael Winkelmann", version, about = "sdf2mesh")]
struct Arguments {
    /// Input SDF file
    #[arg(short = 'i', long)]
    sdf: String,

    /// Recipient TOML file (optional)
    #[arg(short = 'o', long)]
    mesh: String,

    /// Optional latex output file
    #[arg(long)]
    debug_png: String,

    /// Grid resolution. Default: 256x256x256
    #[arg(short = 'd', long)]
    resolution: Option<u32>,
}

#[derive(Debug, ShaderType, Clone, Copy)]
struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Debug, ShaderType, Clone, Copy)]
struct Dims {
    x: u32,
    y: u32,
    z: u32,
    z_slice_idx: u32,
}

#[derive(Debug, ShaderType)]
struct AppState {
    pub bb_min: Vec4,
    pub bb_max: Vec4,
    pub dims: Dims,
}

impl AppState {
    // Translating Rust structures to WGSL is always tricky and can prove
    // incredibly difficult to remember all the rules by which WGSL
    // lays out and formats structs in memory. It is also often extremely
    // frustrating to debug when things don't go right.
    //
    // You may sometimes see structs translated to bytes through
    // using `#[repr(C)]` on the struct so that the struct has a defined,
    // guaranteed internal layout and then implementing bytemuck's POD
    // trait so that one can preform a bitwise cast. There are issues with
    // this approach though as C's struct layouts aren't always compatible
    // with WGSL, such as when special WGSL types like vec's and mat's
    // get involved that have special alignment rules and especially
    // when the target buffer is going to be used in the uniform memory
    // space.
    //
    // Here though, we use the encase crate which makes translating potentially
    // complex Rust structs easy through combined use of the [`ShaderType`] trait
    // / derive macro and the buffer structs which hold data formatted for WGSL
    // in either the storage or uniform spaces.
    fn as_wgsl_bytes(&self) -> encase::internal::Result<Vec<u8>> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self)?;
        Ok(buffer.into_inner())
    }

    fn set_z(&mut self, z_slice_idx: u32) {
        let z_size = self.bb_max.z - self.bb_min.z;
        self.dims.z_slice_idx = z_slice_idx;
        self.bb_min.w = (z_slice_idx as f32) / (self.dims.z as f32) * z_size + self.bb_min.z;
    }
}

impl Default for AppState {
    fn default() -> Self {
        let bounds = Bounds3D::centered(&Vec3D::new(2.0, 2.0, 2.0));

        let min = bounds.min().to_f32();
        let max = bounds.max().to_f32();
        AppState {
            bb_min: Vec4 {
                x: min.x,
                y: min.y,
                z: min.z,
                /* z */ w: 0.0,
            },
            bb_max: Vec4 {
                x: max.x,
                y: max.y,
                z: max.z,
                /* eps */ w: 0.0001,
            },
            dims: Dims {
                x: 128,
                y: 128,
                z: 128,
                z_slice_idx: 0,
            },
        }
    }
}

struct Rgba32FloatTextureStorage {
    data: Vec<f32>,
    dims: (usize, usize),
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    binding_id: u32,
}

impl Rgba32FloatTextureStorage {
    fn new(device: &wgpu::Device, dims: (usize, usize), binding_id: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: dims.0 as u32,
                height: dims.1 as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let data = vec![0.0_f32; dims.0 * dims.1 * 4];
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

    fn get_rgba(&self, x: u32, y: u32) -> (f32, f32, f32, f32) {
        let idx = ((y * (self.dims.0 as u32) + x) * 4) as usize;
        (
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        )
    }

    fn bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
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

    fn bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: self.binding_id,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }

    fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
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
                    bytes_per_row: Some((self.dims.0 * 16) as u32),
                    rows_per_image: Some(self.dims.1 as u32),
                },
            },
            wgpu::Extent3d {
                width: self.dims.0 as u32,
                height: self.dims.1 as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    async fn map_buffer(&mut self, device: &wgpu::Device) {
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

trait ToPngImage {
    fn to_png_image(&self, path: impl AsRef<std::path::Path>);
}

impl ToPngImage for Rgba32FloatTextureStorage {
    fn to_png_image(&self, path: impl AsRef<std::path::Path>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let image_data = self
                .data
                .iter()
                .map(|f| (*f * 127.0 + 128.0).clamp(0.0, 255.0) as u8)
                .collect();
            output_image_native(image_data, self.dims, path);
        }
        #[cfg(target_arch = "wasm32")]
        output_image_wasm(self.data.to_vec(), self.dims);
    }
}

/// Replaces the site body with a message telling the user to open the console and use that.
pub fn output_image_native(
    image_data: Vec<u8>,
    texture_dims: (usize, usize),
    path: impl AsRef<std::path::Path>,
) {
    let mut png_data = Vec::<u8>::with_capacity(image_data.len());
    let mut encoder = png::Encoder::new(
        std::io::Cursor::new(&mut png_data),
        texture_dims.0 as u32,
        texture_dims.1 as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    let mut png_writer = encoder.write_header().unwrap();
    png_writer.write_image_data(&image_data[..]).unwrap();
    png_writer.finish().unwrap();
    log::info!("PNG file encoded in memory.");

    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(&png_data[..]).unwrap();
    log::info!("PNG file written to disc as \"{:?}\".", &path.as_ref());
}

async fn run(_path: Option<String>) {
    let mut state = AppState::default();

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let mut normal_texture =
        Rgba32FloatTextureStorage::new(&device, (state.dims.x as usize, state.dims.y as usize), 1);
    let mut position_texture =
        Rgba32FloatTextureStorage::new(&device, (state.dims.x as usize, state.dims.y as usize), 2);

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            normal_texture.bind_group_layout_entry(),
            position_texture.bind_group_layout_entry(),
        ],
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<AppState>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            normal_texture.bind_group_entry(),
            position_texture.bind_group_entry(),
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    log::info!("Wgpu context set up.");

    let mut vertex_items = VertexList::default();

    //----------------------------------------
    for z_slice_idx in 0..state.dims.z {
        state.set_z(z_slice_idx);
        queue.write_buffer(
            &uniform_buffer,
            0,
            &state
                .as_wgsl_bytes()
                .expect("Error in encase translating AppState struct to WGSL bytes."),
        );

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.set_pipeline(&pipeline);
            compute_pass.dispatch_workgroups(state.dims.x, state.dims.z, 1);
        }

        normal_texture.copy_texture_to_buffer(&mut command_encoder);
        position_texture.copy_texture_to_buffer(&mut command_encoder);

        normal_texture.map_buffer(&device).await;
        position_texture.map_buffer(&device).await;

        for y in 0..state.dims.y {
            for x in 0..state.dims.x {
                use sdf2mesh::Vertex;

                let p = position_texture.get_rgba(x, y);

                if p.3 > 0.0 {
                    let n = normal_texture.get_rgba(x, y);
                    let vertex = Vertex {
                        normal: Vec3D::new(n.0 as f64, n.1 as f64, n.2 as f64),
                        pos: Vec3D::new(p.0 as f64, p.1 as f64, p.2 as f64),
                    };
                    let cell = (x as u16, y as u16, z_slice_idx as u16);
                    let s = n.3 as u32;
                    let sign_changes = (s & 1 != 0, s & 2 != 0, s & 4 != 0, s & 8 != 0);

                    vertex_items.insert(cell, sign_changes, vertex);
                }
            }
        }

        //normal_texture.to_png_image(format!("{path}{idx:04}_normal.png", path = _path.as_ref().unwrap(), idx = z_slice_idx));
        //position_texture.to_png_image(format!("{path}{idx:04}_position.png", path = _path.as_ref().unwrap(), idx = z_slice_idx));

        queue.submit(Some(command_encoder.finish()));
    }
    log::info!("Have {} vertices.", vertex_items.len());
    let vertices = vertex_items.fetch_vertices();
    let triangle_indices = vertex_items.fetch_triangle_indices();

    let mesh = TriangleMesh {
        vertices,
        triangle_indices,
    };
    mesh.write_ply_to_file("shader_test.ply")
        .expect("Could not write PLY file!");

    log::info!("Done.")
}

pub fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .format_timestamp_nanos()
            .init();

        let path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "please_don't_git_push_me.png".to_string());
        pollster::block_on(run(Some(path)));
    }
}
