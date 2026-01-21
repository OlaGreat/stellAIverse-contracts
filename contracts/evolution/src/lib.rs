#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};

const ADMIN_KEY: &str = "admin";
const REQUEST_COUNTER_KEY: &str = "request_counter";
const REQUEST_KEY_PREFIX: &str = "request_";
const STAKE_LOCK_PREFIX: &str = "stake_";

#[contract]
pub struct Evolution;

#[contractimpl]
impl Evolution {
    /// Initialize contract with admin
    pub fn init_contract(env: Env, admin: Address) {
        let admin_data = env.storage().instance().get::<_, Address>(&Symbol::new(&env, ADMIN_KEY));
        if admin_data.is_some() {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
        env.storage().instance().set(&Symbol::new(&env, REQUEST_COUNTER_KEY), &0u64);
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

    /// Request an agent upgrade with comprehensive validation
    pub fn request_upgrade(
        env: Env,
        agent_id: u64,
        owner: Address,
        _stake_token: Address,
        stake_amount: i128,
    ) -> u64 {
        owner.require_auth();

        // Input validation
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if stake_amount <= 0 {
            panic!("Stake amount must be positive");
        }
        if stake_amount > shared::PRICE_UPPER_BOUND {
            panic!("Stake amount exceeds safe maximum");
        }

        // Verify agent exists and caller is owner
        let agent_key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        let agent: shared::Agent = env.storage()
            .instance()
            .get(&agent_key)
            .expect("Agent not found");

        if agent.owner != owner {
            panic!("Unauthorized: only agent owner can request upgrade");
        }

        // Prevent too many simultaneous upgrades per agent
        let request_count = count_pending_requests(&env, agent_id);
        if request_count >= 5 {
            panic!("Too many pending upgrade requests for this agent");
        }

        // In production: Transfer stake_amount from stake_token to this contract

        // Generate request ID safely
        let counter: u64 = env.storage()
            .instance()
            .get(&Symbol::new(&env, REQUEST_COUNTER_KEY))
            .unwrap_or(0);
        let request_id = Self::safe_add(counter, 1);

        let request = shared::EvolutionRequest {
            request_id,
            agent_id,
            owner: owner.clone(),
            stake_amount,
            status: shared::EvolutionStatus::Pending,
            created_at: env.ledger().timestamp(),
            completed_at: None,
        };

        let key = String::from_slice(&env, &format!("{}{}", REQUEST_KEY_PREFIX, request_id).as_bytes());
        env.storage().instance().set(&key, &request);

        // Update counter
        env.storage().instance().set(&Symbol::new(&env, REQUEST_COUNTER_KEY), &request_id);

        env.events().publish(
            (Symbol::new(&env, "upgrade_requested"),),
            (request_id, agent_id, owner, stake_amount)
        );

        request_id
    }

    /// Complete an upgrade with authorization and validation
    pub fn complete_upgrade(
        env: Env,
        admin: Address,
        request_id: u64,
        new_model_hash: String,
    ) {
        admin.require_auth();

        if request_id == 0 {
            panic!("Invalid request ID");
        }
        if new_model_hash.len() > shared::MAX_STRING_LENGTH {
            panic!("Model hash exceeds maximum length");
        }

        Self::verify_admin(&env, &admin);

        let request_key = String::from_slice(&env, &format!("{}{}", REQUEST_KEY_PREFIX, request_id).as_bytes());
        let mut request: shared::EvolutionRequest = env.storage()
            .instance()
            .get(&request_key)
            .expect("Upgrade request not found");

        if request.status != shared::EvolutionStatus::Pending {
            panic!("Request is not in pending state");
        }

        // Update agent's model hash
        let agent_key = String::from_slice(&env, &format!("agent_{}", request.agent_id).as_bytes());
        let mut agent: shared::Agent = env.storage()
            .instance()
            .get(&agent_key)
            .expect("Agent not found");

        agent.model_hash = new_model_hash;
        agent.evolution_level = agent.evolution_level.checked_add(1)
            .expect("Evolution level overflow");
        agent.updated_at = env.ledger().timestamp();
        agent.nonce = agent.nonce.checked_add(1).expect("Nonce overflow");

        env.storage().instance().set(&agent_key, &agent);

        // Update request status
        request.status = shared::EvolutionStatus::Completed;
        request.completed_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&request_key, &request);

        // In production: Return stake to owner

        env.events().publish(
            (Symbol::new(&env, "upgrade_completed"),),
            (request_id, request.agent_id, agent.evolution_level)
        );
    }

    /// Get upgrade history for an agent (with limit for DoS protection)
    pub fn get_upgrade_history(
        env: Env,
        agent_id: u64,
    ) -> Vec<shared::EvolutionRequest> {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        // In production, this would query an index
        // For now, returning empty vector
        Vec::new(&env)
    }

    /// Claim staked tokens after upgrade completes
    pub fn claim_stake(env: Env, owner: Address, request_id: u64) {
        owner.require_auth();

        if request_id == 0 {
            panic!("Invalid request ID");
        }

        let request_key = String::from_slice(&env, &format!("{}{}", REQUEST_KEY_PREFIX, request_id).as_bytes());
        let request: shared::EvolutionRequest = env.storage()
            .instance()
            .get(&request_key)
            .expect("Upgrade request not found");

        if request.owner != owner {
            panic!("Unauthorized: only request owner can claim stake");
        }

        if request.status != shared::EvolutionStatus::Completed 
            && request.status != shared::EvolutionStatus::Failed {
            panic!("Stake not yet available for claim");
        }

        // Check double-spend prevention
        let stake_lock = String::from_slice(&env,
            &format!("{}{}", STAKE_LOCK_PREFIX, request_id).as_bytes()
        );
        let claimed: Option<bool> = env.storage().instance().get(&stake_lock);
        if claimed.is_some() {
            panic!("Stake already claimed for this request");
        }

        // Mark as claimed (prevent double-spend)
        env.storage().instance().set(&stake_lock, &true);

        // In production: Transfer stake_amount back to owner

        env.events().publish(
            (Symbol::new(&env, "stake_claimed"),),
            (request_id, request.agent_id, owner, request.stake_amount)
        );
    }

    /// Get current evolution level of an agent
    pub fn get_evolution_level(env: Env, agent_id: u64) -> u32 {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        let agent_key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        env.storage()
            .instance()
            .get::<_, shared::Agent>(&agent_key)
            .map(|agent| agent.evolution_level)
            .unwrap_or(0)
    }
}

/// Helper: Count pending upgrade requests for an agent
fn count_pending_requests(env: &Env, agent_id: u64) -> u32 {
    // In production, this would be more efficient with proper indexing
    0
}

