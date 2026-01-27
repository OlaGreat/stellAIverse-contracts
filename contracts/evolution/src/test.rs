#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

#[test]
fn test_evolution_history_recording() {
    let env = Env::default();
    
    // 1. Enable Auth Mocking FIRST
    // This allows init_contract and other functions to pass require_auth() checks
    env.mock_all_auths(); 
    
    // 2. Setup users
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    
    // 3. Register Contract
    let contract_id = env.register_contract(None, Evolution);
    let client = EvolutionClient::new(&env, &contract_id);

    // 4. Initialize (Now this will pass auth)
    client.init_contract(&admin);

    // 5. Create a request (User action)
    let request_id = client.create_request(&1, &owner, &1000);

    // 6. Verify history is empty initially
    let initial_history = client.get_agent_evolution_history(&owner);
    assert_eq!(initial_history.len(), 0);

    // 7. Execute Evolution (Admin action)
    // This should trigger the `append_evolution` logic
    client.execute_evolution(&request_id, &1, &2);

    // 8. Verify History was recorded
    let history = client.get_agent_evolution_history(&owner);
    
    // Check count
    assert_eq!(history.len(), 1);
    
    // Check record details
    let record = history.get(0).unwrap();
    assert_eq!(record.from_stage, 1);
    assert_eq!(record.to_stage, 2);
    assert_eq!(record.agent_id, owner);
    assert_eq!(record.trigger, Symbol::new(&env, "admin_exe"));
    
    // Check latest getter
    let latest = client.get_agent_latest_evolution(&owner).unwrap();
    assert_eq!(latest.to_stage, 2);
}