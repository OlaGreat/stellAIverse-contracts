#![no_std]

use soroban_sdk::{ contract, contractimpl, token, Address, Env, Symbol, Vec, String };
use stellai_lib::{
    Listing,
    ListingType,
    RoyaltyInfo,
    LISTING_COUNTER_KEY,
    Auction,
    AuctionType,
    AuctionStatus,
    ApprovalConfig,
    Approval,
    ApprovalStatus,
    ApprovalHistory,
};

mod storage;

use storage::*;

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    /// Initialize contract with admin
    pub fn init_contract(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        set_admin(&env, &admin);

        env.storage().instance().set(&Symbol::new(&env, LISTING_COUNTER_KEY), &0u64);
    }

    /// Set a new admin
    pub fn set_admin(env: Env, new_admin: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        admin.require_auth();
        set_admin(&env, &new_admin);
    }

    /// Set the payment token
    pub fn set_payment_token(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        let current_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(admin == current_admin, "Unauthorized");

        set_payment_token(&env, token);
    }

    /// Create a new listing
    pub fn create_listing(
        env: Env,
        agent_id: u64,
        seller: Address,
        listing_type: u32,
        price: i128
    ) -> u64 {
        seller.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if listing_type > 2 {
            panic!("Invalid listing type");
        }
        if price <= 0 {
            panic!("Price must be positive");
        }

        // Generate listing ID
        let counter: u64 = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, LISTING_COUNTER_KEY))
            .unwrap_or(0);
        let listing_id = counter + 1;

        let listing = Listing {
            listing_id,
            agent_id,
            seller: seller.clone(),
            price,
            listing_type: match listing_type {
                0 => ListingType::Sale,
                1 => ListingType::Lease,
                2 => ListingType::Auction,
                _ => panic!("Invalid listing type"),
            },
            active: true,
            created_at: env.ledger().timestamp(),
        };

        // Store listing using tuple key
        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        env.storage().instance().set(&listing_key, &listing);

        // Update counter
        env.storage().instance().set(&Symbol::new(&env, LISTING_COUNTER_KEY), &listing_id);

        env.events().publish(
            (Symbol::new(&env, "listing_created"),),
            (listing_id, agent_id, seller.clone(), price)
        );

        // Log audit entry for sale created
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{\"listing_created\":true}");
        let tx_hash = String::from_str(&env, "create_listing");
        let description = Some(String::from_str(&env, "Marketplace listing created"));
        
        let _ = create_audit_log(
            &env,
            seller,
            OperationType::SaleCreated,
            before_state,
            after_state,
            tx_hash,
            description,
        );

        listing_id
    }

    /// Purchase an agent
    pub fn buy_agent(env: Env, listing_id: u64, buyer: Address) {
        buyer.require_auth();

        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        let mut listing: Listing = env
            .storage()
            .instance()
            .get(&listing_key)
            .expect("Listing not found");

        if !listing.active {
            panic!("Listing is not active");
        }

        // Check if multi-signature approval is required
        let config = get_approval_config(&env);
        if listing.price >= config.threshold {
            panic!("High-value sale requires multi-signature approval. Use propose_sale() first.");
        }

        // Mark listing as inactive
        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        env.events().publish(
            (Symbol::new(&env, "agent_sold"),),
            (listing_id, listing.agent_id, buyer.clone())
        );

        // Log audit entry for sale completed
        let before_state = String::from_str(&env, "{\"active\":true}");
        let after_state = String::from_str(&env, "{\"active\":false}");
        let tx_hash = String::from_str(&env, "buy_agent");
        let description = Some(String::from_str(&env, "Sale completed"));
        
        let _ = create_audit_log(
            &env,
            buyer,
            OperationType::SaleCompleted,
            before_state,
            after_state,
            tx_hash,
            description,
        );
    }

    /// Cancel a listing
    pub fn cancel_listing(env: Env, listing_id: u64, seller: Address) {
        seller.require_auth();

        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        let mut listing: Listing = env
            .storage()
            .instance()
            .get(&listing_key)
            .expect("Listing not found");

        if listing.seller != seller {
            panic!("Unauthorized: only seller can cancel listing");
        }

        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        env.events().publish(
            (Symbol::new(&env, "listing_cancelled"),),
            (listing_id, listing.agent_id, seller)
        );
    }

    /// Get a specific listing
    pub fn get_listing(env: Env, listing_id: u64) -> Option<Listing> {
        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        env.storage().instance().get(&listing_key)
    }

    /// Set royalty info for an agent
    pub fn set_royalty(env: Env, agent_id: u64, creator: Address, recipient: Address, fee: u32) {
        creator.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if fee > 10000 {
            // 100% in basis points
            panic!("Royalty fee exceeds maximum (100%)");
        }

        let royalty_info = RoyaltyInfo { recipient, fee };

        let royalty_key = (Symbol::new(&env, "royalty"), agent_id);
        env.storage().instance().set(&royalty_key, &royalty_info);

        env.events().publish((Symbol::new(&env, "royalty_set"),), (agent_id, fee));
    }

    /// Get royalty info for an agent
    pub fn get_royalty(env: Env, agent_id: u64) -> Option<RoyaltyInfo> {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        let royalty_key = (Symbol::new(&env, "royalty"), agent_id);
        env.storage().instance().get(&royalty_key)
    }

    // ---------------- MULTI-SIGNATURE APPROVAL ----------------

    /// Configure approval settings (admin only)
    pub fn set_approval_config(
        env: Env,
        admin: Address,
        threshold: i128,
        approvers_required: u32,
        total_approvers: u32,
        ttl_seconds: u64
    ) {
        let current_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(admin == current_admin, "Unauthorized");

        assert!(threshold > 0, "Threshold must be positive");
        assert!(approvers_required > 0, "Approvers required must be positive");
        assert!(total_approvers >= approvers_required, "Total approvers must be >= required");
        assert!(ttl_seconds > 0, "TTL must be positive");

        let config = ApprovalConfig {
            threshold,
            approvers_required,
            total_approvers,
            ttl_seconds,
        };

        set_approval_config(&env, &config);

        env.events().publish(
            (Symbol::new(&env, "ApprovalConfigUpdated"),),
            (threshold, approvers_required, total_approvers, ttl_seconds)
        );
    }

    /// Get current approval configuration
    pub fn get_approval_config(env: Env) -> ApprovalConfig {
        get_approval_config(&env)
    }

    /// Propose a sale for multi-signature approval (fixed-price listing)
    pub fn propose_sale(env: Env, listing_id: u64, buyer: Address, approvers: Vec<Address>) -> u64 {
        buyer.require_auth();

        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        let listing: Listing = env
            .storage()
            .instance()
            .get(&listing_key)
            .expect("Listing not found");

        if !listing.active {
            panic!("Listing is not active");
        }

        let config = get_approval_config(&env);

        // Check if approval is required
        if listing.price < config.threshold {
            panic!("Price below approval threshold");
        }

        assert!(approvers.len() >= config.approvers_required, "Insufficient approvers");
        assert!(approvers.len() <= config.total_approvers, "Too many approvers");

        let approval_id = increment_approval_counter(&env);
        let now = env.ledger().timestamp();

        let approval = Approval {
            approval_id,
            listing_id: Some(listing_id),
            auction_id: None,
            buyer: buyer.clone(),
            price: listing.price,
            proposed_at: now,
            expires_at: now + config.ttl_seconds,
            status: ApprovalStatus::Pending,
            required_approvals: config.approvers_required,
            approvers: approvers.clone(),
            approvals_received: Vec::new(&env),
            rejections_received: Vec::new(&env),
            rejection_reasons: Vec::new(&env),
        };

        set_approval(&env, &approval);

        // Add to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "proposed"),
            actor: buyer.clone(),
            timestamp: now,
            reason: None,
        };
        add_approval_history(&env, approval_id, &history);

        env.events().publish(
            (Symbol::new(&env, "SaleProposed"),),
            (approval_id, listing_id, buyer, listing.price)
        );

        approval_id
    }

    /// Propose an auction win for multi-signature approval
    pub fn propose_auction_sale(env: Env, auction_id: u64, approvers: Vec<Address>) -> u64 {
        let auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.highest_bidder.is_some(), "No winning bid");

        let config = get_approval_config(&env);

        // Check if approval is required
        if auction.highest_bid < config.threshold {
            panic!("Price below approval threshold");
        }

        assert!(approvers.len() >= config.approvers_required, "Insufficient approvers");
        assert!(approvers.len() <= config.total_approvers, "Too many approvers");

        let approval_id = increment_approval_counter(&env);
        let now = env.ledger().timestamp();
        let buyer = auction.highest_bidder.unwrap();

        let approval = Approval {
            approval_id,
            listing_id: None,
            auction_id: Some(auction_id),
            buyer: buyer.clone(),
            price: auction.highest_bid,
            proposed_at: now,
            expires_at: now + config.ttl_seconds,
            status: ApprovalStatus::Pending,
            required_approvals: config.approvers_required,
            approvers: approvers.clone(),
            approvals_received: Vec::new(&env),
            rejections_received: Vec::new(&env),
            rejection_reasons: Vec::new(&env),
        };

        set_approval(&env, &approval);

        // Add to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "proposed"),
            actor: buyer.clone(),
            timestamp: now,
            reason: None,
        };
        add_approval_history(&env, approval_id, &history);

        env.events().publish(
            (Symbol::new(&env, "SaleProposed"),),
            (approval_id, auction_id, buyer, auction.highest_bid)
        );

        approval_id
    }

    /// Approve a proposed sale
    pub fn approve_sale(env: Env, approval_id: u64, approver: Address) {
        approver.require_auth();

        if approval_id == 0 {
            panic!("Invalid approval ID");
        }

        let mut approval = get_approval(&env, approval_id).expect("Approval not found");

        assert!(approval.status == ApprovalStatus::Pending, "Approval not pending");
        assert!(env.ledger().timestamp() < approval.expires_at, "Approval expired");

        // Check if approver is authorized
        assert!(approval.approvers.contains(&approver), "Unauthorized approver");

        // Check if already approved
        assert!(!approval.approvals_received.contains(&approver), "Already approved");
        assert!(!approval.rejections_received.contains(&approver), "Already rejected");

        approval.approvals_received.push_back(approver.clone());

        // Add to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "approved"),
            actor: approver.clone(),
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        add_approval_history(&env, approval_id, &history);

        // Check if we have enough approvals
        if approval.approvals_received.len() >= approval.required_approvals {
            approval.status = ApprovalStatus::Approved;

            // Add final approval to history
            let final_history = ApprovalHistory {
                approval_id,
                action: String::from_str(&env, "fully_approved"),
                actor: approver,
                timestamp: env.ledger().timestamp(),
                reason: None,
            };
            add_approval_history(&env, approval_id, &final_history);

            env.events().publish(
                (Symbol::new(&env, "SaleApproved"),),
                (approval_id, approval.approvals_received.len())
            );
        } else {
            env.events().publish(
                (Symbol::new(&env, "SaleApprovalReceived"),),
                (approval_id, approver, approval.approvals_received.len())
            );
        }

        set_approval(&env, &approval);
    }

    /// Reject a proposed sale
    pub fn reject_sale(env: Env, approval_id: u64, approver: Address, reason: String) {
        approver.require_auth();

        if approval_id == 0 {
            panic!("Invalid approval ID");
        }

        let mut approval = get_approval(&env, approval_id).expect("Approval not found");

        assert!(approval.status == ApprovalStatus::Pending, "Approval not pending");
        assert!(env.ledger().timestamp() < approval.expires_at, "Approval expired");

        // Check if approver is authorized
        assert!(approval.approvers.contains(&approver), "Unauthorized approver");

        // Check if already voted
        assert!(!approval.approvals_received.contains(&approver), "Already approved");
        assert!(!approval.rejections_received.contains(&approver), "Already rejected");

        approval.rejections_received.push_back(approver.clone());
        approval.rejection_reasons.push_back(reason.clone());
        approval.status = ApprovalStatus::Rejected;

        // Add to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "rejected"),
            actor: approver.clone(),
            timestamp: env.ledger().timestamp(),
            reason: Some(reason),
        };
        add_approval_history(&env, approval_id, &history);

        env.events().publish((Symbol::new(&env, "SaleRejected"),), (approval_id, approver));

        set_approval(&env, &approval);
    }

    /// Execute an approved sale
    pub fn execute_approved_sale(env: Env, approval_id: u64) {
        if approval_id == 0 {
            panic!("Invalid approval ID");
        }

        let approval = get_approval(&env, approval_id).expect("Approval not found");

        assert!(approval.status == ApprovalStatus::Approved, "Approval not approved");
        assert!(env.ledger().timestamp() < approval.expires_at, "Approval expired");

        // Execute the sale based on type
        if let Some(listing_id) = approval.listing_id {
            // Fixed-price sale
            Marketplace::execute_approved_listing_sale(env, approval_id, listing_id);
        } else if let Some(auction_id) = approval.auction_id {
            // Auction sale
            Marketplace::execute_approved_auction_sale(env, approval_id, auction_id);
        } else {
            panic!("Invalid approval: no listing or auction ID");
        }
    }

    /// Execute approved fixed-price sale (internal function)
    fn execute_approved_listing_sale(env: Env, approval_id: u64, listing_id: u64) {
        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        let mut listing: Listing = env
            .storage()
            .instance()
            .get(&listing_key)
            .expect("Listing not found");

        let approval = get_approval(&env, approval_id).expect("Approval not found");

        // Mark listing as inactive
        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        // Update approval status
        let mut updated_approval = approval;
        updated_approval.status = ApprovalStatus::Executed;
        set_approval(&env, &updated_approval);

        // Add execution to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "executed"),
            actor: env.current_contract_address(),
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        add_approval_history(&env, approval_id, &history);

        env.events().publish(
            (Symbol::new(&env, "SaleExecuted"),),
            (approval_id, listing_id, updated_approval.buyer)
        );
    }

    /// Execute approved auction sale (internal function)
    fn execute_approved_auction_sale(env: Env, approval_id: u64, auction_id: u64) {
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        let approval = get_approval(&env, approval_id).expect("Approval not found");

        // Process the auction resolution
        if let Some(winner) = auction.highest_bidder.clone() {
            if auction.highest_bid >= auction.reserve_price {
                let royalty_info = Marketplace::get_royalty(env.clone(), auction.agent_id).expect(
                    "Royalty info not found"
                );

                let royalty = (((auction.highest_bid as u128) * (royalty_info.fee as u128)) /
                    10000) as i128;
                let seller_amount = auction.highest_bid - royalty;

                let token_client = token::Client::new(&env, &get_payment_token(&env));

                // Transfer royalty
                token_client.transfer(
                    &env.current_contract_address(),
                    &royalty_info.recipient,
                    &royalty
                );

                // Transfer seller payout
                token_client.transfer(
                    &env.current_contract_address(),
                    &auction.seller,
                    &seller_amount
                );

                // NOTE: NFT transfer logic should be added here

                auction.status = AuctionStatus::Won;

                env.events().publish(
                    (Symbol::new(&env, "AuctionWon"),),
                    (auction_id, winner, auction.highest_bid)
                );
            } else {
                // Refund if reserve not met
                let token_client = token::Client::new(&env, &get_payment_token(&env));
                token_client.transfer(
                    &env.current_contract_address(),
                    &winner,
                    &auction.highest_bid
                );
                auction.status = AuctionStatus::Ended;
            }
        } else {
            auction.status = AuctionStatus::Ended;
        }

        set_auction(&env, &auction);

        // Update approval status
        let mut updated_approval = approval;
        updated_approval.status = ApprovalStatus::Executed;
        set_approval(&env, &updated_approval);

        // Add execution to history
        let history = ApprovalHistory {
            approval_id,
            action: String::from_str(&env, "executed"),
            actor: env.current_contract_address(),
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        add_approval_history(&env, approval_id, &history);

        env.events().publish(
            (Symbol::new(&env, "SaleExecuted"),),
            (approval_id, auction_id, updated_approval.buyer)
        );
    }

    /// Get approval details
    pub fn get_approval(env: Env, approval_id: u64) -> Option<Approval> {
        if approval_id == 0 {
            panic!("Invalid approval ID");
        }
        get_approval(&env, approval_id)
    }

    /// Get approval history
    pub fn get_approval_history(env: Env, approval_id: u64) -> Vec<ApprovalHistory> {
        if approval_id == 0 {
            panic!("Invalid approval ID");
        }

        let history_count = get_approval_history_count(&env, approval_id);
        let mut history = Vec::new(&env);

        for i in 0..history_count {
            if let Some(entry) = get_approval_history(&env, approval_id, i) {
                history.push_back(entry);
            }
        }

        history
    }

    /// Clean up expired approvals (can be called by anyone)
    pub fn cleanup_expired_approvals(env: Env) {
        let counter = get_approval_counter(&env);
        let mut cleaned_count = 0u64;

        for approval_id in 1..=counter {
            if let Some(approval) = get_approval(&env, approval_id) {
                if
                    approval.status == ApprovalStatus::Pending &&
                    env.ledger().timestamp() >= approval.expires_at
                {
                    // Mark as expired
                    let mut expired_approval = approval;
                    expired_approval.status = ApprovalStatus::Expired;
                    set_approval(&env, &expired_approval);

                    // Add to history
                    let history = ApprovalHistory {
                        approval_id,
                        action: String::from_str(&env, "expired"),
                        actor: env.current_contract_address(),
                        timestamp: env.ledger().timestamp(),
                        reason: None,
                    };
                    add_approval_history(&env, approval_id, &history);

                    cleaned_count += 1;
                }
            }
        }

        if cleaned_count > 0 {
            env.events().publish((Symbol::new(&env, "ExpiredApprovalsCleaned"),), (cleaned_count,));
        }
    }

    // ---------------- AUCTIONS ----------------

    /// Dutch params: (start_price, end_price, duration_seconds, price_decay). Use (None,None,None,None) for non-Dutch.
    pub fn create_auction(
        env: Env,
        agent_id: u64,
        seller: Address,
        auction_type: AuctionType,
        start_price: i128,
        reserve_price: i128,
        duration: u64,
        min_bid_increment_bps: u32,
        dutch_params: (Option<i128>, Option<i128>, Option<u64>, Option<u32>),
    ) -> u64 {
        seller.require_auth();
        assert!(start_price > 0, "Invalid start price");
        assert!(duration > 0, "Invalid duration");

        let (dutch_start_price, dutch_end_price, dutch_duration_seconds, dutch_price_decay) = dutch_params;

        let auction_id = increment_auction_counter(&env);
        let start_time = env.ledger().timestamp();
        let end_time = start_time + duration;

        let auction = Auction {
            auction_id,
            agent_id,
            seller,
            auction_type,
            start_price,
            reserve_price,
            highest_bidder: None,
            highest_bid: 0,
            start_time,
            end_time,
            min_bid_increment_bps,
            status: AuctionStatus::Active,
            dutch_start_price,
            dutch_end_price,
            dutch_duration_seconds,
            dutch_price_decay,
        };

        set_auction(&env, &auction);

        env.events().publish(
            (Symbol::new(&env, "AuctionCreated"),),
            (auction_id, agent_id, auction_type, start_price)
        );

        auction_id
    }

    pub fn calculate_dutch_price(env: Env, auction_id: u64) -> i128 {
        let auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.auction_type == AuctionType::Dutch, "Not a Dutch auction");

        let start_price = auction.dutch_start_price.expect("Missing Dutch config");
        let end_price = auction.dutch_end_price.expect("Missing Dutch config");
        let duration = auction.dutch_duration_seconds.expect("Missing Dutch config");
        let price_decay = auction.dutch_price_decay.unwrap_or(0);

        let now = env.ledger().timestamp();

        if now <= auction.start_time {
            return start_price;
        }
        if now >= auction.end_time {
            return end_price;
        }

        let elapsed = now - auction.start_time;
        let price_range = start_price - end_price;
        if price_decay == 1 {
            start_price - (price_range * (elapsed as i128)) / (duration as i128)
        } else {
            start_price - (price_range * (elapsed as i128)) / (duration as i128)
        }
    }

    pub fn place_bid(env: Env, auction_id: u64, bidder: Address, amount: i128) {
        bidder.require_auth();
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.auction_type == AuctionType::English, "Not an English auction");
        assert!(env.ledger().timestamp() < auction.end_time, "Auction expired");

        let min_increment = (auction.highest_bid * (auction.min_bid_increment_bps as i128)) / 10000;
        let min_bid =
            auction.highest_bid + (if min_increment > 1000 { min_increment } else { 1000 });
        assert!(amount >= min_bid, "Bid too low");

        let token_client = token::Client::new(&env, &get_payment_token(&env));

        // Refund previous highest bidder
        if let Some(prev_bidder) = auction.highest_bidder {
            token_client.transfer(
                &env.current_contract_address(),
                &prev_bidder,
                &auction.highest_bid
            );
        }

        // Lock new bid in contract
        token_client.transfer(&bidder, &env.current_contract_address(), &amount);

        auction.highest_bidder = Some(bidder.clone());
        auction.highest_bid = amount;

        // Extend auction by 5 minutes if bid in final 5 minutes
        let time_left = auction.end_time - env.ledger().timestamp();
        if time_left < 300 {
            auction.end_time += 300;
        }

        set_auction(&env, &auction);

        env.events().publish(
            (Symbol::new(&env, "BidPlaced"),),
            (auction_id, bidder.clone(), amount, auction.end_time)
        );

        // Audit log for bid placement
        let before_state = String::from_str(&env, "{\"bid_placed\":false}");
        let after_state = String::from_str(&env, "{\"bid_placed\":true}");
        let tx_hash = String::from_str(&env, "place_bid");
        let description = Some(String::from_str(&env, "Auction bid placed"));
        
        let _ = create_audit_log(
            &env,
            bidder,
            OperationType::AuctionBidPlaced,
            before_state,
            after_state,
            tx_hash,
            description,
        );
    }

    pub fn accept_dutch_price(env: Env, auction_id: u64, buyer: Address) {
        buyer.require_auth();
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.auction_type == AuctionType::Dutch, "Not a Dutch auction");

        let current_price = Marketplace::calculate_dutch_price(env.clone(), auction_id);

        let token_client = token::Client::new(&env, &get_payment_token(&env));
        token_client.transfer(&buyer, &env.current_contract_address(), &current_price);

        auction.highest_bidder = Some(buyer);
        auction.highest_bid = current_price;

        set_auction(&env, &auction);

        Marketplace::resolve_auction(env, auction_id);
    }

    pub fn resolve_auction(env: Env, auction_id: u64) {
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.status == AuctionStatus::Active, "Auction not active");

        let is_dutch = auction.auction_type == AuctionType::Dutch;
        let is_english = auction.auction_type == AuctionType::English;

        assert!(
            (is_english && env.ledger().timestamp() >= auction.end_time) ||
                (is_dutch && auction.highest_bidder.is_some()),
            "Auction not yet ended"
        );

        if let Some(winner) = auction.highest_bidder.clone() {
            if auction.highest_bid >= auction.reserve_price {
                // Check if multi-signature approval is required
                let config = get_approval_config(&env);
                if auction.highest_bid >= config.threshold {
                    panic!(
                        "High-value auction requires multi-signature approval. Use propose_auction_sale() first."
                    );
                }

                let royalty_info = Marketplace::get_royalty(env.clone(), auction.agent_id).expect(
                    "Royalty info not found"
                );

                let royalty = (((auction.highest_bid as u128) * (royalty_info.fee as u128)) /
                    10000) as i128;
                let seller_amount = auction.highest_bid - royalty;

                let token_client = token::Client::new(&env, &get_payment_token(&env));

                // Transfer royalty
                token_client.transfer(
                    &env.current_contract_address(),
                    &royalty_info.recipient,
                    &royalty
                );

                // Transfer seller payout
                token_client.transfer(
                    &env.current_contract_address(),
                    &auction.seller,
                    &seller_amount
                );

                // NOTE: NFT transfer logic should be added here

                auction.status = AuctionStatus::Won;

                env.events().publish(
                    (Symbol::new(&env, "AuctionWon"),),
                    (auction_id, winner, auction.highest_bid)
                );
            } else {
                // Refund if reserve not met (English only)
                if is_english {
                    let token_client = token::Client::new(&env, &get_payment_token(&env));
                    token_client.transfer(
                        &env.current_contract_address(),
                        &winner,
                        &auction.highest_bid
                    );
                }
                auction.status = AuctionStatus::Ended;
            }
        } else {
            auction.status = AuctionStatus::Ended;
        }

        set_auction(&env, &auction);

        env.events().publish((Symbol::new(&env, "AuctionEnded"),), (auction_id, auction.status));
    }

    pub fn cancel_auction(env: Env, auction_id: u64) {
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        auction.seller.require_auth();
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.highest_bidder.is_none(), "Cannot cancel with active bids");

        auction.status = AuctionStatus::Cancelled;
        set_auction(&env, &auction);

        env.events().publish((Symbol::new(&env, "AuctionCancelled"),), (auction_id,));
    }

    // LEASE LIFECYCLE (issue #49)
    pub fn initiate_lease(env: Env, listing_id: u64, lessee: Address, duration_seconds: u64) -> u64 {
        lessee.require_auth();
        assert!(listing_id != 0, "Invalid listing ID");
        assert!(duration_seconds > 0, "Duration must be positive");
        let listing_key = (Symbol::new(&env, "listing"), listing_id);
        let mut listing: Listing = env.storage().instance().get(&listing_key).expect("Listing not found");
        assert!(listing.active, "Listing not active");
        assert!(listing.listing_type == ListingType::Lease, "Listing is not a lease");
        assert!(listing.seller != lessee, "Lessor cannot be lessee");
        let config = get_lease_config(&env);
        let total_value = listing.price;
        let deposit_amount = (total_value * (config.deposit_bps as i128)) / 10_000;
        let total_payment = total_value + deposit_amount;
        let token_client = token::Client::new(&env, &get_payment_token(&env));
        token_client.transfer(&lessee, &env.current_contract_address(), &total_payment);
        token_client.transfer(&env.current_contract_address(), &listing.seller, &total_value);
        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);
        let lease_id = increment_lease_counter(&env);
        let now = env.ledger().timestamp();
        let end_time = now + duration_seconds;
        let lease = stellai_lib::LeaseData {
            lease_id,
            agent_id: listing.agent_id,
            listing_id,
            lessor: listing.seller.clone(),
            lessee: lessee.clone(),
            start_time: now,
            end_time,
            duration_seconds,
            deposit_amount,
            total_value,
            auto_renew: false,
            lessee_consent_for_renewal: false,
            status: stellai_lib::LeaseState::Active,
            pending_extension_id: None,
        };
        set_lease(&env, &lease);
        lessee_leases_append(&env, &lessee, lease_id);
        lessor_leases_append(&env, &listing.seller, lease_id);
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "initiated"),
            actor: lessee.clone(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseInitiated"),), (lease_id, listing.agent_id, listing.seller, lessee, total_value, deposit_amount));
        lease_id
    }

    pub fn request_lease_extension(env: Env, lease_id: u64, lessee: Address, additional_duration_seconds: u64) -> u64 {
        lessee.require_auth();
        assert!(lease_id != 0, "Invalid lease ID");
        assert!(additional_duration_seconds > 0, "Additional duration must be positive");
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.lessee == lessee, "Unauthorized");
        assert!(lease.status == stellai_lib::LeaseState::Active, "Lease not active");
        assert!(env.ledger().timestamp() < lease.end_time, "Lease already ended");
        let extension_id = increment_lease_extension_counter(&env);
        let now = env.ledger().timestamp();
        let req = stellai_lib::LeaseExtensionRequest {
            extension_id,
            lease_id,
            additional_duration_seconds,
            requested_at: now,
            approved: false,
        };
        set_lease_extension_request(&env, &req);
        lease.status = stellai_lib::LeaseState::ExtensionRequested;
        lease.pending_extension_id = Some(extension_id);
        set_lease(&env, &lease);
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "extension_requested"),
            actor: lessee.clone(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseExtensionRequested"),), (lease_id, extension_id, lessee, additional_duration_seconds));
        extension_id
    }

    pub fn approve_lease_extension(env: Env, lease_id: u64, extension_id: u64, lessor: Address) {
        lessor.require_auth();
        assert!(lease_id != 0, "Invalid lease ID");
        assert!(extension_id != 0, "Invalid extension ID");
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.lessor == lessor, "Unauthorized");
        assert!(lease.status == stellai_lib::LeaseState::ExtensionRequested, "Lease not in extension requested state");
        assert!(lease.pending_extension_id == Some(extension_id), "Extension ID mismatch");
        let req = get_lease_extension_request(&env, extension_id).expect("Extension request not found");
        assert!(!req.approved, "Already approved");
        assert!(req.lease_id == lease_id, "Extension not for this lease");
        let now = env.ledger().timestamp();
        assert!(now <= req.requested_at + stellai_lib::LEASE_EXTENSION_REQUEST_TTL_SECONDS, "Extension request expired");
        lease.end_time += req.additional_duration_seconds;
        lease.duration_seconds += req.additional_duration_seconds;
        lease.status = stellai_lib::LeaseState::Active;
        lease.pending_extension_id = None;
        set_lease(&env, &lease);
        let mut approved_req = req.clone();
        approved_req.approved = true;
        set_lease_extension_request(&env, &approved_req);
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "extended"),
            actor: lessor.clone(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseExtended"),), (lease_id, extension_id, approved_req.additional_duration_seconds, lease.end_time));
    }

    pub fn early_termination(env: Env, lease_id: u64, lessee: Address, termination_fee_paid: i128) {
        lessee.require_auth();
        assert!(lease_id != 0, "Invalid lease ID");
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.lessee == lessee, "Unauthorized");
        assert!(lease.status == stellai_lib::LeaseState::Active, "Lease not active");
        assert!(env.ledger().timestamp() < lease.end_time, "Lease already ended");
        let config = get_lease_config(&env);
        let now = env.ledger().timestamp();
        let remaining_seconds = lease.end_time.saturating_sub(now) as i128;
        let total_seconds = lease.duration_seconds as i128;
        let remaining_value = if total_seconds > 0 { (lease.total_value * remaining_seconds) / total_seconds } else { 0 };
        let penalty = (remaining_value * (config.early_termination_penalty_bps as i128)) / 10_000;
        assert!(termination_fee_paid >= penalty, "Insufficient termination fee");
        let token_client = token::Client::new(&env, &get_payment_token(&env));
        if termination_fee_paid > 0 {
            token_client.transfer(&lessee, &env.current_contract_address(), &termination_fee_paid);
        }
        let refund_to_lessee = lease.deposit_amount - penalty;
        let refund_to_lessee = if refund_to_lessee < 0 { 0 } else { refund_to_lessee };
        let to_lessor = lease.deposit_amount - refund_to_lessee + termination_fee_paid;
        token_client.transfer(&env.current_contract_address(), &lease.lessor, &to_lessor);
        if refund_to_lessee > 0 {
            token_client.transfer(&env.current_contract_address(), &lessee, &refund_to_lessee);
        }
        lease.status = stellai_lib::LeaseState::Terminated;
        lease.pending_extension_id = None;
        set_lease(&env, &lease);
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "terminated"),
            actor: lessee.clone(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseTerminated"),), (lease_id, lessee, termination_fee_paid, refund_to_lessee));
    }

    pub fn settle_lease_expiry(env: Env, lease_id: u64) {
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.status == stellai_lib::LeaseState::Active || lease.status == stellai_lib::LeaseState::ExtensionRequested, "Lease not active");
        assert!(env.ledger().timestamp() >= lease.end_time, "Lease not yet expired");
        let token_client = token::Client::new(&env, &get_payment_token(&env));
        token_client.transfer(&env.current_contract_address(), &lease.lessee, &lease.deposit_amount);
        lease.status = stellai_lib::LeaseState::Terminated;
        lease.pending_extension_id = None;
        set_lease(&env, &lease);
        let now = env.ledger().timestamp();
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "expired"),
            actor: env.current_contract_address(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseExpired"),), (lease_id, lease.lessee, lease.deposit_amount));
    }

    pub fn set_lease_renewal_consent(env: Env, lease_id: u64, lessee: Address, consent: bool) {
        lessee.require_auth();
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.lessee == lessee, "Unauthorized");
        assert!(lease.status == stellai_lib::LeaseState::Active, "Lease not active");
        lease.lessee_consent_for_renewal = consent;
        set_lease(&env, &lease);
    }

    /// Lessor enables or disables automatic renewal for a lease.
    pub fn set_lease_auto_renew(env: Env, lease_id: u64, lessor: Address, auto_renew: bool) {
        lessor.require_auth();
        let mut lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(lease.lessor == lessor, "Unauthorized");
        assert!(lease.status == stellai_lib::LeaseState::Active, "Lease not active");
        lease.auto_renew = auto_renew;
        set_lease(&env, &lease);
    }

    pub fn process_lease_renewal(env: Env, lease_id: u64) -> u64 {
        let old_lease = get_lease(&env, lease_id).expect("Lease not found");
        assert!(old_lease.status == stellai_lib::LeaseState::Active || old_lease.status == stellai_lib::LeaseState::ExtensionRequested, "Lease not active");
        assert!(env.ledger().timestamp() >= old_lease.end_time, "Lease not yet expired");
        assert!(old_lease.auto_renew, "Auto renewal not configured");
        assert!(old_lease.lessee_consent_for_renewal, "Lessee has not consented to renewal");
        let now = env.ledger().timestamp();
        let new_lease_id = increment_lease_counter(&env);
        let new_end_time = now + old_lease.duration_seconds;
        let mut old_lease_updated = old_lease.clone();
        old_lease_updated.status = stellai_lib::LeaseState::Renewed;
        old_lease_updated.pending_extension_id = None;
        set_lease(&env, &old_lease_updated);
        let new_lease = stellai_lib::LeaseData {
            lease_id: new_lease_id,
            agent_id: old_lease.agent_id,
            listing_id: old_lease.listing_id,
            lessor: old_lease.lessor.clone(),
            lessee: old_lease.lessee.clone(),
            start_time: now,
            end_time: new_end_time,
            duration_seconds: old_lease.duration_seconds,
            deposit_amount: old_lease.deposit_amount,
            total_value: old_lease.total_value,
            auto_renew: old_lease.auto_renew,
            lessee_consent_for_renewal: false,
            status: stellai_lib::LeaseState::Active,
            pending_extension_id: None,
        };
        set_lease(&env, &new_lease);
        lessee_leases_append(&env, &old_lease.lessee, new_lease_id);
        lessor_leases_append(&env, &old_lease.lessor, new_lease_id);
        // Deposit is retained as the new lease's deposit (no transfer)
        let history_entry = stellai_lib::LeaseHistoryEntry {
            lease_id,
            action: String::from_str(&env, "renewed"),
            actor: env.current_contract_address(),
            timestamp: now,
            details: None,
        };
        add_lease_history(&env, lease_id, &history_entry);
        env.events().publish((Symbol::new(&env, "LeaseRenewed"),), (lease_id, new_lease_id, old_lease.lessee, new_end_time));
        new_lease_id
    }

    pub fn set_lease_config(env: Env, admin: Address, deposit_bps: u32, early_termination_penalty_bps: u32) {
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Contract not initialized");
        admin.require_auth();
        assert!(admin == current_admin, "Unauthorized");
        assert!(deposit_bps <= 10_000, "deposit_bps max 10000");
        assert!(early_termination_penalty_bps <= 10_000, "penalty_bps max 10000");
        let config = storage::LeaseConfig { deposit_bps, early_termination_penalty_bps };
        set_lease_config(&env, &config);
    }

    pub fn get_lease_by_id(env: Env, lease_id: u64) -> Option<stellai_lib::LeaseData> {
        get_lease(&env, lease_id)
    }

    pub fn get_active_leases(env: Env, address: Address) -> Vec<stellai_lib::LeaseData> {
        let lessee_ids = get_lessee_lease_ids(&env, &address);
        let lessor_ids = get_lessor_lease_ids(&env, &address);
        let mut out = Vec::new(&env);
        for i in 0..lessee_ids.len() {
            if let Some(id) = lessee_ids.get(i) {
                if let Some(lease) = get_lease(&env, id) {
                    if lease.status == stellai_lib::LeaseState::Active {
                        out.push_back(lease);
                    }
                }
            }
        }
        for i in 0..lessor_ids.len() {
            if let Some(id) = lessor_ids.get(i) {
                if let Some(lease) = get_lease(&env, id) {
                    if lease.status != stellai_lib::LeaseState::Active {
                        continue;
                    }
                    let mut found = false;
                    for j in 0..out.len() {
                        if let Some(existing) = out.get(j) {
                            if existing.lease_id == lease.lease_id {
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        out.push_back(lease);
                    }
                }
            }
        }
        out
    }

    pub fn get_lease_history(env: Env, lease_id: u64) -> Vec<stellai_lib::LeaseHistoryEntry> {
        let n = get_lease_history_count(&env, lease_id);
        let mut out = Vec::new(&env);
        for i in 0..n {
            if let Some(entry) = get_lease_history_entry(&env, lease_id, i) {
                out.push_back(entry);
            }
        }
        out
    }
}

#[cfg(test)]
mod test_approval;
#[cfg(test)]
mod test_lease;
