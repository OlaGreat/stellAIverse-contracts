#![cfg(test)]

use soroban_sdk::{symbol_short, Address, Env, String, Vec};
use stellai_lib::{
    ApprovalConfig, ApprovalStatus, ApprovalHistory, DEFAULT_APPROVAL_THRESHOLD,
    DEFAULT_APPROVERS_REQUIRED, DEFAULT_TOTAL_APPROVERS, DEFAULT_APPROVAL_TTL_SECONDS
};

use crate::{
    Marketplace,
    storage::*,
    contractdata::DataKey,
};

#[test]
fn test_approval_config_default() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Get default config
    let config = Marketplace::get_approval_config(env.clone());
    
    assert_eq!(config.threshold, DEFAULT_APPROVAL_THRESHOLD);
    assert_eq!(config.approvers_required, DEFAULT_APPROVERS_REQUIRED);
    assert_eq!(config.total_approvers, DEFAULT_TOTAL_APPROVERS);
    assert_eq!(config.ttl_seconds, DEFAULT_APPROVAL_TTL_SECONDS);
}

#[test]
fn test_set_approval_config() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set custom config
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        5000_000_000, // 5,000 USDC
        3, // require 3 approvals
        5, // out of 5 total approvers
        86400 * 14, // 14 days
    );
    
    // Verify config
    let config = Marketplace::get_approval_config(env.clone());
    assert_eq!(config.threshold, 5000_000_000);
    assert_eq!(config.approvers_required, 3);
    assert_eq!(config.total_approvers, 5);
    assert_eq!(config.ttl_seconds, 86400 * 14);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_set_approval_config_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin);
    
    // Try to set config with unauthorized user
    Marketplace::set_approval_config(
        env,
        unauthorized,
        5000_000_000,
        3,
        5,
        86400 * 14,
    );
}

#[test]
fn test_propose_sale_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set low threshold for testing
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        1000, // very low threshold
        2,
        3,
        604800,
    );
    
    // Create a high-value listing
    let listing_id = Marketplace::create_listing(
        env.clone(),
        1, // agent_id
        seller,
        0, // Sale type
        5000, // price above threshold
    );
    
    // Propose sale
    let approvers = Vec::from_array(&env, [approver1, approver2, approver3]);
    let approval_id = Marketplace::propose_sale(
        env.clone(),
        listing_id,
        buyer,
        approvers,
    );
    
    // Verify approval was created
    let approval = Marketplace::get_approval(env.clone(), approval_id).unwrap();
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
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set high threshold
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        10000, // high threshold
        2,
        3,
        604800,
    );
    
    // Create a low-value listing
    let listing_id = Marketplace::create_listing(
        env.clone(),
        1, // agent_id
        seller,
        0, // Sale type
        5000, // price below threshold
    );
    
    // Try to propose sale
    let approvers = Vec::from_array(&env, [approver1]);
    Marketplace::propose_sale(
        env,
        listing_id,
        buyer,
        approvers,
    );
}

#[test]
#[should_panic(expected = "Insufficient approvers")]
fn test_propose_sale_insufficient_approvers() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config requiring 2 approvers
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        1000,
        2, // require 2 approvers
        3,
        604800,
    );
    
    // Create a listing
    let listing_id = Marketplace::create_listing(
        env.clone(),
        1, // agent_id
        seller,
        0, // Sale type
        5000,
    );
    
    // Try to propose with only 1 approver
    let approvers = Vec::from_array(&env, [approver1]);
    Marketplace::propose_sale(
        env,
        listing_id,
        buyer,
        approvers,
    );
}

#[test]
fn test_approve_sale_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let approver3 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        1000,
        2, // require 2 approvals
        3,
        604800,
    );
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2, approver3]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // First approval
    Marketplace::approve_sale(env.clone(), approval_id, approver1);
    
    // Check status - still pending
    let approval = Marketplace::get_approval(env.clone(), approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert_eq!(approval.approvals_received.len(), 1);
    
    // Second approval - should reach required threshold
    Marketplace::approve_sale(env.clone(), approval_id, approver2);
    
    // Check status - now approved
    let approval = Marketplace::get_approval(env.clone(), approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Approved);
    assert_eq!(approval.approvals_received.len(), 2);
}

#[test]
#[should_panic(expected = "Unauthorized approver")]
fn test_approve_sale_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // Try to approve with unauthorized user
    Marketplace::approve_sale(env, approval_id, unauthorized);
}

#[test]
fn test_reject_sale() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // Reject sale
    let reason = String::from_str(&env, "Suspicious activity detected");
    Marketplace::reject_sale(env.clone(), approval_id, approver1, reason.clone());
    
    // Check status - rejected
    let approval = Marketplace::get_approval(env.clone(), approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Rejected);
    assert_eq!(approval.rejections_received.len(), 1);
    assert_eq!(approval.rejection_reasons.get(0).unwrap(), &reason);
}

#[test]
fn test_approval_history() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // Get history
    let history = Marketplace::get_approval_history(env.clone(), approval_id);
    assert_eq!(history.len(), 1); // Should have "proposed" entry
    assert_eq!(history.get(0).unwrap().action, String::from_str(&env, "proposed"));
    
    // Approve
    Marketplace::approve_sale(env.clone(), approval_id, approver1);
    
    // Check history again
    let history = Marketplace::get_approval_history(env.clone(), approval_id);
    assert_eq!(history.len(), 2); // Should have "proposed" and "approved"
    assert_eq!(history.get(1).unwrap().action, String::from_str(&env, "approved"));
}

#[test]
fn test_approval_expiration() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set short TTL for testing
    Marketplace::set_approval_config(
        env.clone(),
        admin,
        1000,
        2,
        3,
        1, // 1 second TTL
    );
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // Fast forward time
    env.ledger().set_timestamp(env.ledger().timestamp() + 2);
    
    // Try to approve - should fail due to expiration
    let result = std::panic::catch_unwind(|| {
        Marketplace::approve_sale(env.clone(), approval_id, approver1);
    });
    assert!(result.is_err());
    
    // Clean up expired approvals
    Marketplace::cleanup_expired_approvals(env.clone());
    
    // Check status - should be expired
    let approval = Marketplace::get_approval(env, approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Expired);
}

#[test]
fn test_execute_approved_sale() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set approval config
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create and propose sale
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_sale(env.clone(), listing_id, buyer, approvers);
    
    // Approve sale
    Marketplace::approve_sale(env.clone(), approval_id, approver1);
    Marketplace::approve_sale(env.clone(), approval_id, approver2);
    
    // Execute sale
    Marketplace::execute_approved_sale(env.clone(), approval_id);
    
    // Check that listing is no longer active
    let listing = Marketplace::get_listing(env.clone(), listing_id).unwrap();
    assert!(!listing.active);
    
    // Check approval status
    let approval = Marketplace::get_approval(env, approval_id).unwrap();
    assert_eq!(approval.status, ApprovalStatus::Executed);
}

#[test]
fn test_buy_agent_requires_approval_for_high_value() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set low threshold
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create a high-value listing
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    
    // Try to buy directly - should fail
    let result = std::panic::catch_unwind(|| {
        Marketplace::buy_agent(env.clone(), listing_id, buyer);
    });
    assert!(result.is_err());
}

#[test]
fn test_buy_agent_below_threshold() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set high threshold
    Marketplace::set_approval_config(env.clone(), admin, 10000, 2, 3, 604800);
    
    // Create a low-value listing
    let listing_id = Marketplace::create_listing(env.clone(), 1, seller, 0, 5000);
    
    // Buy directly - should succeed
    Marketplace::buy_agent(env.clone(), listing_id, buyer.clone());
    
    // Check that listing is no longer active
    let listing = Marketplace::get_listing(env, listing_id).unwrap();
    assert!(!listing.active);
}

#[test]
fn test_propose_auction_sale() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let bidder = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let approver2 = Address::generate(&env);
    
    // Initialize contract
    Marketplace::init_contract(env.clone(), admin.clone());
    
    // Set low threshold
    Marketplace::set_approval_config(env.clone(), admin, 1000, 2, 3, 604800);
    
    // Create auction
    let auction_id = Marketplace::create_auction(
        env.clone(),
        1, // agent_id
        seller,
        stellai_lib::AuctionType::English,
        1000, // start_price
        500,  // reserve_price
        86400, // duration
        1000, // min_bid_increment_bps
        None, // dutch_config
    );
    
    // Simulate a bid
    Marketplace::place_bid(env.clone(), auction_id, bidder, 2000);
    
    // End the auction
    env.ledger().set_timestamp(env.ledger().timestamp() + 86401);
    
    // Propose auction sale
    let approvers = Vec::from_array(&env, [approver1, approver2]);
    let approval_id = Marketplace::propose_auction_sale(env.clone(), auction_id, approvers);
    
    // Verify approval was created
    let approval = Marketplace::get_approval(env, approval_id).unwrap();
    assert_eq!(approval.approval_id, approval_id);
    assert_eq!(approval.auction_id, Some(auction_id));
    assert_eq!(approval.buyer, bidder);
    assert_eq!(approval.price, 2000);
    assert_eq!(approval.status, ApprovalStatus::Pending);
}
