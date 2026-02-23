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
    DutchAuctionConfig,
    PriceDecay,
    ApprovalConfig,
    Approval,
    ApprovalStatus,
    ApprovalHistory,
    DEFAULT_APPROVAL_THRESHOLD,
    DEFAULT_APPROVERS_REQUIRED,
    DEFAULT_TOTAL_APPROVERS,
    DEFAULT_APPROVAL_TTL_SECONDS,
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
            (listing_id, agent_id, seller, price)
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
            (listing_id, listing.agent_id, buyer)
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

        assert!(approvers.len() >= (config.approvers_required as usize), "Insufficient approvers");
        assert!(approvers.len() <= (config.total_approvers as usize), "Too many approvers");

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
            actor: buyer,
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

        assert!(approvers.len() >= (config.approvers_required as usize), "Insufficient approvers");
        assert!(approvers.len() <= (config.total_approvers as usize), "Too many approvers");

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
            actor: buyer,
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
        if approval.approvals_received.len() >= (approval.required_approvals as usize) {
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
            (approval_id, listing_id, approval.buyer)
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
            (approval_id, auction_id, approval.buyer)
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

    pub fn create_auction(
        env: Env,
        agent_id: u64,
        seller: Address,
        auction_type: AuctionType,
        start_price: i128,
        reserve_price: i128,
        duration: u64,
        min_bid_increment_bps: u32,
        dutch_config: Option<DutchAuctionConfig>
    ) -> u64 {
        seller.require_auth();
        assert!(start_price > 0, "Invalid start price");
        assert!(duration > 0, "Invalid duration");

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
            dutch_config,
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

        let config = auction.dutch_config.expect("Missing Dutch config");
        let now = env.ledger().timestamp();

        if now <= auction.start_time {
            return config.start_price;
        }
        if now >= auction.end_time {
            return config.end_price;
        }

        let elapsed = now - auction.start_time;
        let duration = config.duration_seconds;

        match config.price_decay {
            PriceDecay::Linear => {
                let price_range = config.start_price - config.end_price;
                config.start_price - (price_range * (elapsed as i128)) / (duration as i128)
            }
            PriceDecay::Exponential => {
                let price_range = config.start_price - config.end_price;
                config.start_price - (price_range * (elapsed as i128)) / (duration as i128)
            }
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
            (auction_id, bidder, amount, auction.end_time)
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
}

#[cfg(test)]
mod test_approval;
