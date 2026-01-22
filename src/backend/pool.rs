use super::server::{Backend, BackendStatus, LoadBalancingStrategy};
use arc_swap::ArcSwap;
use std::{sync::Arc};
use tokio::sync::RwLock;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicUsize;

#[derive(Debug)]
pub struct BackendPool {
    pub state: Arc<ArcSwap<Vec<Arc<BackendState>>>>,
    pub strategy: LoadBalancingStrategy,
    pub round_robin_idx: Arc<AtomicUsize>,
    pub weighted_rr_state: Arc<RwLock<WeightedRRState>>,
}

#[derive(Debug)]
pub struct BackendState {
    pub backend: Backend,
    pub status: BackendStatus,
    pub connections: AtomicU32,
}
// Implementa Clone manualmente
impl Clone for BackendPool {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            strategy: self.strategy,
            round_robin_idx: Arc::clone(&self.round_robin_idx),
            weighted_rr_state: Arc::clone(&self.weighted_rr_state),
        }
    }
}

#[derive(Default,Debug)]
pub struct WeightedRRState {
    pub expanded_list: Vec<String>, 
    pub generation: AtomicU64,      
    pub last_backends_hash: u64,   
}

impl BackendPool {
    pub fn new(backends: Vec<Backend>, strategy: LoadBalancingStrategy) -> Self {
        let backend_states: Vec<Arc<BackendState>> = backends
            .into_iter()
            .map(|backend| {
                Arc::new(BackendState {
                    backend,
                    status: BackendStatus::Unknown,
                    connections: AtomicU32::new(0),
                })
            })
            .collect();

        let weighted_rr_state = match strategy {
            LoadBalancingStrategy::WeightedRoundRobin => {
                // Pre-compute expanded list for weighted round robin
                let expanded_list = backend_states
                    .iter()
                    .flat_map(|state| {
                        std::iter::repeat(state.backend.name.clone())
                            .take(state.backend.weight as usize)
                    })
                    .collect();
                
                Arc::new(RwLock::new(WeightedRRState {
                    expanded_list,
                    generation: AtomicU64::new(1),
                    last_backends_hash: 0, // Will be computed on first use
                }))
            }
            _ => Arc::new(RwLock::new(WeightedRRState {
                expanded_list: Vec::new(),
                generation: AtomicU64::new(0),
                last_backends_hash: 0,
            })),
        };

        Self {
            state: Arc::new(ArcSwap::new(Arc::new(backend_states))),
            strategy,
            round_robin_idx: Arc::new(AtomicUsize::new(0)),
            weighted_rr_state,
        }
    }

pub fn get_healthy_backends(&self) -> Vec<Arc<BackendState>> {
    let state = self.state.load();
    state.iter()
        .filter(|s| s.status == BackendStatus::Healthy)
        .map(Arc::clone)
        .collect()
}
    pub async fn select_and_increment(&self) -> Option<Arc<BackendState>> {
        // Get a snapshot of current healthy backends
        let state = self.state.load();

        // Filter healthy backends
        let healthy: Vec<Arc<BackendState>> = state
            .iter()
            .filter(|backend_state| backend_state.status == BackendStatus::Healthy)
            .cloned()
            .collect();

        if healthy.is_empty() {
            return None;
        }

        // Select based on strategy
        let selected = match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin_select(&healthy).await,
            LoadBalancingStrategy::LeastConnections => self.least_connections_select(&healthy).await,
            LoadBalancingStrategy::WeightedRoundRobin => self.weighted_round_robin_select(&healthy).await,
            LoadBalancingStrategy::Random => self.random_select(&healthy),
        }?;

        selected.connections.fetch_add(1, Ordering::SeqCst);

        Some(selected)
    }
    pub async fn decrement_connections(&self, backend_name: &str) {
        let backend_opt = self.get_backend_by_name(backend_name).await;
        if let Some(backend) = backend_opt{
            let count = &backend.connections;
            count.fetch_sub(1, Ordering::SeqCst);

        }
    }

    pub fn get_connection_count(&self, backend_name: &str) -> u32 {
        let state = self.state.load();
        state
            .iter()
            .find(|backend_state| backend_state.backend.name == backend_name)
            .map(|backend_state| backend_state.connections.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
    pub async fn get_backend_by_name(&self, name: &str) -> Option<Arc<BackendState>> {
        let backends_state = self.state.load();
        backends_state
            .iter()
            .find(|backend_state| backend_state.backend.name == name)
            .cloned()  // This clones the Arc, not the BackendState
    }

}
