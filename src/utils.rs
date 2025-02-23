use std::{env, path::PathBuf};
use tracing::{info, error, debug, Level};

// Helper function to get the relative path
pub fn get_relative_path(template_dir: &PathBuf, path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let absolute_template_dir = if template_dir.is_relative() {
        cwd.join(template_dir)
    } else {
        template_dir.clone()
    };

    let relative_path = path.strip_prefix(&absolute_template_dir)?;
    let relative_path_str = relative_path.to_string_lossy().to_string();
    Ok(relative_path_str)
}
