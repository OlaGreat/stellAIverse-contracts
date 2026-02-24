//! Tests for lease lifecycle (issue #49): extension, termination, history, get_active_leases.

#![cfg(test)]

use soroban_sdk::{Address, Env, Symbol, String};
use soroban_sdk::testutils::Address as _;
use stellai_lib::{LeaseData, LeaseState, LeaseHistoryEntry, Listing, ListingType, LISTING_COUNTER_KEY};

use crate::{Marketplace, MarketplaceClient, storage::*};

/// Setup env with marketplace initialized and a lease written to storage (no token needed).
/// Call after init_contract; all storage writes run inside contract context.
fn setup_lease_in_storage(env: &Env, contract_id: &Address) -> (Address, Address, u64, u64) {
    let lessor = Address::generate(env);
    let lessee = Address::generate(env);

    env.as_contract(contract_id, || {
        let listing_id = 1u64;
        let listing_key = (Symbol::new(env, "listing"), listing_id);
        let listing = Listing {
            listing_id,
            agent_id: 10,
            seller: lessor.clone(),
            price: 1000,
            listing_type: ListingType::Lease,
            active: false,
            created_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(&listing_key, &listing);
        env.storage()
            .instance()
            .set(&Symbol::new(env, LISTING_COUNTER_KEY), &listing_id);

        let lease_id = increment_lease_counter(env);
        assert_eq!(lease_id, 1);

        let now = env.ledger().timestamp();
        let duration_seconds = 86400 * 30; // 30 days
        let end_time = now + duration_seconds;
        let total_value = 1000i128;
        let deposit_bps = 1000u32; // 10%
        let deposit_amount = (total_value * (deposit_bps as i128)) / 10_000;

        let lease = LeaseData {
            lease_id,
            agent_id: 10,
            listing_id,
            lessor: lessor.clone(),
            lessee: lessee.clone(),
            start_time: now,
            end_time,
            duration_seconds,
            deposit_amount,
            total_value,
            auto_renew: false,
            lessee_consent_for_renewal: false,
            status: LeaseState::Active,
            pending_extension_id: None,
        };
        set_lease(env, &lease);
        lessee_leases_append(env, &lessee, lease_id);
        lessor_leases_append(env, &lessor, lease_id);

        let entry = LeaseHistoryEntry {
            lease_id,
            action: String::from_str(env, "initiated"),
            actor: lessee.clone(),
            timestamp: now,
            details: None,
        };
        add_lease_history(env, lease_id, &entry);

        (lessor, lessee, lease_id, listing_id)
    })
}

#[test]
fn test_lease_config_default() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.init_contract(&admin);

    let config = env.as_contract(&contract_id, || get_lease_config(&env));
    assert_eq!(config.deposit_bps, stellai_lib::DEFAULT_LEASE_DEPOSIT_BPS);
    assert_eq!(
        config.early_termination_penalty_bps,
        stellai_lib::DEFAULT_EARLY_TERMINATION_PENALTY_BPS
    );
}

#[test]
fn test_set_lease_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.init_contract(&admin);
    client.set_lease_config(&admin, &1500, &2500);

    let config = env.as_contract(&contract_id, || get_lease_config(&env));
    assert_eq!(config.deposit_bps, 1500);
    assert_eq!(config.early_termination_penalty_bps, 2500);
}

#[test]
fn test_get_lease_by_id_and_active_leases() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init_contract(&admin);
    let (lessor, lessee, lease_id, _) = setup_lease_in_storage(&env, &contract_id);

    let lease = client.get_lease_by_id(&lease_id).unwrap();
    assert_eq!(lease.lease_id, lease_id);
    assert_eq!(lease.lessee, lessee);
    assert_eq!(lease.lessor, lessor);
    assert!(lease.status == LeaseState::Active);

    let lessee_leases = client.get_active_leases(&lessee);
    assert_eq!(lessee_leases.len(), 1);
    assert_eq!(lessee_leases.get(0).unwrap().lease_id, lease_id);

    let lessor_leases = client.get_active_leases(&lessor);
    assert_eq!(lessor_leases.len(), 1);
    assert_eq!(lessor_leases.get(0).unwrap().lease_id, lease_id);
}

#[test]
fn test_lease_extension_request_and_approve() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init_contract(&admin);
    let (lessor, lessee, lease_id, _) = setup_lease_in_storage(&env, &contract_id);

    let extension_id = client.request_lease_extension(&lease_id, &lessee, &(86400 * 7));
    assert!(extension_id > 0);

    let lease = client.get_lease_by_id(&lease_id).unwrap();
    assert!(lease.status == LeaseState::ExtensionRequested);
    assert_eq!(lease.pending_extension_id, Some(extension_id));

    client.approve_lease_extension(&lease_id, &extension_id, &lessor);

    let lease_after = client.get_lease_by_id(&lease_id).unwrap();
    assert!(lease_after.status == LeaseState::Active);
    assert_eq!(lease_after.pending_extension_id, None);
    assert_eq!(
        lease_after.duration_seconds,
        lease.duration_seconds + 86400 * 7
    );
}

#[test]
fn test_lease_history() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init_contract(&admin);
    let (_lessor, lessee, lease_id, _) = setup_lease_in_storage(&env, &contract_id);

    let history_before = client.get_lease_history(&lease_id);
    assert_eq!(history_before.len(), 1);
    assert_eq!(history_before.get(0).unwrap().action, String::from_str(&env, "initiated"));

    client.request_lease_extension(&lease_id, &lessee, &3600);

    let history = client.get_lease_history(&lease_id);
    assert!(history.len() >= 2);
    assert_eq!(history.get(0).unwrap().action, String::from_str(&env, "initiated"));
    assert_eq!(history.get(1).unwrap().action, String::from_str(&env, "extension_requested"));
}
