#![no_std]

use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};
use stellai_lib::{
    Listing, ListingType, RoyaltyInfo, LISTING_COUNTER_KEY,
    Auction, AuctionType, AuctionStatus, DutchAuctionConfig, PriceDecay
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
        
        env.storage()
            .instance()
            .set(&Symbol::new(&env, LISTING_COUNTER_KEY), &0u64);
    }

    /// Set a new admin
    pub fn set_admin(env: Env, new_admin: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Contract not initialized");
        admin.require_auth();
        set_admin(&env, &new_admin);
    }

    /// Set the payment token
    pub fn set_payment_token(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Contract not initialized");
        assert!(admin == current_admin, "Unauthorized");
        
        set_payment_token(&env, token);
    }

    /// Create a new listing
    pub fn create_listing(
        env: Env,
        agent_id: u64,
        seller: Address,
        listing_type: u32,
        price: i128,
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
        env.storage()
            .instance()
            .set(&Symbol::new(&env, LISTING_COUNTER_KEY), &listing_id);

        env.events().publish(
            (Symbol::new(&env, "listing_created"),),
            (listing_id, agent_id, seller, price),
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

        // Mark listing as inactive
        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        env.events().publish(
            (Symbol::new(&env, "agent_sold"),),
            (listing_id, listing.agent_id, buyer),
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
            (listing_id, listing.agent_id, seller),
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

        env.events()
            .publish((Symbol::new(&env, "royalty_set"),), (agent_id, fee));
    }

    /// Get royalty info for an agent
    pub fn get_royalty(env: Env, agent_id: u64) -> Option<RoyaltyInfo> {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        let royalty_key = (Symbol::new(&env, "royalty"), agent_id);
        env.storage().instance().get(&royalty_key)
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
        dutch_config: Option<DutchAuctionConfig>,
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
            (auction_id, agent_id, auction_type, start_price),
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
                config.start_price - (price_range * elapsed as i128 / duration as i128)
            }
            PriceDecay::Exponential => {
                let price_range = config.start_price - config.end_price;
                config.start_price - (price_range * elapsed as i128 / duration as i128)
            }
        }
    }

    pub fn place_bid(env: Env, auction_id: u64, bidder: Address, amount: i128) {
        bidder.require_auth();
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.auction_type == AuctionType::English, "Not an English auction");
        assert!(env.ledger().timestamp() < auction.end_time, "Auction expired");

        let min_increment = (auction.highest_bid * auction.min_bid_increment_bps as i128) / 10000;
        let min_bid = auction.highest_bid + if min_increment > 1000 { min_increment } else { 1000 };
        assert!(amount >= min_bid, "Bid too low");

        let token_client = token::Client::new(&env, &get_payment_token(&env));

        // Refund previous highest bidder
        if let Some(prev_bidder) = auction.highest_bidder {
            token_client.transfer(&env.current_contract_address(), &prev_bidder, &auction.highest_bid);
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
            (auction_id, bidder, amount, auction.end_time),
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
                let royalty_info = Marketplace::get_royalty(env.clone(), auction.agent_id).expect("Royalty info not found");
                
                let royalty = ((auction.highest_bid as u128 * royalty_info.fee as u128) / 10000) as i128;
                let seller_amount = auction.highest_bid - royalty;

                let token_client = token::Client::new(&env, &get_payment_token(&env));

                // Transfer royalty
                token_client.transfer(&env.current_contract_address(), &royalty_info.recipient, &royalty);

                // Transfer seller payout
                token_client.transfer(&env.current_contract_address(), &auction.seller, &seller_amount);

                // NOTE: NFT transfer logic should be added here

                auction.status = AuctionStatus::Won;
                
                env.events().publish(
                    (Symbol::new(&env, "AuctionWon"),),
                    (auction_id, winner, auction.highest_bid),
                );
            } else {
                // Refund if reserve not met (English only)
                if is_english {
                    let token_client = token::Client::new(&env, &get_payment_token(&env));
                    token_client.transfer(&env.current_contract_address(), &winner, &auction.highest_bid);
                }
                auction.status = AuctionStatus::Ended;
            }
        } else {
            auction.status = AuctionStatus::Ended;
        }

        set_auction(&env, &auction);

        env.events().publish(
            (Symbol::new(&env, "AuctionEnded"),),
            (auction_id, auction.status),
        );
    }

    pub fn cancel_auction(env: Env, auction_id: u64) {
        let mut auction = get_auction(&env, auction_id).expect("Auction not found");
        auction.seller.require_auth();
        assert!(auction.status == AuctionStatus::Active, "Auction not active");
        assert!(auction.highest_bidder.is_none(), "Cannot cancel with active bids");

        auction.status = AuctionStatus::Cancelled;
        set_auction(&env, &auction);

        env.events().publish(
            (Symbol::new(&env, "AuctionCancelled"),),
            (auction_id,),
        );
    }
}
