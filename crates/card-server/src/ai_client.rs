use std::sync::Mutex;
use std::time::Duration;

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::types::InstanceId;

pub struct AiClient {
    rng: Mutex<ChaCha8Rng>,
}

impl AiClient {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Mutex::new(ChaCha8Rng::seed_from_u64(seed)),
        }
    }
}

fn rand_index(rng: &mut ChaCha8Rng, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    (rng.next_u64() % len as u64) as usize
}

impl PhaseClient for AiClient {
    fn choose_action(&self, available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction> {
        if available.is_empty() {
            return None;
        }
        let mut rng = self.rng.lock().unwrap();
        let idx = rand_index(&mut rng, available.len());
        Some(available[idx].clone())
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        if options.is_empty() || count == 0 {
            return Some(vec![]);
        }
        let mut rng = self.rng.lock().unwrap();
        let take = count.min(options.len());
        let mut indices: Vec<usize> = (0..options.len()).collect();
        let mut selected = Vec::with_capacity(take);
        for i in 0..take {
            let remaining = indices.len() - i;
            let j = i + rand_index(&mut rng, remaining);
            indices.swap(i, j);
            selected.push(options[indices[i]].instance_id);
        }
        Some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::engine::phase::{PhaseAction, RecoveryCardOption};
    use card_core::types::{CardId, InstanceId};
    use std::time::Duration;

    #[test]
    fn ai_chooses_action_from_available() {
        let ai = AiClient::new(42);
        let actions = vec![PhaseAction::Pass, PhaseAction::Surrender];
        let choice = ai.choose_action(&actions, Duration::from_secs(1));
        assert!(choice.is_some());
    }

    #[test]
    fn ai_same_seed_same_sequence() {
        let ai1 = AiClient::new(42);
        let ai2 = AiClient::new(42);
        let actions = vec![PhaseAction::Pass, PhaseAction::Surrender];
        let timeout = Duration::from_secs(1);
        let c1 = ai1.choose_action(&actions, timeout);
        let c2 = ai2.choose_action(&actions, timeout);
        assert_eq!(c1, c2);
    }

    #[test]
    fn ai_empty_available_returns_none() {
        let ai = AiClient::new(42);
        let choice = ai.choose_action(&[], Duration::from_secs(1));
        assert!(choice.is_none());
    }

    #[test]
    fn ai_recovery_selects_from_options() {
        let ai = AiClient::new(42);
        let options = vec![
            RecoveryCardOption {
                instance_id: InstanceId(1),
                definition_id: CardId::new("S000-C-001"),
            },
            RecoveryCardOption {
                instance_id: InstanceId(2),
                definition_id: CardId::new("S000-C-002"),
            },
        ];
        let result = ai.choose_recovery_cards(&options, 1, Duration::from_secs(1));
        assert!(result.is_some());
        let selected = result.unwrap();
        assert_eq!(selected.len(), 1);
        assert!(selected[0] == InstanceId(1) || selected[0] == InstanceId(2));
    }

    #[test]
    fn ai_recovery_empty_returns_empty() {
        let ai = AiClient::new(42);
        let result = ai.choose_recovery_cards(&[], 2, Duration::from_secs(1));
        assert_eq!(result, Some(vec![]));
    }
}
