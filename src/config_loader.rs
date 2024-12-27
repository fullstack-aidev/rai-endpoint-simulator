use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub source: String,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn load() -> Self {
        let config_content = fs::read_to_string("config.yml").expect("Failed to read config file");
        serde_yaml::from_str(&config_content).expect("Failed to parse config file")
    }
}