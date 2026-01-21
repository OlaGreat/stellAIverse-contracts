#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol};

const ADMIN_KEY: &str = "admin";
const LISTING_COUNTER_KEY: &str = "listing_counter";
const LISTING_KEY_PREFIX: &str = "listing_";
const ROYALTY_KEY_PREFIX: &str = "royalty_";

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    /// Initialize contract with admin
    pub fn init_contract(env: Env, admin: Address) {
        let admin_data = env.storage().instance().get::<_, Address>(&Symbol::new(&env, ADMIN_KEY));
        if admin_data.is_some() {
            panic!("Contract already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
        env.storage().instance().set(&Symbol::new(&env, LISTING_COUNTER_KEY), &0u64);
    }

    /// Safe addition with overflow checks
    fn safe_add(a: u64, b: u64) -> u64 {
        a.checked_add(b).expect("Arithmetic overflow in safe_add")
    }

    /// Safe multiplication with overflow checks for price calculations
    fn safe_mul_i128(a: i128, b: u32) -> i128 {
        a.checked_mul(b as i128).expect("Arithmetic overflow in multiplication")
    }

    /// Create a new listing with comprehensive validation
    pub fn create_listing(
        env: Env,
        agent_id: u64,
        seller: Address,
        listing_type: u32, // 0=Sale, 1=Lease, 2=Auction
        price: i128,
        duration_days: Option<u64>, // For leases
    ) -> u64 {
        seller.require_auth();

        // Input validation
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if listing_type > 2 {
            panic!("Invalid listing type");
        }

        // Price bounds checking to prevent overflow/underflow
        if price < shared::PRICE_LOWER_BOUND || price > shared::PRICE_UPPER_BOUND {
            panic!("Price out of valid range");
        }

        // Validate lease duration if applicable
        if listing_type == 1 {
            let duration = duration_days.expect("Duration required for lease listings");
            if duration == 0 || duration > shared::MAX_DURATION_DAYS {
                panic!("Lease duration out of valid range");
            }
        }

        // Verify agent exists and seller is owner
        let agent_key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        let agent: shared::Agent = env.storage()
            .instance()
            .get(&agent_key)
            .expect("Agent not found");

        if agent.owner != seller {
            panic!("Unauthorized: only agent owner can create listings");
        }

        // Generate listing ID safely
        let counter: u64 = env.storage()
            .instance()
            .get(&Symbol::new(&env, LISTING_COUNTER_KEY))
            .unwrap_or(0);
        let listing_id = Self::safe_add(counter, 1);

        // Create listing
        let listing = shared::Listing {
            listing_id,
            agent_id,
            seller: seller.clone(),
            price,
            listing_type: match listing_type {
                0 => shared::ListingType::Sale,
                1 => shared::ListingType::Lease,
                2 => shared::ListingType::Auction,
                _ => panic!("Invalid listing type"),
            },
            active: true,
            created_at: env.ledger().timestamp(),
        };

        // Store listing
        let key = String::from_slice(&env, &format!("{}{}", LISTING_KEY_PREFIX, listing_id).as_bytes());
        env.storage().instance().set(&key, &listing);
        
        // Update counter
        env.storage().instance().set(&Symbol::new(&env, LISTING_COUNTER_KEY), &listing_id);

        env.events().publish(
            (Symbol::new(&env, "listing_created"),),
            (listing_id, agent_id, seller, price)
        );

        listing_id
    }

    /// Purchase or lease an agent with comprehensive security checks
    pub fn buy_agent(
        env: Env,
        listing_id: u64,
        buyer: Address,
        _payment_token: Address, // In production, would transfer from this token contract
        amount: i128,
    ) {
        buyer.require_auth();

        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        // Get listing
        let listing_key = String::from_slice(&env, &format!("{}{}", LISTING_KEY_PREFIX, listing_id).as_bytes());
        let mut listing: shared::Listing = env.storage()
            .instance()
            .get(&listing_key)
            .expect("Listing not found");

        // Validation checks
        if !listing.active {
            panic!("Listing is not active");
        }
        if amount < listing.price {
            panic!("Insufficient payment amount");
        }

        // Prevent payment overflow issues
        if amount > shared::PRICE_UPPER_BOUND {
            panic!("Payment amount exceeds safe maximum");
        }

        // Get royalty info if exists
        let royalty_key = String::from_slice(&env, 
            &format!("{}{}", ROYALTY_KEY_PREFIX, listing.agent_id).as_bytes()
        );
        let royalty_info: Option<shared::RoyaltyInfo> = env.storage()
            .instance()
            .get(&royalty_key);

        // Calculate and validate royalty (if exists)
        let mut royalty_amount: i128 = 0;
        if let Some(royalty) = &royalty_info {
            if royalty.percentage > shared::MAX_ROYALTY_PERCENTAGE {
                panic!("Invalid royalty percentage");
            }
            // Safe calculation: (amount * percentage) / 10000
            royalty_amount = Self::safe_mul_i128(amount, royalty.percentage)
                .checked_div(10000)
                .expect("Division by zero");
        }

        // Calculate seller amount (with safe arithmetic)
        let seller_amount = amount
            .checked_sub(royalty_amount)
            .expect("Arithmetic underflow in seller amount calculation");

        // In production:
        // - Transfer payment_token from buyer to seller
        // - Transfer royalty to royalty recipient
        // - Transfer agent NFT from seller to buyer
        // - Update agent ownership

        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        env.events().publish(
            (Symbol::new(&env, "agent_sold"),),
            (listing_id, listing.agent_id, buyer.clone(), seller_amount, royalty_amount)
        );
    }

    /// Cancel a listing with proper authorization
    pub fn cancel_listing(env: Env, listing_id: u64, seller: Address) {
        seller.require_auth();

        if listing_id == 0 {
            panic!("Invalid listing ID");
        }

        let listing_key = String::from_slice(&env, &format!("{}{}", LISTING_KEY_PREFIX, listing_id).as_bytes());
        let mut listing: shared::Listing = env.storage()
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

    /// Get active listings (with pagination to prevent DoS)
    pub fn get_listings(
        env: Env,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<shared::Listing> {
        // Limit query size to prevent DoS
        if limit > 100 || limit == 0 {
            panic!("Limit must be between 1 and 100");
        }
        if offset > 1_000_000 {
            panic!("Offset exceeds maximum allowed");
        }

        // In production, this would query from a more efficient data structure
        // For now, returning empty vec - would iterate stored listings
        soroban_sdk::Vec::new(&env)
    }

    /// Set royalty info for an agent with validation
    pub fn set_royalty(
        env: Env,
        agent_id: u64,
        creator: Address,
        recipient: Address,
        percentage: u32,
    ) {
        creator.require_auth();

        if agent_id == 0 {
            panic!("Invalid agent ID");
        }
        if percentage > shared::MAX_ROYALTY_PERCENTAGE {
            panic!("Royalty percentage exceeds maximum (100%)");
        }

        // Get agent to verify caller is creator
        let agent_key = String::from_slice(&env, &format!("agent_{}", agent_id).as_bytes());
        let agent: shared::Agent = env.storage()
            .instance()
            .get(&agent_key)
            .expect("Agent not found");

        if agent.owner != creator {
            panic!("Unauthorized: only agent creator can set royalty");
        }

        let royalty_info = shared::RoyaltyInfo {
            recipient,
            percentage,
        };

        let royalty_key = String::from_slice(&env,
            &format!("{}{}", ROYALTY_KEY_PREFIX, agent_id).as_bytes()
        );
        env.storage().instance().set(&royalty_key, &royalty_info);

        env.events().publish(
            (Symbol::new(&env, "royalty_set"),),
            (agent_id, percentage)
        );
    }

    /// Get royalty info for an agent
    pub fn get_royalty(env: Env, agent_id: u64) -> Option<shared::RoyaltyInfo> {
        if agent_id == 0 {
            panic!("Invalid agent ID");
        }

        let royalty_key = String::from_slice(&env,
            &format!("{}{}", ROYALTY_KEY_PREFIX, agent_id).as_bytes()
        );
        env.storage().instance().get(&royalty_key)
    }
}

