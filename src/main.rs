use anyhow::Result;
use tracing::info;
use load_balancer_rs::config::Config;
use load_balancer_rs::lb::LoadBalancer;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    info!("Starting Load Balancer...");

    // Parse CLI e carica config 
    let config = match Config::from_file("config/config.yaml"){
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load config file: {error}");
            // Usa configurazione di default o esci
            Config::default()
        }
    };

    // Crea e inzia load balancer
    let lb = LoadBalancer::new(config).await?;
    lb.start().await?;

    Ok(())
}
