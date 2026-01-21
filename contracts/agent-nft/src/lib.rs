#![no_std]

use soroban_sdk::{contract, contractimpl, Symbol, Address, String, Env, map, Map, Vec};

const ADMIN_KEY: &str = "admin";
const AGENT_COUNTER_KEY: &str = "agent_counter";
const AGENT_KEY_PREFIX: &str = "agent_";
const AGENT_OWNER_INDEX: &str = "owner_agents";

#[contract]
pub struct AgentNFT;

#[contractimpl]
impl AgentNFT {
    /// Initialize contract with admin (one-time setup)
    pub fn init_contract(env: Env, admin: Address) {
        // Security: Verify this is first initialization
        let admin_data = env.storage().instance().get::<_, Address>(&Symbol::new(&env, ADMIN_KEY));
        if admin_data.is_some() {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
        env.storage().instance().set(&Symbol::new(&env, AGENT_COUNTER_KEY), &0u64);
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

    /// Safe addition with overflow checks
    fn safe_add(a: u64, b: u64) -> u64 {
        a.checked_add(b).expect("Arithmetic overflow in safe_add")
    }

    /// Mint a new agent NFT with comprehensive security checks
    pub fn mint_agent(
        env: Env,
        owner: Address,
        name: String,
        model_hash: String,
        capabilities: Vec<String>,
    ) -> u64 {
        owner.require_auth();
        
        // Input validation
        if name.len() > shared::MAX_STRING_LENGTH {
            panic!("Agent name exceeds maximum length");
        }
        if model_hash.len() > shared::MAX_STRING_LENGTH {
            panic!("Model hash exceeds maximum length");
        }
        if capabilities.len() > shared::MAX_CAPABILITIES {
            panic!("Capabilities exceed maximum allowed");
        }

        // Validate each capability string
        for i in 0..capabilities.len() {
            if let Some(cap) = capabilities.get(i) {
                if cap.len() > shared::MAX_STRING_LENGTH {
                    panic!("Individual capability exceeds maximum length");
                }
            }
        }

        // Increment agent counter safely
        let counter: u64 = env.storage()
            .instance()
            .get(&Symbol::new(&env, AGENT_COUNTER_KEY))
            .unwrap_or(0);
        
        let agent_id = Self::safe_add(counter, 1);
        
        // Create agent with nonce initialized to 0 (for replay protection)
        let agent = shared::Agent {
            id: agent_id,
            owner: owner.clone(),
            name,
            model_hash,
            capabilities,
            evolution_level: 0,
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            nonce: 0,
        };

        // Store agent safely
        let key = String::from_slice(&env, &format!("{}{}", AGENT_KEY_PREFIX, agent_id).as_bytes());
        env.storage().instance().set(&key, &agent);
        
        // Update counter
        env.storage().instance().set(&Symbol::new(&env, AGENT_COUNTER_KEY), &agent_id);

        // Emit event (in Soroban, events are emitted via contract data)
        env.events().publish((Symbol::new(&env, "mint_agent"),), (agent_id, owner));

        agent_id
    }

    /// Get agent metadata with bounds checking
    pub fn get_agent(env: Env, agent_id: u64) -> Option<shared::Agent> {
        if agent_id == 0 {
            panic!("Invalid agent ID: must be greater than 0");
        }

        let key = String::from_slice(&env, &format!("{}{}", AGENT_KEY_PREFIX, agent_id).as_bytes());
        env.storage().instance().get::<_, shared::Agent>(&key)
    }

    /// Update agent metadata with authorization check
    pub fn update_agent(
        env: Env,
        agent_id: u64,
        owner: Address,
        name: Option<String>,
        capabilities: Option<Vec<String>>,
    ) {
        owner.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID: must be greater than 0");
        }

        let key = String::from_slice(&env, &format!("{}{}", AGENT_KEY_PREFIX, agent_id).as_bytes());
        let mut agent: shared::Agent = env.storage()
            .instance()
            .get(&key)
            .expect("Agent not found");

        // Authorization check: only owner can update
        if agent.owner != owner {
            panic!("Unauthorized: only agent owner can update");
        }

        // Update fields with validation
        if let Some(new_name) = name {
            if new_name.len() > shared::MAX_STRING_LENGTH {
                panic!("Agent name exceeds maximum length");
            }
            agent.name = new_name;
        }

        if let Some(new_capabilities) = capabilities {
            if new_capabilities.len() > shared::MAX_CAPABILITIES {
                panic!("Capabilities exceed maximum allowed");
            }
            for i in 0..new_capabilities.len() {
                if let Some(cap) = new_capabilities.get(i) {
                    if cap.len() > shared::MAX_STRING_LENGTH {
                        panic!("Individual capability exceeds maximum length");
                    }
                }
            }
            agent.capabilities = new_capabilities;
        }

        // Increment nonce for replay protection
        agent.nonce = agent.nonce.checked_add(1).expect("Nonce overflow");
        agent.updated_at = env.ledger().timestamp();

        env.storage().instance().set(&key, &agent);
        env.events().publish((Symbol::new(&env, "update_agent"),), (agent_id, owner));
    }

    /// Get total agents minted
    pub fn total_agents(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, AGENT_COUNTER_KEY))
            .unwrap_or(0)
    }

    /// Get nonce for replay protection (used by other contracts)
    pub fn get_nonce(env: Env, agent_id: u64) -> u64 {
        if agent_id == 0 {
            panic!("Invalid agent ID: must be greater than 0");
        }

        let key = String::from_slice(&env, &format!("{}{}", AGENT_KEY_PREFIX, agent_id).as_bytes());
        env.storage()
            .instance()
            .get::<_, shared::Agent>(&key)
            .map(|agent| agent.nonce)
            .unwrap_or(0)
    }
}

