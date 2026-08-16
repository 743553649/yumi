use anyhow::Result;
use std::path::Path;

pub fn find_config_path() -> Option<&'static str> {
    let paths = [
        "/data/adb/yumi/config/rules.yaml",
        "/data/local/tmp/yumi/rules.yaml",
        "/data/adb/yumi/rules.yaml",
    ];

    paths.iter().find(|p| Path::new(p).exists()).copied()
}

pub fn read_config_file() -> Result<String> {
    let path = find_config_path().ok_or_else(|| anyhow::anyhow!("Config file not found"))?;
    Ok(std::fs::read_to_string(path)?)
}
