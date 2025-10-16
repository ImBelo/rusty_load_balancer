use crate::backend::pool::BackendPool;
use crate::backend::*;

impl BackendPool {

    pub async fn round_robin_select(&self, healthy: &[(Backend, BackendStatus)]) -> Option<Backend> {
        let mut index = self.current_index.write().await;
        let selected = healthy.get(*index % healthy.len()).map(|(b, _)| b.clone()); 
        *index = (*index + 1) % healthy.len();
        selected
    }

    pub async fn least_connections_select(&self, healthy: &[(Backend, BackendStatus)]) -> Option<Backend> {
        let connection_counts = self.connections_counts.read().await;
        healthy.iter()
        .filter_map(|(backend, _)| {
            let count = connection_counts.get(backend)?;
            Some((backend, count))
        })
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(backend, _)| backend.clone())  // Clone di Arc (economico)
    }

    pub async fn weighted_round_robin_select(&self, healthy: &[(Backend, BackendStatus)]) -> Option<Backend> {
        
        let mut state = self.weighted_rr_state.write().await;
        
        if state.expanded_list.is_empty() {
            for (backend, _) in healthy {
                for _ in 0..backend.weight {
                    state.expanded_list.push(backend.clone());
                }
            }
        }

        if state.expanded_list.is_empty() {
            return None;
        }

        // Leggi e aggiorna l'indice in una sola operazione
        let selected_index = {
            let mut index_guard = self.current_index.write().await;
            let current = *index_guard;
            *index_guard = (current + 1) % state.expanded_list.len();
            current
        };

        Some(state.expanded_list[selected_index].clone())

    }

    pub fn random_select(&self, healthy: &[(Backend, BackendStatus)]) -> Option<Backend> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        healthy.choose(&mut rng).map(|(b, _)| b.clone())
    }

    pub async fn update_backend_status(&self, index: usize, status: BackendStatus) {
        let mut statuses = self.statuses.write().await;
        if let Some(s) = statuses.get_mut(index) {
            *s = status;
        }
    }

}
