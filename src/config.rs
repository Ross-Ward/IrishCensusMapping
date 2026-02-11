use std::collections::HashMap;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub input: InputConfig,
    pub processing: ProcessingConfig,
    pub output: OutputConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputConfig {
    pub shapefile: PathBuf,
    pub data_csv: PathBuf,
    pub join_column_shape: String,
    pub join_column_csv: String,
    pub water_mask: Option<PathBuf>, // Added for water masking
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingConfig {
    pub datasets: HashMap<String, DatasetConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatasetConfig {
    pub categories: Vec<CategoryConfig>,
    pub not_stated: Option<NotStatedConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CategoryConfig {
    pub name: String,
    pub color: String, // Hex code
    pub columns: Vec<String>, // CSV columns to sum
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotStatedConfig {
    pub column: String,
}


#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub tile_dir: PathBuf,
    pub min_zoom: u8,
    pub max_zoom: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

impl AppConfig {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        let config: AppConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        Ok(config)
    }
}
