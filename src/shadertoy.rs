static API_KEY: &str = "rdnjhn";

use serde::Deserialize;

#[derive(clap::Parser, Debug, Deserialize)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    shader_id: String,
    //#[arg(short, long)]
    //output: String,
}

#[derive(Debug, Deserialize)]
struct ShaderInfo {
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

#[derive(Debug, Deserialize)]
struct ShaderOutput {
    id: i32,
    channel: i32,
}

#[derive(Debug, Deserialize)]
struct RenderPass {
    inputs: Vec<i32>,
    outputs: Vec<ShaderOutput>,
    code: String,
    name: String,
    description: String,
    r#type: String,
}
#[derive(Debug, Deserialize)]
struct Shader {
    ver: String,
    info: ShaderInfo,
    renderpass: Vec<RenderPass>,
}

#[derive(Debug)]
enum ShaderProcessingError {
    RequestError(reqwest::Error),
    ShaderError(String),
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

#[derive(Debug, Deserialize)]
enum ShaderToyApiResponse {
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
		//layout(binding=0) uniform float     iChannelTime[4];       // channel playback time (in seconds)
		layout(binding=0) uniform vec4      iMouse;                // mouse pixel coords. xy: current (if MLB down), zw: click
		layout(binding=0) uniform vec4      iDate;                 // (year, month, day, time in seconds)
		layout(binding=0) uniform float     iSampleRate;           // sound sample rate (i.e., 44100)
        "#
    }
}

#[cfg(test)]
mod tests {
    use naga::back::wgsl::WriterFlags;
    use pollster::FutureExt;

    use super::*;

    #[test]
    fn test_naga() {
        use naga::front::glsl::{Frontend, Options};
        use naga::ShaderStage;

        let mut glsl = String::from("#version 450 core\n");

        glsl += Shader::default_uniform_block();
        glsl += r#"
const int MAX_STEP = 32;
const float MAX_DISTANCE = 1000.f;
float current_ray_length = 0.f;
vec3 c = vec3(0, 0, 0);
const float r = 1.0;
const float MIN_DISTANCE = 0.01;


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

"#;
        // We simply add an empty main function to the shader
        // Because the shader can only be parsed if it has a main function
        // The actual main function is added later via dualcontour.wgsl shader
        glsl += r#" void main() {}"#;

        let mut frontend = Frontend::default();
        let options = Options::from(ShaderStage::Fragment);

        let module = frontend.parse(&options, &glsl).unwrap();

        let mut out_string = String::new();
        let mut wgsl_writer = naga::back::wgsl::Writer::new(&mut out_string, WriterFlags::empty());

        use naga::valid::Validator;
        let module_info = Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();

        wgsl_writer.write(&module, &module_info).unwrap();

        println!("{}", out_string);
    }
}
