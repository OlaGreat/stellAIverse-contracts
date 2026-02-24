#![cfg(test)]

use soroban_sdk::{Address, Env, String, Vec};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use stellai_lib::{
    ApprovalStatus, DEFAULT_APPROVAL_THRESHOLD,
    DEFAULT_APPROVERS_REQUIRED, DEFAULT_TOTAL_APPROVERS, DEFAULT_APPROVAL_TTL_SECONDS
};

use crate::{Marketplace, MarketplaceClient};
use crate::storage::{get_auction, set_auction};

fn setup() -> (Env, MarketplaceClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.init_contract(&admin);

    let config = client.get_approval_config();
    assert_eq!(config.threshold, DEFAULT_APPROVAL_THRESHOLD);
    assert_eq!(config.approvers_required, DEFAULT_APPROVERS_REQUIRED);
    assert_eq!(config.total_approvers, DEFAULT_TOTAL_APPROVERS);
    assert_eq!(config.ttl_seconds, DEFAULT_APPROVAL_TTL_SECONDS);
}

#[test]
fn test_set_approval_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(
        &admin,
        &5000_000_000, // 5,000 USDC
        &3, // require 3 approvals
        &5, // out of 5 total approvers
        &(86400 * 14), // 14 days
    );

    let config = client.get_approval_config();
    assert_eq!(config.threshold, 5000_000_000);
    assert_eq!(config.approvers_required, 3);
    assert_eq!(config.total_approvers, 5);
    assert_eq!(config.ttl_seconds, 86400 * 14);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_set_approval_config_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(
        &unauthorized,
        &5000_000_000,
        &3,
        &5,
        &(86400 * 14),
    );
}

#[test]
fn test_propose_sale_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone(), approver3]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);

    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.approval_id, approval_id);
    assert_eq!(approval.listing_id, Some(listing_id));
    assert_eq!(approval.buyer, buyer);
    assert_eq!(approval.price, 5000);
    assert!(approval.status == ApprovalStatus::Pending);
    assert_eq!(approval.required_approvals, 2);
    assert_eq!(approval.approvers.len(), 3);
}

#[test]
#[should_panic(expected = "Price below approval threshold")]
fn test_propose_sale_below_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &10000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1]);
    client.propose_sale(&listing_id, &buyer, &approvers);
}

#[test]
#[should_panic(expected = "Insufficient approvers")]
fn test_propose_sale_insufficient_approvers() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1]);
    client.propose_sale(&listing_id, &buyer, &approvers);
}

#[test]
fn test_approve_sale_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone(), approver3]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);

    client.approve_sale(&approval_id, &approver1);
    let approval = client.get_approval(&approval_id).unwrap();
    assert!(approval.status == ApprovalStatus::Pending);
    assert_eq!(approval.approvals_received.len(), 1);

    client.approve_sale(&approval_id, &approver2);
    let approval = client.get_approval(&approval_id).unwrap();
    assert!(approval.status == ApprovalStatus::Approved);
    assert_eq!(approval.approvals_received.len(), 2);
}

#[test]
#[should_panic(expected = "Unauthorized approver")]
fn test_approve_sale_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);

    client.approve_sale(&approval_id, &unauthorized);
}

#[test]
fn test_reject_sale() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);

    let reason = String::from_str(&env, "Suspicious activity detected");
    client.reject_sale(&approval_id, &approver1, &reason);

    let approval = client.get_approval(&approval_id).unwrap();
    assert!(approval.status == ApprovalStatus::Rejected);
    assert_eq!(approval.rejections_received.len(), 1);
    assert_eq!(approval.rejection_reasons.get(0).unwrap(), reason);
}

#[test]
fn test_approval_history() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone()]);
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
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &1);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone()]);
    let approval_id = client.propose_sale(&listing_id, &buyer, &approvers);

    env.ledger().set_timestamp(env.ledger().timestamp() + 2);
    client.cleanup_expired_approvals();

    let approval = client.get_approval(&approval_id).unwrap();
    assert!(approval.status == ApprovalStatus::Expired);
}

#[test]
fn test_execute_approved_sale() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);

    client.init_contract(&admin);
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
    assert!(approval.status == ApprovalStatus::Executed);
}

#[test]
#[should_panic]
fn test_buy_agent_requires_approval_for_high_value() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    client.buy_agent(&listing_id, &buyer);
}

#[test]
fn test_buy_agent_below_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &10000, &2, &3, &604800);

    let listing_id = client.create_listing(&1, &seller, &0, &5000);
    client.buy_agent(&listing_id, &buyer);

    let listing = client.get_listing(&listing_id).unwrap();
    assert!(!listing.active);
}

#[test]
fn test_propose_auction_sale() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, Marketplace);
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let bidder = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);

    client.init_contract(&admin);
    client.set_approval_config(&admin, &1000, &2, &3, &604800);

    let auction_id = client.create_auction(
        &1,
        &seller,
        &stellai_lib::AuctionType::English,
        &1000,
        &500,
        &86400,
        &1000,
        &(None, None, None, None),
    );

    // Simulate a bid by setting auction state directly (avoids needing a payment token in tests)
    env.as_contract(&contract_id, || {
        let mut auction = get_auction(&env, auction_id).expect("auction not found");
        auction.highest_bidder = Some(bidder.clone());
        auction.highest_bid = 2000;
        set_auction(&env, &auction);
    });

    env.ledger().set_timestamp(env.ledger().timestamp() + 86401);

    let approvers = Vec::from_array(&env, [approver1.clone(), approver2.clone()]);
    let approval_id = client.propose_auction_sale(&auction_id, &approvers);

    let approval = client.get_approval(&approval_id).unwrap();
    assert_eq!(approval.approval_id, approval_id);
    assert_eq!(approval.auction_id, Some(auction_id));
    assert_eq!(approval.buyer, bidder);
    assert_eq!(approval.price, 2000);
    assert!(approval.status == ApprovalStatus::Pending);
}
