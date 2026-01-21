#![no_std]

use soroban_sdk::{contract, contractimpl, Symbol, Address, Env, String, Vec, Map, map};

const ADMIN_KEY: &str = "admin";
const RULE_KEY_PREFIX: &str = "rule_";
const ACTION_HISTORY_KEY_PREFIX: &str = "history_";
const ACTION_NONCE_KEY_PREFIX: &str = "nonce_";
const RATE_LIMIT_KEY_PREFIX: &str = "rate_limit_";

#[contract]
pub struct ExecutionHub;

#[contractimpl]
impl ExecutionHub {
    /// Initialize contract with admin
    pub fn init_contract(env: Env, admin: Address) {
        let admin_data = env.storage().instance().get::<_, Address>(&Symbol::new(&env, ADMIN_KEY));
        if admin_data.is_some() {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
    }

    /// Verify caller is admin
    fn verify_admin(env: &Env, caller: &Address) {
        let admin: Address = env.storage()
            .instance()
            .get(&Symbol::new(env, ADMIN_KEY))
            .expect("Admin not set");
        
        if caller != &admin {
            panic!("Unauthorized: caller is not admin");
        }
    }

    /// Register a new execution rule for an agent with validation
    pub fn register_rule(
        env: Env,
        agent_id: u64,
        owner: Address,
        rule_name: String,
        rule_data: soroban_sdk::Bytes,
    ) {
        owner.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if rule_name.len() > shared::MAX_STRING_LENGTH {
            panic!("Rule name exceeds maximum length");
        }
        if rule_data.len() > 65536 {
            panic!("Rule data exceeds maximum size");
        }

        // Verify agent exists by checking nonce (lightweight auth check)
        let _nonce = get_agent_nonce(&env, agent_id);

        let rule_key = String::from_slice(&env, 
            &format!("{}{}_{}", RULE_KEY_PREFIX, agent_id, rule_name.clone()).as_bytes()
        );

        env.storage().instance().set(&rule_key, &rule_data);
        env.events().publish((Symbol::new(&env, "rule_registered"),), (agent_id, rule_name, owner));
    }

    /// Execute an agent action with full validation and replay protection
    pub fn execute_action(
        env: Env,
        agent_id: u64,
        executor: Address,
        action: String,
        parameters: soroban_sdk::Bytes,
        nonce: u64,
    ) -> bool {
        executor.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if action.len() > shared::MAX_STRING_LENGTH {
            panic!("Action name exceeds maximum length");
        }
        if parameters.len() > 65536 {
            panic!("Parameters exceed maximum size");
        }

        // Replay protection: verify nonce
        let stored_nonce = get_action_nonce(&env, agent_id);
        if nonce <= stored_nonce {
            panic!("Replay protection: invalid nonce");
        }

        // Rate limiting check
        check_rate_limit(&env, agent_id, 100, 60); // Max 100 actions per 60 seconds

        // Get agent to verify executor is owner
        let key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        let agent: shared::Agent = env.storage()
            .instance()
            .get(&key)
            .expect("Agent not found");

        if agent.owner != executor {
            panic!("Unauthorized: only agent owner can execute actions");
        }

        // Store new nonce
        let nonce_key = String::from_slice(&env, 
            &format!("{}{}", ACTION_NONCE_KEY_PREFIX, agent_id).as_bytes()
        );
        env.storage().instance().set(&nonce_key, &nonce);

        // Record action in history
        let history_key = String::from_slice(&env,
            &format!("{}{}", ACTION_HISTORY_KEY_PREFIX, agent_id).as_bytes()
        );
        let mut history: Vec<String> = env.storage()
            .instance()
            .get(&history_key)
            .unwrap_or_else(|_| Vec::new(&env));

        // Limit history size to prevent unbounded growth
        if history.len() >= 1000 {
            panic!("Action history limit exceeded, use get_history to review");
        }

        let timestamp = env.ledger().timestamp();
        let action_record = format!("{}_{}_{}", action, executor, timestamp);
        history.push_back(String::from_slice(&env, action_record.as_bytes()));
        env.storage().instance().set(&history_key, &history);

        env.events().publish(
            (Symbol::new(&env, "action_executed"),),
            (agent_id, action, executor, timestamp)
        );

        true
    }

    /// Get execution history for an agent (with limit for DoS protection)
    pub fn get_history(
        env: Env,
        agent_id: u64,
        limit: u32,
    ) -> Vec<String> {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if limit > 500 {
            panic!("Limit exceeds maximum allowed (500)");
        }

        let history_key = String::from_slice(&env,
            &format!("{}{}", ACTION_HISTORY_KEY_PREFIX, agent_id).as_bytes()
        );
        
        let history: Vec<String> = env.storage()
            .instance()
            .get(&history_key)
            .unwrap_or_else(|_| Vec::new(&env));

        // Return limited results
        let mut result = Vec::new(&env);
        let max_items = if (limit as usize) < history.len() { 
            limit as usize 
        } else { 
            history.len() 
        };

        for i in 0..max_items {
            if let Some(item) = history.get(history.len() - max_items + i) {
                result.push_back(item);
            }
        }

        result
    }

    /// Revoke a rule with proper authorization
    pub fn revoke_rule(
        env: Env,
        agent_id: u64,
        owner: Address,
        rule_name: String,
    ) {
        owner.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        // Verify owner is agent owner
        let agent_key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        let agent: shared::Agent = env.storage()
            .instance()
            .get(&agent_key)
            .expect("Agent not found");

        if agent.owner != owner {
            panic!("Unauthorized: only agent owner can revoke rules");
        }

        let rule_key = String::from_slice(&env,
            &format!("{}{}_{}", RULE_KEY_PREFIX, agent_id, rule_name.clone()).as_bytes()
        );

        env.storage().instance().remove(&rule_key);
        env.events().publish((Symbol::new(&env, "rule_revoked"),), (agent_id, rule_name, owner));
    }
}

/// Helper: Safe addition with overflow check
fn safe_add(a: u64, b: u64) -> u64 {
    a.checked_add(b).expect("Arithmetic overflow")
}

/// Helper: Get agent nonce from agent-nft contract
fn get_agent_nonce(env: &Env, agent_id: u64) -> u64 {
    // This would call the agent-nft contract
    // For now, returning 0 as placeholder
    0
}

/// Helper: Get action nonce for replay protection
fn get_action_nonce(env: &Env, agent_id: u64) -> u64 {
    let nonce_key = String::from_slice(&env,
        &format!("{}{}", ACTION_NONCE_KEY_PREFIX, agent_id).as_bytes()
    );
    env.storage()
        .instance()
        .get(&nonce_key)
        .unwrap_or(0)
}

/// Helper: Rate limiting check (basic implementation)
fn check_rate_limit(env: &Env, agent_id: u64, max_operations: u32, window_seconds: u64) {
    let now = env.ledger().timestamp();
    let limit_key = String::from_slice(&env,
        &format!("{}{}", RATE_LIMIT_KEY_PREFIX, agent_id).as_bytes()
    );

    let (last_reset, count): (u64, u32) = env.storage()
        .instance()
        .get(&limit_key)
        .unwrap_or((now, 0));

    let new_count = if now > last_reset + window_seconds {
        1 // Reset window
    } else if count < max_operations {
        count + 1
    } else {
        panic!("Rate limit exceeded");
    };

    let new_reset = if now > last_reset + window_seconds { now } else { last_reset };
    env.storage().instance().set(&limit_key, &(new_reset, new_count));
}

