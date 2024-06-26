// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

extern crate sdf2mesh;

use encase::ShaderType;
use sdf2mesh::{png::ToPngFile, *};

use clap::Parser;
use shader::Sdf3DShader;

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
        self.dims.z_slice_idx = z_slice_idx;
    }
}

impl Default for AppState {
    fn default() -> Self {
        let bounds = Bounds3D::centered(&Vec3D::new(2.0, 2.0, 2.0));

        let min = bounds.min();
        let max = bounds.max();
        AppState {
            bb_min: Vec4 {
                x: min.x,
                y: min.y,
                z: min.z,
                /* unused */ w: 0.0,
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

#[derive(Parser, Debug)]
#[command(author = "Michael Winkelmann", version, about = "sdf2mesh")]
struct Arguments {
    /// Input SDF file
    #[arg(short = 'i', long)]
    sdf: Option<String>,

    /// Input ShaderToy shader ID
    #[arg(long)]
    shadertoy_id: Option<String>,

    /// ShaderToy SDF name
    #[arg(long, default_value = "sdf")]
    shadertoy_sdf: Option<String>,

    /// Input GLSL fragment shader
    #[arg(long)]
    glsl: Option<String>,

    /// GLSL SDF name
    #[arg(long, default_value = "sdf")]
    glsl_sdf: Option<String>,

    /// Output mesh file (supports STL and PLY output)
    #[arg(short = '0', long)]
    mesh: String,

    /// Output WGSL file for debugging
    #[arg(long)]
    debug_wgsl: Option<String>,

    /// Write PNG images for debugging
    #[arg(long)]
    debug_png: Option<String>,

    /// Grid resolution. Default: 256
    #[arg(short = 'r', long)]
    resolution: Option<u32>,

    /// Size of bounding box. Default: 2
    #[arg(short = 'b', long)]
    bounds: Option<f32>,
}

impl From<&Arguments> for AppState {
    fn from(args: &Arguments) -> Self {
        let mut res = args.resolution.unwrap_or(256);
        if res.count_ones() > 1 {
            res = 2 << res.ilog2();
            log::warn!(
                "Resolution should be a power of 2 (actual resolution : {})",
                res
            );
        }

        let bounds = Bounds3D::cube(args.bounds.unwrap_or(2.0), &Vec3D::zero());
        let min = bounds.min();
        let max = bounds.max();

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
                x: res,
                y: res,
                z: res,
                z_slice_idx: 0u32,
            },
        }
    }
}

async fn run(args: Arguments) {
    let mut state = AppState::from(&args);

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let mut sdf3d_file = Sdf3DShader::default();

    if let Some(shadertoy_id) = &args.shadertoy_id {
        log::info!("Reading SDF from ShaderToy (shader ID {})", {
            shadertoy_id
        });
        sdf3d_file = shader::Sdf3DShader::from_shadertoy_api(
            shadertoy_id,
            args.shadertoy_sdf.unwrap_or("sdf".into()).as_str(),
        )
        .await
        .unwrap();
    } else if let Some(sdf) = &args.sdf {
        log::info!("Reading SDF from {}...", sdf);

        sdf3d_file = shader::Sdf3DShader::from_path(sdf);
    } else if let Some(glsl) = &args.glsl {
        log::info!("Reading SDF from GLSL fragment shader {}...", glsl);
        sdf3d_file = shader::Sdf3DShader::from_glsl_fragment_shader(
            glsl,
            args.glsl_sdf.unwrap_or("sdf".into()).as_str(),
        )
        .unwrap();
    }

    if let Some(debug_wgsl) = &args.debug_wgsl {
        sdf3d_file.write_to_file(debug_wgsl).unwrap();
    }

    sdf3d_file.add_to_source(include_str!("dualcontour.wgsl"));

    let shader = sdf3d_file.create_shader_module(&device);

    let mut normal_texture =
        texture::Rgba32FloatTextureStorage::new(&device, (state.dims.x, state.dims.y), 1);
    let mut position_texture =
        texture::Rgba32FloatTextureStorage::new(&device, (state.dims.x, state.dims.y), 2);

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
        compilation_options: Default::default(),
    });

    log::info!("Wgpu context set up.");

    let mut vertex_items =
        mesh::VertexList::with_capacity(state.dims.x as usize * state.dims.y as usize);

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
                let p = position_texture.get_rgba(x, y);

                if p.3 > 0.0 {
                    let n = normal_texture.get_rgba(x, y);
                    let vertex = Vertex {
                        normal: Vec3D::new(n.0, n.1, n.2),
                        pos: Vec3D::new(p.0, p.1, p.2),
                    };
                    let cell = (x as u16, y as u16, z_slice_idx as u16);
                    let s = n.3 as u32;
                    let sign_changes = (s & 1 != 0, s & 2 != 0, s & 4 != 0, s & 8 != 0);

                    vertex_items.insert(cell, sign_changes, vertex);
                }
            }
        }

        if let Some(path) = &args.debug_png {
            normal_texture.to_png_file(format!("{path}{z_slice_idx:04}_normal.png"));
            position_texture.to_png_file(format!("{path}{z_slice_idx:04}_position.png"));
        }

        if z_slice_idx % 128 == 0 {
            log::info!("Slice #{}", z_slice_idx);
        }

        queue.submit(Some(command_encoder.finish()));
    }
    log::info!("Mesh has {} vertices.", vertex_items.len());

    if let Err(err) = mesh::TriangleMesh::from(vertex_items).write_to_file(&args.mesh) {
        log::error!("Could not write mesh to {}!", err);
    }

    log::info!("Mesh written to {}", args.mesh)
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    pollster::block_on(run(args));
}
