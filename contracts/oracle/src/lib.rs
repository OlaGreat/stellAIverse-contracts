#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests;
#[cfg(any(test, feature = "testutils"))]
mod testutils;

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, IntoVal, Symbol, Val, Vec};
use stellai_lib::{OracleData, ADMIN_KEY, PROVIDER_LIST_KEY};

#[contracttype]
pub enum DataKey {
    Oracle(BytesN<32>),
    OracleNonce(BytesN<32>),
}

#[contracttype]
#[derive(Clone)]
pub struct RelayRequest {
    pub relay_contract: Address,
    pub oracle_pubkey: BytesN<32>,
    pub target_contract: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub nonce: u64,
    pub deadline: u64,
}

#[contract]
pub struct Oracle;

#[contractimpl]
impl Oracle {
    pub fn init_contract(env: Env, admin: Address) {
        let admin_data: Option<Address> =
            env.storage().instance().get(&Symbol::new(&env, ADMIN_KEY));
        if admin_data.is_some() {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        env.storage()
            .instance()
            .set(&Symbol::new(&env, ADMIN_KEY), &admin);

        let providers: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, PROVIDER_LIST_KEY), &providers);
    }

    fn verify_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(env, ADMIN_KEY))
            .unwrap_or_else(|| panic!("Contract not initialized"));

        if caller != &admin {
            panic!("Caller is not admin");
        }
    }

    fn is_authorized_provider(env: &Env, provider: &Address) -> bool {
        let providers: Vec<Address> = env
            .storage()
            .instance()
            .get(&Symbol::new(env, PROVIDER_LIST_KEY))
            .unwrap_or_else(|| Vec::new(env));

        for p in providers.iter() {
            if &p == provider {
                return true;
            }
        }
        false
    }

    pub fn register_provider(env: Env, admin: Address, provider: Address) {
        admin.require_auth();
        Self::verify_admin(&env, &admin);

        let mut providers: Vec<Address> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, PROVIDER_LIST_KEY))
            .unwrap_or_else(|| Vec::new(&env));

        for p in providers.iter() {
            if p == provider {
                panic!("Provider already registered");
            }
        }

        providers.push_back(provider.clone());
        env.storage()
            .instance()
            .set(&Symbol::new(&env, PROVIDER_LIST_KEY), &providers);

        env.events().publish(
            (Symbol::new(&env, "provider_registered"),),
            (admin, provider),
        );
    }

    pub fn submit_data(env: Env, provider: Address, key: Symbol, value: i128) {
        provider.require_auth();

        if !Self::is_authorized_provider(&env, &provider) {
            panic!("Unauthorized: provider not registered");
        }

        let timestamp = env.ledger().timestamp();

        let oracle_data = OracleData {
            key: key.clone(),
            value,
            timestamp,
            provider: provider.clone(),
            signature: None,
            source: None,
        };

        env.storage().instance().set(&key, &oracle_data);

        env.events().publish(
            (Symbol::new(&env, "data_submitted"),),
            (key, timestamp, provider),
        );
    }

    pub fn get_data(env: Env, key: Symbol) -> Option<OracleData> {
        env.storage().instance().get(&key)
    }

    pub fn deregister_provider(env: Env, admin: Address, provider: Address) {
        admin.require_auth();
        Self::verify_admin(&env, &admin);

        let providers: Vec<Address> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, PROVIDER_LIST_KEY))
            .unwrap_or_else(|| Vec::new(&env));

        let mut updated_providers = Vec::new(&env);
        let mut found = false;

        for p in providers.iter() {
            if p != provider {
                updated_providers.push_back(p.clone());
            } else {
                found = true;
            }
        }

        if !found {
            panic!("Provider not found");
        }

        env.storage()
            .instance()
            .set(&Symbol::new(&env, PROVIDER_LIST_KEY), &updated_providers);

        env.events().publish(
            (Symbol::new(&env, "provider_deregistered"),),
            (admin, provider),
        );
    }

    fn is_approved_oracle_key(env: &Env, oracle_pubkey: &BytesN<32>) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&DataKey::Oracle(oracle_pubkey.clone()))
            .unwrap_or(false)
    }

    pub fn register_oracle_key(env: Env, admin: Address, oracle_pubkey: BytesN<32>) {
        admin.require_auth();
        Self::verify_admin(&env, &admin);

        if Self::is_approved_oracle_key(&env, &oracle_pubkey) {
            panic!("Oracle key already registered");
        }

        env.storage()
            .instance()
            .set(&DataKey::Oracle(oracle_pubkey.clone()), &true);

        env.events().publish(
            (Symbol::new(&env, "oracle_key_registered"),),
            (admin, oracle_pubkey),
        );
    }

    pub fn deregister_oracle_key(env: Env, admin: Address, oracle_pubkey: BytesN<32>) {
        admin.require_auth();
        Self::verify_admin(&env, &admin);

        if !Self::is_approved_oracle_key(&env, &oracle_pubkey) {
            panic!("Oracle key not found");
        }

        env.storage()
            .instance()
            .remove(&DataKey::Oracle(oracle_pubkey.clone()));
        env.storage()
            .instance()
            .remove(&DataKey::OracleNonce(oracle_pubkey.clone()));

        env.events().publish(
            (Symbol::new(&env, "oracle_key_deregistered"),),
            (admin, oracle_pubkey),
        );
    }

    pub fn is_registered_oracle_key(env: Env, oracle_pubkey: BytesN<32>) -> bool {
        Self::is_approved_oracle_key(&env, &oracle_pubkey)
    }

    fn get_oracle_nonce(env: &Env, oracle_pubkey: &BytesN<32>) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::OracleNonce(oracle_pubkey.clone()))
            .unwrap_or(0u64)
    }

    fn set_oracle_nonce(env: &Env, oracle_pubkey: &BytesN<32>, nonce: u64) {
        env.storage()
            .instance()
            .set(&DataKey::OracleNonce(oracle_pubkey.clone()), &nonce);
    }

    fn build_relay_message(env: &Env, req: &RelayRequest) -> Bytes {
        // Hash of deterministic Val encoding (works on wasm guest; signer hashes same Val).
        let val = req.clone().into_val(env);
        let serialized = env.serialize_to_bytes(val);
        let hash = env.crypto().sha256(&serialized);
        Bytes::from_slice(env, hash.as_slice())
    }

    pub fn relay_signed(
        env: Env,
        oracle_pubkey: BytesN<32>,
        target_contract: Address,
        function: Symbol,
        args: Vec<Val>,
        nonce: u64,
        deadline: u64,
        signature: BytesN<64>,
    ) -> Val {
        if !Self::is_approved_oracle_key(&env, &oracle_pubkey) {
            panic!("Oracle not approved");
        }

        if env.ledger().timestamp() > deadline {
            panic!("Signature expired");
        }

        let stored_nonce = Self::get_oracle_nonce(&env, &oracle_pubkey);
        if nonce <= stored_nonce {
            panic!("Invalid nonce: replay protection triggered");
        }

        let req = RelayRequest {
            relay_contract: env.current_contract_address(),
            oracle_pubkey: oracle_pubkey.clone(),
            target_contract: target_contract.clone(),
            function: function.clone(),
            args: args.clone(),
            nonce,
            deadline,
        };

        let message = Self::build_relay_message(&env, &req);
        env.crypto()
            .ed25519_verify(&oracle_pubkey, &message, &signature);

        Self::set_oracle_nonce(&env, &oracle_pubkey, nonce);

        let result: Val = env.invoke_contract(&target_contract, &function, args);

        env.events().publish(
            (Symbol::new(&env, "payload_relayed"),),
            (oracle_pubkey, target_contract, function, nonce),
        );

        result
    }
}
