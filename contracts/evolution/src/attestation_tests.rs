#![cfg(test)]

use soroban_sdk::{Address, Env, String, Bytes};
use crate::Evolution;

struct TestSetup {
    env: Env,
    admin: Address,
    owner: Address,
    oracle_provider: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let oracle_provider = Address::generate(&env);

        Evolution::init_contract(env.clone(), admin.clone());

        TestSetup {
            env,
            admin,
            owner,
            oracle_provider,
        }
    }

    fn create_mock_agent(&self, id: u64) -> shared::Agent {
        let agent = shared::Agent {
            id,
            owner: self.owner.clone(),
            name: String::from_str(&self.env, "TestAgent"),
            model_hash: String::from_str(&self.env, "original_hash"),
            capabilities: soroban_sdk::Vec::from_array(&self.env, [
                String::from_str(&self.env, "execute"),
            ]),
            evolution_level: 0,
            created_at: self.env.ledger().timestamp(),
            updated_at: self.env.ledger().timestamp(),
            nonce: 0,
            escrow_locked: false,
            escrow_holder: None,
        };

        let agent_key = String::from_str(&self.env, "agent_1");
        self.env.storage().instance().set(&agent_key, &agent);
        agent
    }

    fn create_evolution_request(&self, request_id: u64, agent_id: u64) -> shared::EvolutionRequest {
        let request = shared::EvolutionRequest {
            request_id,
            agent_id,
            owner: self.owner.clone(),
            stake_amount: 1000,
            status: shared::EvolutionStatus::Pending,
            created_at: self.env.ledger().timestamp(),
            completed_at: None,
        };

        let key = String::from_str(&self.env, "request_1");
        self.env.storage().instance().set(&key, &request);
        request
    }

    fn create_attestation(&self, request_id: u64, agent_id: u64, nonce: u64) -> shared::EvolutionAttestation {
        shared::EvolutionAttestation {
            request_id,
            agent_id,
            oracle_provider: self.oracle_provider.clone(),
            new_model_hash: String::from_str(&self.env, "upgraded_hash_v1"),
            attestation_data: Bytes::from_slice(&self.env, b"training_data_hash"),
            signature: Bytes::from_slice(&self.env, &[0u8; 64]),
            timestamp: self.env.ledger().timestamp(),
            nonce,
        }
    }
}

#[test]
fn test_valid_attestation_updates_agent() {
    let setup = TestSetup::new();
    let env = &setup.env;

    // Setup: Create agent and request
    let agent_id = 1u64;
    let request_id = 1u64;
    setup.create_mock_agent(agent_id);
    setup.create_evolution_request(request_id, agent_id);

    // Get initial state
    let agent_key = String::from_str(env, "agent_1");
    let initial_agent: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(initial_agent.evolution_level, 0);
    assert_eq!(initial_agent.model_hash, String::from_str(env, "original_hash"));

    // Apply valid attestation
    let attestation = setup.create_attestation(request_id, agent_id, 1);
    Evolution::apply_attestation(env.clone(), attestation.clone());

    // Verify agent was updated
    let updated_agent: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(updated_agent.evolution_level, 1);
    assert_eq!(updated_agent.model_hash, String::from_str(env, "upgraded_hash_v1"));
    assert_eq!(updated_agent.nonce, 1);

    // Verify request status changed
    let request_key = String::from_str(env, "request_1");
    let updated_request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    assert_eq!(updated_request.status, shared::EvolutionStatus::Completed);
    assert!(updated_request.completed_at.is_some());
}

#[test]
fn test_attestation_invalid_signature_size_rejected() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Create attestation with invalid signature size
    let mut attestation = setup.create_attestation(1, 1, 1);
    attestation.signature = Bytes::from_slice(env, &[0u8; 32]); // Wrong size

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation);
    }));

    assert!(result.is_err());
}

#[test]
fn test_replay_protection_prevents_reuse() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Apply attestation with nonce 1
    let attestation1 = setup.create_attestation(1, 1, 1);
    Evolution::apply_attestation(env.clone(), attestation1);

    let agent_key = String::from_str(env, "agent_1");
    let agent_after_first: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent_after_first.evolution_level, 1);

    // Reset request for second attempt
    let request_key = String::from_str(env, "request_1");
    let mut request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    request.status = shared::EvolutionStatus::Pending;
    request.completed_at = None;
    env.storage().instance().set(&request_key, &request);

    // Try to apply with same nonce (should fail)
    let attestation2 = setup.create_attestation(1, 1, 1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation2);
    }));

    assert!(result.is_err()); // Should panic due to replay protection

    // Verify agent wasn't updated again
    let agent_after_replay: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent_after_replay.evolution_level, 1); // Still 1, not 2
}

#[test]
fn test_replay_protection_with_higher_nonce_allowed() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Apply first attestation
    let attestation1 = setup.create_attestation(1, 1, 1);
    Evolution::apply_attestation(env.clone(), attestation1);

    let agent_key = String::from_str(env, "agent_1");
    let agent_after_first: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent_after_first.evolution_level, 1);

    // Reset request for second attestation
    let request_key = String::from_str(env, "request_1");
    let mut request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    request.status = shared::EvolutionStatus::Pending;
    request.completed_at = None;
    env.storage().instance().set(&request_key, &request);

    // Apply with higher nonce (should succeed)
    let attestation2 = setup.create_attestation(1, 1, 2);
    Evolution::apply_attestation(env.clone(), attestation2);

    let agent_after_second: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent_after_second.evolution_level, 2);
}

#[test]
fn test_attestation_invalid_request_rejected() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    // Don't create request - try to apply attestation for non-existent request

    let attestation = setup.create_attestation(999, 1, 1); // Non-existent request

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation);
    }));

    assert!(result.is_err());
}

#[test]
fn test_attestation_agent_mismatch_rejected() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Create attestation with mismatched agent ID
    let mut attestation = setup.create_attestation(1, 1, 1);
    attestation.agent_id = 999; // Different from request

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation);
    }));

    assert!(result.is_err());
}

#[test]
fn test_attestation_non_pending_request_rejected() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Mark request as already completed
    let request_key = String::from_str(env, "request_1");
    let mut request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    request.status = shared::EvolutionStatus::Completed;
    env.storage().instance().set(&request_key, &request);

    let attestation = setup.create_attestation(1, 1, 1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation);
    }));

    assert!(result.is_err());
}

#[test]
fn test_attestation_oversized_data_rejected() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Create attestation with oversized data
    let mut attestation = setup.create_attestation(1, 1, 1);
    attestation.attestation_data = Bytes::from_slice(env, &vec![0u8; shared::MAX_ATTESTATION_DATA_SIZE + 1]);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation);
    }));

    assert!(result.is_err());
}

#[test]
fn test_attestation_updates_nonce_tracking() {
    let setup = TestSetup::new();
    let env = &setup.env;

    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Apply attestation with nonce 5
    let attestation = setup.create_attestation(1, 1, 5);
    Evolution::apply_attestation(env.clone(), attestation);

    // Reset request
    let request_key = String::from_str(env, "request_1");
    let mut request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    request.status = shared::EvolutionStatus::Pending;
    request.completed_at = None;
    env.storage().instance().set(&request_key, &request);

    // Attempt with nonce 3 (lower than stored 5) should fail
    let attestation_low = setup.create_attestation(1, 1, 3);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Evolution::apply_attestation(env.clone(), attestation_low);
    }));

    assert!(result.is_err());
}

#[test]
fn test_multiple_attestations_sequential() {
    let setup = TestSetup::new();
    let env = &setup.env;

    // Create first agent and request
    setup.create_mock_agent(1);
    setup.create_evolution_request(1, 1);

    // Apply first attestation
    let att1 = setup.create_attestation(1, 1, 1);
    Evolution::apply_attestation(env.clone(), att1);

    let agent_key = String::from_str(env, "agent_1");
    let agent1: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent1.evolution_level, 1);

    // Reset for second evolution
    let request_key = String::from_str(env, "request_1");
    let mut request: shared::EvolutionRequest = env.storage().instance().get(&request_key).unwrap();
    request.status = shared::EvolutionStatus::Pending;
    request.completed_at = None;
    env.storage().instance().set(&request_key, &request);

    // Apply second attestation with higher nonce
    let mut att2 = setup.create_attestation(1, 1, 2);
    att2.new_model_hash = String::from_str(env, "upgraded_hash_v2");
    Evolution::apply_attestation(env.clone(), att2);

    let agent2: shared::Agent = env.storage().instance().get(&agent_key).unwrap();
    assert_eq!(agent2.evolution_level, 2);
    assert_eq!(agent2.model_hash, String::from_str(env, "upgraded_hash_v2"));
}
