#![cfg(test)]

use crate::testutils::MockOracle;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

#[test]
fn test_mock_oracle_generates_valid_attestation() {
    let env = Env::default();
    let provider = Address::generate(&env);

    let attestation = MockOracle::generate_attestation(&env, 1, 1, provider.clone(), "new_hash", 1);

    assert_eq!(attestation.request_id, 1);
    assert_eq!(attestation.agent_id, 1);
    assert_eq!(attestation.oracle_provider, provider);
    assert_eq!(attestation.nonce, 1);
    assert_eq!(attestation.signature.len(), 64);
}

#[test]
fn test_mock_oracle_generates_invalid_signature() {
    let env = Env::default();
    let provider = Address::generate(&env);

    let attestation =
        MockOracle::generate_invalid_attestation_signature(&env, 1, 1, provider.clone());

    // Verify the other fields remain correct
    assert_eq!(attestation.request_id, 1);
    assert_eq!(attestation.agent_id, 1);
    assert_eq!(attestation.oracle_provider, provider);
    assert_eq!(attestation.nonce, 1);

    // Signature should be intentionally invalid
    assert_eq!(attestation.signature.len(), 32); // Invalid size
}

#[test]
fn test_mock_oracle_generates_data() {
    let env = Env::default();

    let data = MockOracle::generate_data(&env, "price", "100", "binance");

    assert_eq!(data.key, soroban_sdk::String::from_str(&env, "price"));
    assert_eq!(data.value, soroban_sdk::String::from_str(&env, "100"));
    assert_eq!(data.source, soroban_sdk::String::from_str(&env, "binance"));
}
