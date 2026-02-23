use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec, Val, vec};

#[contract]
pub struct StellAIverseProxy;

#[contractimpl]
impl StellAIverseProxy {
    /// Updates the implementation address and runs migration
    pub fn upgrade(env: Env, new_implementation: Address) {
        // 1. Access Control (Admin only)
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        // 2. Pause the contract (Safety)
        env.storage().instance().set(&IS_PAUSED_KEY, &true);

        // 3. Store history (Acceptance Criteria #7)
        let mut history: Vec<(u64, Address)> = env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(vec![&env]);
        history.push_back((env.ledger().timestamp(), new_implementation.clone()));
        env.storage().instance().set(&UPGRADE_HISTORY, &history);

        // 4. Update the pointer
        env.storage().instance().set(&IMPLEMENTATION_KEY, &new_implementation);

        // 5. CALL THE MIGRATION (Acceptance Criteria #5)
        // This invokes the 'migrate' function on the NEW contract code
        env.invoke_contract::<()>(&new_implementation, &Symbol::new(&env, "migrate"), vec![&env]);

        // 6. Resume operations
        env.storage().instance().set(&IS_PAUSED_KEY, &false);
    }

    /// This is the "Dispatcher". It forwards any unknown call to the implementation.
    pub fn __dispatch(env: Env, function: Symbol, args: Vec<Val>) -> Val {
        // Check if paused
        let paused: bool = env.storage().instance().get(&IS_PAUSED_KEY).unwrap_or(false);
        if paused { panic!("Contract is currently migrating"); }

        let impl_addr: Address = env.storage().instance().get(&IMPLEMENTATION_KEY).unwrap();
        
        // Forward the call
        env.invoke_contract(&impl_addr, &function, args)
    }
}