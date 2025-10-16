use crate::config::BackendConfig;
use clap::Parser;
use crate::config::Config;

#[derive(Parser, Debug)]
#[command(name = "load-balancer")]
#[command(about = "A high-performance load balancer written in Rust")]
#[command(version = "1.0")]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/config.yaml")]
    pub config: String,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,

    /// Load balancing strategy
    #[arg(long, default_value = "round_robin")]
    pub strategy: String,

    /// Health check interval in seconds
    #[arg(long, default_value_t = 10)]
    pub health_check_interval: u64,
}

impl Cli {
    pub fn parse() -> Config {
        let cli = Self::parse_args();
        
        // Try to load from config file, fallback to CLI args
        match Config::from_file(&cli.config) {
            Ok(mut config) => {
                // Override with CLI args if provided
                if cli.host != "127.0.0.1" {
                    config.host = cli.host;
                }
                if cli.port != 3000 {
                    config.port = cli.port;
                }
                if cli.strategy != "round_robin" {
                    config.lb_strategy = cli.strategy;
                }
                if cli.health_check_interval != 10 {
                    config.health_check_interval = cli.health_check_interval;
                }
                config
            }
            Err(_) => {
                // Use CLI args to create config
                Config {
                    host: cli.host,
                    port: cli.port,
                    lb_strategy: cli.strategy,
                    health_check_interval: cli.health_check_interval,
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
    }

    fn parse_args() -> Self {
        <Self as Parser>::parse()
    }
}
