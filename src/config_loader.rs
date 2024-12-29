use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub source: String,
    pub database: DatabaseConfig,
    pub tracking: TrackingConfig,
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)] // Add this struct
pub struct TrackingConfig {
    pub enabled: bool,
}

impl Config {
    pub fn load() -> Self {
        let config_content = fs::read_to_string("config.yml").expect("Failed to read config file");
        serde_yaml::from_str(&config_content).expect("Failed to parse config file")
    }
}