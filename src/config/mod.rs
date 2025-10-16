use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub lb_strategy: String,
    pub health_check_interval: u64,
    pub backends: Vec<BackendConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackendConfig {
    pub name: String,
    pub url: String,
    pub weight: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            lb_strategy: "round_robin".to_string(),
            health_check_interval: 10,
            backends: vec![
                BackendConfig {
                    name: "backend-1".to_string(),
                    url: "http://127.0.0.1:8081".to_string(),
                    weight: Some(1),
                },
                BackendConfig {
                    name: "backend-2".to_string(),
                    url: "http://127.0.0.1:8082".to_string(),
                    weight: Some(1),
                },
            ],
        }
    }
}
impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

}
