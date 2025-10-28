use serde::Deserialize;
use std::hash::Hasher;
use std::hash::Hash;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct Backend {
    pub url: String,
    pub name: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
    WeightedRoundRobin,
}
// Implementa Hash e Eq per Backend
impl Hash for Backend {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);  // Hash basato su URL
    }
}

impl PartialEq for Backend {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url  // Confronta per URL
    }
}
impl Eq for Backend {}  

impl Backend {
    pub fn new(url: String, name: String, weight: u32) -> Self {
        Self { url, name, weight }
    }
    pub async fn simulate_delay(&self) {
        println!("Backend {}: simulando ritardo di 1s", self.url);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

}
