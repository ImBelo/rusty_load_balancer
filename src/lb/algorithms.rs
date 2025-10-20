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
        let counts = self.connections_counts.read().await;

        // Trova il backend con meno connessioni
        let selected = healthy.iter()
            .min_by_key(|(backend, _)| {
                counts.get(backend)
                    .map(|atomic| atomic.load(std::sync::atomic::Ordering::Relaxed))
                    .unwrap_or(0)
            })?
            .0.clone();

        // INCREMENTA le connessioni per il backend selezionato
        drop(counts);  // Rilascia il read lock

        Some(selected)
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
