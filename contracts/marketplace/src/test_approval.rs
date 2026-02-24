#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use stellai_lib::{
    ApprovalStatus, AuctionType, DEFAULT_APPROVAL_THRESHOLD,
    DEFAULT_APPROVERS_REQUIRED, DEFAULT_TOTAL_APPROVERS, DEFAULT_APPROVAL_TTL_SECONDS
};

use crate::{Marketplace, MarketplaceClient};

fn setup() -> (Env, MarketplaceClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    
    client.init_contract(&admin);
    
    (env, client, admin)
}

#[test]
fn test_approval_config_default() {
    let (_env, client, _admin) = setup();
    
    let config = client.get_approval_config();
    
    assert_eq!(config.threshold, DEFAULT_APPROVAL_THRESHOLD);
    assert_eq!(config.approvers_required, DEFAULT_APPROVERS_REQUIRED);
    assert_eq!(config.total_approvers, DEFAULT_TOTAL_APPROVERS);
    assert_eq!(config.ttl_seconds, DEFAULT_APPROVAL_TTL_SECONDS);
}

#[test]
fn test_set_approval_config() {
    let (_env, client, admin) = setup();
    
    client.set_approval_config(
        &admin,
        &5000_000_000,
        &3,
        &5,
        &(86400 * 14),
    );
    
    let config = client.get_approval_config();
    assert_eq!(config.threshold, 5000_000_000);
    assert_eq!(config.approvers_required, 3);
    assert_eq!(config.total_approvers, 5);
    assert_eq!(config.ttl_seconds, 86400 * 14);
}

#[test]
fn test_propose_sale_success() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    
    let approvers = Vec::from_array(&env, [approver1, approver2, approver3]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.approval_id, approval_id);
    assert_eq!(approval.listing_id, Some(listing_id));
    assert_eq!(approval.buyer, buyer);
    assert_eq!(approval.price, 5000);
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert_eq!(approval.required_approvals, 2);
    assert_eq!(approval.approvers.len(), 3);
}

#[test]
#[should_panic(expected = "Price below approval threshold")]
fn test_propose_sale_below_threshold() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    
    client.set_approval_config(&admin, &10000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    
    let approvers = Vec::from_array(&env, [approver1]);
    client.propose_sale(&listing_id, &buyer, &approvers);
}

#[test]
#[should_panic(expected = "Insufficient approvers")]
fn test_propose_sale_insufficient_approvers() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    
    let approvers = Vec::from_array(&env, [approver1]);
    client.propose_sale(&listing_id, &buyer, &approvers);
}

#[test]
fn test_approve_sale_success() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone(), approver3]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    client.approve_sale(&approval_id, &approver1);
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert_eq!(approval.approvals_received.len(), 1);
    
    client.approve_sale(&approval_id, &approver2);
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Approved);
    assert_eq!(approval.approvals_received.len(), 2);
}

#[test]
#[should_panic(expected = "Unauthorized approver")]
fn test_approve_sale_unauthorized() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    client.approve_sale(&approval_id, &unauthorized);
}

#[test]
fn test_reject_sale() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    let reason = String::from_str(&env, "Suspicious activity detected");
    client.reject_sale(&approval_id, &approver1, &reason);
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Rejected);
    assert_eq!(approval.rejections_received.len(), 1);
    assert_eq!(approval.rejection_reasons.get(0).unwrap(), reason);
}

#[test]
fn test_approval_history() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    let history = client.get_approval_history(&approval_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().action, String::from_str(&env, "proposed"));
    
    client.approve_sale(&approval_id, &approver1);
    
    let history = client.get_approval_history(&approval_id);
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(1).unwrap().action, String::from_str(&env, "approved"));
}

#[test]
#[should_panic(expected = "Approval expired")]
fn test_approval_expiration() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &1);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    env.ledger().set_timestamp(env.ledger().timestamp() + 2);
    
    client.approve_sale(&approval_id, &approver1);
}

#[test]
fn test_cleanup_expired_approvals() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &1);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    env.ledger().set_timestamp(env.ledger().timestamp() + 2);
    
    client.cleanup_expired_approvals();
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Expired);
}

#[test]
fn test_execute_approved_sale() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone()]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);
    
    client.approve_sale(&approval_id, &approver1);
    client.approve_sale(&approval_id, &approver2);
    
    client.execute_approved_sale(&approval_id);
    
    let listing = client.get_listing(&listing_id).unwrap();
    assert!(!listing.active);
    
    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Executed);
}

#[test]
#[should_panic(expected = "High-value sale requires multi-signature approval")]
fn test_buy_agent_requires_approval_for_high_value() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    
    client.set_approval_config(&admin, &1000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    
    client.buy_agent(&listing_id, &buyer);
}

#[test]
fn test_buy_agent_below_threshold() {
    let (env, client, admin) = setup();
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    
    client.set_approval_config(&admin, &10000, &2, &3, &604800);
    
    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    
    client.buy_agent(&listing_id, &buyer);
    
    let listing = client.get_listing(&listing_id).unwrap();
    assert!(!listing.active);
}
