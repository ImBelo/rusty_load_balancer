use super::server::{Backend, BackendStatus, LoadBalancingStrategy};
use arc_swap::ArcSwap;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

#[derive(Debug)]
pub struct BackendPool {
    pub backends: Arc<ArcSwap<Vec<Backend>>>,
    pub statuses: Arc<RwLock<Vec<BackendStatus>>>,
    pub strategy: LoadBalancingStrategy,
    pub connections_counts: Arc<RwLock<HashMap<Arc<Backend>,AtomicU32>>>,
    pub current_index: Arc<RwLock<usize>>,
    pub weighted_rr_state: Arc<RwLock<WeightedRRState>>,  // Per weighted round robin
}

// Implementa Clone manualmente
impl Clone for BackendPool {
    fn clone(&self) -> Self {
        Self {
            backends: Arc::clone(&self.backends),
            statuses: Arc::clone(&self.statuses),
            strategy: self.strategy,
            connections_counts: Arc::clone(&self.connections_counts), 
            current_index: Arc::clone(&self.current_index),
            weighted_rr_state: Arc::clone(&self.weighted_rr_state),
        }
    }
}

#[derive(Default,Clone,Debug)]
pub struct WeightedRRState {
    pub expanded_list: Vec<Backend>,
}

impl BackendPool {
    pub fn new(backends: Vec<Backend>, strategy: LoadBalancingStrategy) -> Self {
        let statuses = vec![BackendStatus::Unknown; backends.len()];

        Self {
            backends: Arc::new(ArcSwap::new(Arc::new(backends))),
            statuses: Arc::new(RwLock::new(statuses)),
            strategy,
            connections_counts: Arc::new(RwLock::new(HashMap::new())),
            current_index: Arc::new(RwLock::new(0)),
            weighted_rr_state: Arc::new(RwLock::new(WeightedRRState { expanded_list: Vec::new()})),
        }
    }

    pub async fn get_healthy_backends(&self) -> Vec<(Backend, BackendStatus)> {
        let backends = self.backends.load();
        let statuses = self.statuses.read().await;

        backends.iter()
            .zip(statuses.iter())
            .filter(|(_, status)| **status == BackendStatus::Healthy)
            .map(|(backend, status)| (backend.clone(), *status))
            .collect()
    }
    pub async fn select_and_increment(&self) -> Option<Backend> {
        let healthy = self.get_healthy_backends().await;
        if healthy.is_empty() {
            return None;
        }


        let selected = match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin_select(&healthy).await,
            LoadBalancingStrategy::LeastConnections => self.least_connections_select(&healthy).await,
            LoadBalancingStrategy::WeightedRoundRobin => self.weighted_round_robin_select(&healthy).await,
            LoadBalancingStrategy::Random => self.random_select(&healthy),
        }?;

        let mut counts_guard = self.connections_counts.write().await;
        let arc_backend = Arc::new(selected.clone());
        let entry = counts_guard.entry(arc_backend).or_insert_with(|| AtomicU32::new(0));
        entry.fetch_add(1, Ordering::SeqCst);

        Some(selected)
    }


    pub async fn decrement_connections(&self, backend: &Backend) {
        let counts = self.connections_counts.read().await;
        if let Some(atomic_count) = counts.get(backend) {
            let mut current = atomic_count.load(Ordering::Relaxed);
            while current > 0 {
                match atomic_count.compare_exchange_weak(
                    current, 
                    current - 1, 
                    Ordering::SeqCst, 
                    Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(actual) => current = actual,
                }
            }
        }
    }

    pub async fn get_connection_count(&self, backend: &Backend) -> u32 {
        let counts = self.connections_counts.read().await;
        counts.get(backend)
            .map(|atomic| atomic.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0)
    }

}
