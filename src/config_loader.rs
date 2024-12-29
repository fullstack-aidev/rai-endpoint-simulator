use serde::Deserialize;

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct BindingConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub source: String,
    pub database: DatabaseConfig,
    pub binding: BindingConfig,
    pub tracking: TrackingConfig,
    pub log_level: String,
}

#[derive(Deserialize)]
pub struct TrackingConfig {
    pub enabled: bool,
}

impl Config {
    pub fn load() -> Self {
        let config_str = std::fs::read_to_string("config.yml").expect("Failed to read config file");
        serde_yaml::from_str(&config_str).expect("Failed to parse config file")
    }
}