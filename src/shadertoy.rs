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
}

#[cfg(tests)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_from_api() {
        let shader = Shader::from_api("MlK3zV").unwrap();
        assert_eq!(shader.info.id, "MlK3zV");
    }
}
