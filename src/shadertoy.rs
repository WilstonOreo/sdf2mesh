static API_KEY: &str = "rdnjhn";

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct ShaderInfo {
    id: String,
    date: String,
    viewed: i32,
    name: String,
    username: String,
    description: String,
    likes: i32,
    published: i32,
    flags: i32,

    #[serde(rename = "usePreview")]
    use_preview: i32,
    tags: Vec<String>,
    hasliked: i32,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Sampler {
    filter: String,
    wrap: String,
    vflip: String,
    srgb: String,
    internal: String,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct ShaderInput {
    id: i32,
    src: String,
    ctype: String,
    channel: i32,
    sampler: Sampler,
    published: i32,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct ShaderOutput {
    id: i32,
    channel: i32,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct RenderPass {
    inputs: Vec<ShaderInput>,
    outputs: Vec<ShaderOutput>,
    code: String,
    name: String,
    description: String,
    r#type: String,
}
#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Shader {
    ver: String,
    info: ShaderInfo,
    renderpass: Vec<RenderPass>,
}

#[derive(Debug)]
pub enum ShaderProcessingError {
    RequestError(reqwest::Error),
    ShaderError(String),
    ParseError(naga::front::glsl::ParseError),
    WgslError(naga::back::wgsl::Error),
    ValidationError(naga::WithSpan<naga::valid::ValidationError>),

    /// Error when the SDF is missing in the shader
    MissingSdf(String),
}

impl From<reqwest::Error> for ShaderProcessingError {
    fn from(error: reqwest::Error) -> Self {
        ShaderProcessingError::RequestError(error)
    }
}

impl From<String> for ShaderProcessingError {
    fn from(error: String) -> Self {
        ShaderProcessingError::ShaderError(error)
    }
}

impl From<naga::front::glsl::ParseError> for ShaderProcessingError {
    fn from(error: naga::front::glsl::ParseError) -> Self {
        ShaderProcessingError::ParseError(error)
    }
}

impl From<naga::back::wgsl::Error> for ShaderProcessingError {
    fn from(error: naga::back::wgsl::Error) -> Self {
        ShaderProcessingError::WgslError(error)
    }
}

impl From<naga::WithSpan<naga::valid::ValidationError>> for ShaderProcessingError {
    fn from(error: naga::WithSpan<naga::valid::ValidationError>) -> Self {
        ShaderProcessingError::ValidationError(error)
    }
}

#[derive(Debug, Deserialize)]
pub enum ShaderToyApiResponse {
    Shader(Shader),
    Error(String),
}

impl Shader {
    pub fn fetch_code_from_last_pass(&self) -> Option<String> {
        self.renderpass.last().map(|last| last.code.clone())
    }

    pub async fn from_api(shader_id: &str) -> Result<Self, ShaderProcessingError> {
        let response = reqwest::get(format!(
            "https://www.shadertoy.com/api/v1/shaders/{shader_id}?key={API_KEY}"
        ))
        .await?;

        let shader = response.json::<ShaderToyApiResponse>().await?;

        match shader {
            ShaderToyApiResponse::Shader(shader) => Ok(shader),
            ShaderToyApiResponse::Error(error) => Err(error.into()),
        }
    }

    pub fn default_uniform_block() -> &'static str {
        r#"
        layout(binding=0) uniform vec3      iResolution;           // viewport resolution (in pixels)
		layout(binding=0) uniform float     iTime;                 // shader playback time (in seconds)
		layout(binding=0) uniform float     iTimeDelta;            // render time (in seconds)
		layout(binding=0) uniform int       iFrame;                // shader playback frame
		layout(binding=0) uniform vec4      iChannelTime;          // channel playback time (in seconds)
		layout(binding=0) uniform vec4      iMouse;                // mouse pixel coords. xy: current (if MLB down), zw: click
		layout(binding=0) uniform vec4      iDate;                 // (year, month, day, time in seconds)
		layout(binding=0) uniform float     iSampleRate;           // sound sample rate (i.e., 44100)
        "#
    }

    pub fn generate_wgsl_shader_code(&self) -> Result<WgslShaderCode, ShaderProcessingError> {
        let mut glsl = String::from("#version 450 core\n");

        glsl += Shader::default_uniform_block();

        let shader_code = &self.fetch_code_from_last_pass().unwrap();
        glsl += shader_code;
        glsl += r#" void main() {}"#; // We simply add an empty main function to the shader

        convert_glsl_to_wgsl(&glsl).map(WgslShaderCode)
    }
}

pub fn convert_glsl_to_wgsl(glsl: &str) -> Result<String, ShaderProcessingError> {
    use naga::back::wgsl::WriterFlags;
    use naga::front::glsl::{Frontend, Options};
    use naga::ShaderStage;

    // Setup and parse GLSL fragment shader
    let mut frontend = Frontend::default();
    let options = Options::from(ShaderStage::Fragment);

    let module = frontend.parse(&options, glsl)?;

    // Write to WGSL
    let mut wgsl = String::new();
    let mut wgsl_writer = naga::back::wgsl::Writer::new(&mut wgsl, WriterFlags::empty());

    use naga::valid::Validator;
    let module_info = Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;

    wgsl_writer.write(&module, &module_info)?;

    Ok(wgsl)
}

pub struct WgslShaderCode(String);

impl WgslShaderCode {
    pub fn remove_function(&mut self, function_name: &str) -> Result<(), ShaderProcessingError> {
        self.0 = remove_function_from_wgsl(&self.0, function_name)?;
        Ok(())
    }

    pub fn has_function(&self, function_name: &str) -> bool {
        wgsl_has_function(&self.0, function_name).unwrap_or(false)
    }

    pub fn rename_function(
        &mut self,
        old_function_name: &str,
        new_function_name: &str,
    ) -> Result<(), ShaderProcessingError> {
        self.0 = rename_function_in_wgsl(&self.0, old_function_name, new_function_name)?;
        Ok(())
    }

    pub fn add_line(&mut self, line: &str) {
        self.0 += line;
        self.0 += "\n";
    }
}

impl std::fmt::Display for WgslShaderCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn remove_function_from_wgsl(
    wgsl: &str,
    function_name: &str,
) -> Result<String, ShaderProcessingError> {
    // find function name in wgsl
    let mut lines = wgsl.lines();
    let mut new_wgsl = String::new();
    let mut in_function = false;
    let mut function_found = false;
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.starts_with(function_name) {
            in_function = true;
            function_found = true;
        }

        if in_function {
            if line.starts_with("return;") {
                in_function = false;
                lines.next();
            }
        } else {
            new_wgsl += format!("{}\n", line).as_str();
        }
    }
    if !function_found {
        return Err(ShaderProcessingError::ShaderError(format!(
            "Function {} not found in shader",
            function_name
        )));
    }

    Ok(new_wgsl)
}

fn wgsl_has_function(wgsl: &str, function_name: &str) -> Result<bool, ShaderProcessingError> {
    let lines = wgsl.lines();
    let mut function_found = false;
    for line in lines {
        let line = line.trim();
        if line.starts_with(format!("fn {function_name}(").as_str()) {
            function_found = true;
            break;
        }
    }

    if !function_found {
        return Err(ShaderProcessingError::ShaderError(format!(
            "Function {} not found in shader",
            function_name
        )));
    }

    Ok(true)
}

fn rename_function_in_wgsl(
    wgsl: &str,
    old_function_name: &str,
    new_function_name: &str,
) -> Result<String, ShaderProcessingError> {
    // find function name in wgsl
    let lines = wgsl.lines();
    let mut new_wgsl = String::new();
    let mut in_function = false;
    let mut function_found = false;
    for line in lines {
        let line = line.trim();
        if line.starts_with(format!("fn {old_function_name}(").as_str()) {
            in_function = true;
            function_found = true;
            new_wgsl += line
                .replacen(old_function_name, new_function_name, 1)
                .as_str();
        } else {
            new_wgsl += format!("{}\n", line).as_str();
        }

        if in_function && line.starts_with('}') {
            in_function = false;
        }
    }

    if !function_found {
        return Err(ShaderProcessingError::ShaderError(format!(
            "Function `{}` not found in shader",
            old_function_name
        )));
    }

    Ok(new_wgsl)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remove_function() {
        let wgsl = r#"        
fn mainImage(fragColor: ptr<function, vec4<f32>>, fragCoord: vec2<f32>) {
    var fragCoord_1: vec2<f32>;

    fragCoord_1 = fragCoord;
    return;
}

fn main_1() {
    return;
}

@fragment
fn main() {
    main_1();
    return;
}
"#;

        let new_wgsl = remove_function_from_wgsl(wgsl, "fn main_1()").unwrap();

        assert!(new_wgsl.contains("fn mainImage(fragColor"));
        assert!(!new_wgsl.contains("fn main_1()"));

        let new_wgsl = remove_function_from_wgsl(&new_wgsl, "@fragment").unwrap();
        assert!(new_wgsl.contains("fn mainImage(fragColor"));

        assert!(!new_wgsl.contains("fn main()"));

        let new_wgsl = remove_function_from_wgsl(&new_wgsl, "fn mainImage(").unwrap();

        assert!(new_wgsl.trim().is_empty());
    }

    #[test]

    fn rename_function() {
        let in_wgsl = r#"fn normal(p_4: vec3<f32>, epsilon: f32) -> vec3<f32>"#;
        let out_wgsl = rename_function_in_wgsl(in_wgsl, "normal", "sdf3d_normal").unwrap();

        assert!(out_wgsl.contains("fn sdf3d_normal(p_4: vec3<f32>, epsilon: f32) -> vec3<f32>"));
    }

    #[test]
    fn test_naga() {
        let mut glsl = String::from("#version 450 core\n");

        glsl += Shader::default_uniform_block();

        // Our test shader
        glsl += r#"
vec3 c = vec3(0.0, 0.0, 0.0);
const float r = 1.0;
float distance_from_sphere(vec3 p, vec3 c, float r)
{
    return distance(p, c) - r;
}

float sdf3d(vec3 p)
{
    float sphere_0 = distance_from_sphere(p, c, r);
    
    // set displacement
    float displacement = sin(5.0 * p.x) * sin(5.0 * p.y) * sin(5.0 * p.z) * 0.25 * sin(2.f * iTime);
    
    return sphere_0 + displacement;
}

vec3 sdf3d_normal(in vec3 p, in float epsilon)
{
    const vec3 small_step = vec3(epsilon, 0.0, 0.0);

    float gradient_x = sdf3d(p + small_step.xyy) - sdf3d(p - small_step.xyy);
    float gradient_y = sdf3d(p + small_step.yxy) - sdf3d(p - small_step.yxy);
    float gradient_z = sdf3d(p + small_step.yyx) - sdf3d(p - small_step.yyx);

    vec3 normal = vec3(gradient_x, gradient_y, gradient_z);

    return normalize(normal);
}

void mainImage( out vec4 fragColor, in vec2 fragCoord ) {}

"#;
        // We simply add an empty main function to the shader
        // Because the shader can only be parsed if it has a main function
        // The actual main function is added later via dualcontour.wgsl shader
        glsl += r#" void main() {}"#;

        let wgsl = convert_glsl_to_wgsl(&glsl).unwrap();

        println!("{}", wgsl);
    }
}
