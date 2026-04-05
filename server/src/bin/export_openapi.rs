use std::{env, fs, path::PathBuf};

use server::docs::ApiDoc;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi.json"));

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let spec = ApiDoc::openapi();
    fs::write(output_path, serde_json::to_vec_pretty(&spec)?)?;

    Ok(())
}
