// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

use common_macros::hash_map;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, Write};

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

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P: AsRef<std::path::Path>>(
    filename: P,
) -> std::io::Result<Lines<BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(std::io::BufReader::new(file).lines())
}

type ModuleHandlers = HashMap<String, Box<dyn Fn(&mut dyn Write) -> std::io::Result<()>>>;

pub struct SDF3DShader {
    source: String,
    modules: ModuleHandlers,
}

impl SDF3DShader {
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

    pub async fn from_shadertoy_api(
        shader_id: &str,
    ) -> Result<Self, shadertoy::ShaderProcessingError> {
        let mut s = Self {
            source: String::new(),
            modules: HashMap::new(),
        };

        let shader = shadertoy::Shader::from_api(shader_id).await?;
        let mut wgsl = shader.generate_wgsl_shader_code()?;

        wgsl.remove_function("fn main_1")?;
        wgsl.remove_function("@fragment")?; // Remove main function
        wgsl.remove_function("fn mainImage(")?;

        if wgsl.has_function("normal")? {
            wgsl.rename_function("normal", "sdf3d_normal")?;
        } else {
            wgsl.add_line("use sdf3d::normal;");
            s.add_module("sdf3d::normal", |w| {
                write!(w, "{}", BUILTIN_MODULES["sdf3d::normal"])
            });
        }

        if wgsl.has_function("map")? {
            wgsl.rename_function("map", "sdf3d")?;
        } else {
            return Err(shadertoy::ShaderProcessingError::MissingSdf("map".into()));
        }

        s.source = wgsl.to_string();

        Ok(s)
    }

    pub fn add_module(
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
                eprintln!("Could not include {:?}: {}", path.as_ref(), err);
            }
        }

        Ok(())
    }

    pub fn shader_source(&self, path: impl AsRef<std::path::Path>) -> String {
        let mut w = Vec::new();
        let _ = self.shader_source_input(path, &mut w);

        std::str::from_utf8(w.as_slice()).unwrap().to_string()
    }

    pub fn create_shader_module(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(self.source.as_str())),
        })
    }
}
