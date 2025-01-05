// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

use common_macros::hash_map;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, Read, Write};

use crate::shadertoy;

lazy_static! {
    static ref BUILTIN_MODULES: std::collections::HashMap<&'static str, &'static str> = {
        hash_map! {
            "sdf::op" => include_str!("sdf_op.wgsl"),
            "sdf3d::normal" => include_str!("sdf3d_normal.wgsl"),
            "sdf3d::primitives" => include_str!("sdf3d_primitives.wgsl"),
        }
    };
}

/// The output is wrapped in a Result to allow matching on errors
/// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P: AsRef<std::path::Path>>(
    filename: P,
) -> std::io::Result<Lines<BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(std::io::BufReader::new(file).lines())
}

type ModuleHandlers = HashMap<String, Box<dyn Fn(&mut dyn Write) -> std::io::Result<()>>>;

/// Our shader
#[derive(Default)]
pub struct Sdf3DShader {
    /// Source code of the shader
    source: String,
    /// Handlers to import modules
    modules: ModuleHandlers,
}

impl Sdf3DShader {
    /// Construct a new `Sdf3DShader` from a path
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Self {
        let mut s = Self {
            source: String::new(),
            modules: HashMap::new(),
        };

        s.add_module("sdf::*", |w| write!(w, "{}", BUILTIN_MODULES["sdf::op"]))
            .add_module("sdf::op", |w| write!(w, "{}", BUILTIN_MODULES["sdf::op"]))
            .add_module("sdf3d::normal", |w| {
                write!(w, "{}", BUILTIN_MODULES["sdf3d::normal"])
            })
            .add_module("sdf3d::primitives", |w| {
                write!(w, "{}", BUILTIN_MODULES["sdf3d::primitives"])
            })
            .add_module("sdf3d::*", |w| {
                write!(w, "{}", BUILTIN_MODULES["sdf3d::primitives"])?;
                write!(w, "{}", BUILTIN_MODULES["sdf3d::normal"])?;
                Ok(())
            });

        s.source = s.shader_source(&path);

        s
    }

    /// Construct a new `Sdf3DShader` from glsl shader
    ///
    /// * `path`: Path of the GLSL
    /// * `sdf`: Function name of the SDF function, e.g. `float sdf(vec3)`
    pub fn from_glsl_fragment_shader(
        path: impl AsRef<std::path::Path>,
        sdf: &str,
    ) -> Result<Self, shadertoy::ShaderProcessingError> {
        use shadertoy::*;
        let mut file = File::open(path).unwrap();

        let mut glsl = String::new();
        file.read_to_string(&mut glsl).unwrap();

        let mut wgsl = WgslShaderCode::from_glsl(&glsl)?;
        wgsl.remove_function("fn main_1(")?;
        wgsl.remove_function("fn main(")?;
        wgsl.remove_line("@fragment"); // Remove fragment entry point
        wgsl.add_line(include_str!("sdf3d_normal.wgsl"));

        if wgsl.has_function(sdf) {
            if !wgsl.has_function("sdf3d") {
                // Generate function wrapper
                wgsl.add_line(
                    format!("fn sdf3d(p: vec3<f32>) -> f32 {{ return {}(p); }}", sdf).as_str(),
                );
            }
        } else {
            return Err(shadertoy::ShaderProcessingError::MissingSdf(sdf.into()));
        }

        Ok(Self {
            source: wgsl.to_string(),
            modules: HashMap::new(),
        })
    }

    /// Construct a new Sdf3DShader from the ShaderToy API
    ///
    /// * `shader_id`: Id of the ShaderToy shader, e.g. DldfR7
    /// * `sdf`: Function name of the SDF function, e.g. `float sdf(vec3)`
    pub async fn from_shadertoy_api(
        shader_id: &str,
        sdf: &str,
    ) -> Result<Self, shadertoy::ShaderProcessingError> {
        let shader = shadertoy::Shader::from_api(shader_id).await?;
        log::info!("Shader: {}", shader.info.name);
        log::info!("Shader author: {}", shader.info.username);

        let mut wgsl = shader.generate_wgsl_shader_code()?;

        wgsl.remove_function("fn main_1(")?;
        wgsl.remove_function("fn main(")?;

        wgsl.remove_function("fn mainImage(")?;
        wgsl.remove_line("@fragment"); // Remove fragment entry point

        if wgsl.has_function(sdf) {
            if !wgsl.has_function("sdf3d") {
                // Generate function wrapper
                wgsl.add_line(
                    format!("fn sdf3d(p: vec3<f32>) -> f32 {{ return {}(p); }}", sdf).as_str(),
                );
            }
        } else {
            return Err(shadertoy::ShaderProcessingError::MissingSdf(sdf.into()));
        }

        // Add function to compute the normal
        wgsl.add_line(include_str!("sdf3d_normal.wgsl"));

        Ok(Self {
            source: wgsl.to_string(),
            modules: HashMap::new(),
        })
    }

    fn add_module(
        &mut self,
        name: &str,
        module: impl Fn(&mut dyn Write) -> std::io::Result<()> + 'static,
    ) -> &mut Self {
        self.modules.insert(name.to_string(), Box::new(module));
        self
    }

    pub fn add_to_source(&mut self, source: &str) {
        self.source += source;
    }

    fn shader_source_input(
        &self,
        path: impl AsRef<std::path::Path>,
        w: &mut dyn Write,
    ) -> std::io::Result<()> {
        match read_lines(&path) {
            Ok(lines) => {
                for line in lines.flatten() {
                    let trimmed = line.trim();

                    if trimmed.ends_with(';') {
                        if trimmed.starts_with("use") {
                            let modulename = trimmed
                                .replacen("use", "", 1)
                                .replace(['\"', ';'], "")
                                .trim()
                                .to_string();
                            log::info!("{modulename}");
                            if self.modules.contains_key(&modulename) {
                                self.modules.get(&modulename).unwrap()(w)?;
                            }
                            continue;
                        } else if trimmed.starts_with("include") {
                            let filename = std::path::PathBuf::from(
                                trimmed
                                    .replacen("include", "", 1)
                                    .replace(['\"', ';'], "")
                                    .trim(),
                            );
                            if filename != path.as_ref() {
                                self.shader_source_input(filename, w)?;
                            }
                            continue;
                        }
                    }
                    writeln!(w, "{}", line)?;
                }
            }
            Err(err) => {
                log::error!("Could not include {:?}: {}", path.as_ref(), err);
            }
        }

        Ok(())
    }

    /// Write shader to file
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let mut f = std::io::BufWriter::new(File::create(path)?);
        write!(f, "{}", self.source)
    }

    /// Return shader source as string
    fn shader_source(&self, path: impl AsRef<std::path::Path>) -> String {
        let mut w = Vec::new();
        let _ = self.shader_source_input(path, &mut w);

        std::str::from_utf8(w.as_slice()).unwrap().to_string()
    }

    /// Create a `wgpu::ShaderModule`
    pub fn create_shader_module(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(self.source.as_str())),
        })
    }
}
