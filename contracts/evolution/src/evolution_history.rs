#![allow(non_snake_case)]
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

// 1. Define the Data Structure
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionRecord {
    pub timestamp: u64,      // When it happened
    pub from_stage: u32,     // Old stage
    pub to_stage: u32,       // New stage
    pub agent_id: Address,   // Who evolved
    pub trigger: Symbol,     // Why (manual, auto, quest)
}

// 2. Define Storage Keys
#[contracttype]
pub enum EvolutionDataKey {
    History(Address), // Maps an Address to a Vec<EvolutionRecord>
}

// 3. The Core Function: Append Only (Internal Use)
pub fn append_evolution(
    env: &Env,
    agent_id: &Address,
    from_stage: u32,
    to_stage: u32,
    trigger: Symbol,
) {
    let key = EvolutionDataKey::History(agent_id.clone());
    
    // Retrieve existing history or start a new list
    let mut history: Vec<EvolutionRecord> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));

    // Create the new record
    let record = EvolutionRecord {
        timestamp: env.ledger().timestamp(),
        from_stage,
        to_stage,
        agent_id: agent_id.clone(),
        trigger,
    };

    // Append to the list
    history.push_back(record);

    // Save back to storage (Persistent storage for long-term data)
    env.storage().persistent().set(&key, &history);
    
    // Extend TTL to ensure history isn't lost (30 days worth of blocks)
    // Adjust the sequence number logic based on your specific network config if needed
    env.storage().persistent().extend_ttl(&key, 17280, 518400); 
}

// 4. Getter Functions (Read-Only)

/// Get the full list of evolutions for an agent
pub fn get_evolution_history(env: &Env, agent_id: &Address) -> Vec<EvolutionRecord> {
    let key = EvolutionDataKey::History(agent_id.clone());
    env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env))
}

/// Get the total count of evolutions
pub fn get_evolution_count(env: &Env, agent_id: &Address) -> u32 {
    let history = get_evolution_history(env, agent_id);
    history.len()
}

/// Get the most recent evolution record
pub fn get_latest_evolution(env: &Env, agent_id: &Address) -> Option<EvolutionRecord> {
    let history = get_evolution_history(env, agent_id);
    if history.is_empty() {
        None
    } else {
        // Get the last item
        let last_index = history.len() - 1;
        Some(history.get(last_index).unwrap())
    }
}

/// Get a specific evolution record by index
pub fn get_evolution_at_index(env: &Env, agent_id: &Address, index: u32) -> Option<EvolutionRecord> {
    let history = get_evolution_history(env, agent_id);
    history.get(index)
}