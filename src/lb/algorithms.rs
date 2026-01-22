use crate::backend::pool::BackendPool;
use crate::backend::*;
use std::sync::atomic::Ordering;
use crate::backend::pool::BackendState;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

impl BackendPool {

    pub async fn round_robin_select(&self, healthy: &[Arc<BackendState>]) -> Option<Arc<BackendState>> { 

        if healthy.is_empty() {
            return None;
        }

        let idx = self.round_robin_idx.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| Some((current + 1) % healthy.len())
        ).ok()?;

        Some(healthy[idx].clone())
    }

    pub async fn least_connections_select(&self, healthy: &[Arc<BackendState>]) -> Option<Arc<BackendState>> {
        if healthy.is_empty() { return None; }

        let backends = healthy.to_vec();

        let selected = backends
            .into_iter()
            .min_by_key(|bs| bs.connections.load(Ordering::Relaxed))?;

        Some(selected)
    }

    fn compute_healthy_backends_hash(&self, healthy: &[Arc<BackendState>]) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        for bs in healthy {
            bs.backend.name.hash(&mut hasher);
            bs.backend.weight.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub async fn weighted_round_robin_select(&self, healthy: &[Arc<BackendState>]) -> Option<Arc<BackendState>> {
        if healthy.is_empty() {
            return None;
        }

        let mut wrr_state = self.weighted_rr_state.write().await;

        // Check if we need to recompute (weights or backends changed)
        let current_hash = self.compute_healthy_backends_hash(healthy);
        if wrr_state.last_backends_hash != current_hash {
            // Recompute expanded list
            wrr_state.expanded_list.clear();
            for backend_state in healthy {
                for _ in 0..backend_state.backend.weight {
                    wrr_state.expanded_list.push(backend_state.backend.name.clone());
                }
            }
            wrr_state.last_backends_hash = current_hash;
            wrr_state.generation.fetch_add(1, Ordering::SeqCst);
        }

        if wrr_state.expanded_list.is_empty() {
            return None;
        }

        let idx = self.round_robin_idx.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| Some((current + 1) % wrr_state.expanded_list.len())
        ).ok()?;

        let selected_name = &wrr_state.expanded_list[idx];

        healthy
            .iter()
            .find(|bs| bs.backend.name == *selected_name)
            .cloned()
    }
    pub fn random_select(&self, healthy: &[Arc<BackendState>]) -> Option<Arc<BackendState>> {
        use rand::Rng;

        if healthy.is_empty() {
            return None;
        }

        let idx = rand::thread_rng().gen_range(0..healthy.len());
        Some(healthy[idx].clone())
    }

    pub async fn update_backend_status(&self, index: usize, status: BackendStatus) -> bool {
        let current_state = self.state.load();

        if index >= current_state.len() {
            return false;
        }

        let new_backends_state: Vec<Arc<BackendState>> = current_state
            .iter()
            .enumerate()
            .map(|(i, backend_state)| {
                if i == index {
                    // Update this backend's status
                    Arc::new(BackendState {
                        backend: backend_state.backend.clone(),
                        status,
                        connections: AtomicU32::new(
                            backend_state.connections.load(Ordering::Relaxed)
                        ),
                    })
                } else {
                    backend_state.clone()
                }
            })
            .collect();

        self.state.store(Arc::new(new_backends_state));
        true
    }
}
