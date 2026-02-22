use soroban_sdk::{Env, Address, contracttype};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    PaymentToken,
    RoyaltyBps,
    Auction(u64),
    AuctionCounter,
}

/* ---------------- ADMIN ---------------- */

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

#[allow(dead_code)]
pub fn require_admin(env: &Env) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
}

/* ---------------- PAYMENT TOKEN ---------------- */

pub fn set_payment_token(env: &Env, token: Address) {
    env.storage().instance().set(&DataKey::PaymentToken, &token);
}

pub fn get_payment_token(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::PaymentToken).unwrap()
}

/* ---------------- ROYALTY ---------------- */

#[allow(dead_code)]
pub fn set_royalty_bps(env: &Env, bps: u32) {
    env.storage().instance().set(&DataKey::RoyaltyBps, &bps);
}

#[allow(dead_code)]
pub fn get_royalty_bps(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::RoyaltyBps).unwrap()
}

/* ---------------- AUCTION ---------------- */

pub fn set_auction_counter(env: &Env, counter: u64) {
    env.storage().instance().set(&DataKey::AuctionCounter, &counter);
}

pub fn get_auction_counter(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::AuctionCounter).unwrap_or(0)
}

pub fn increment_auction_counter(env: &Env) -> u64 {
    let counter = get_auction_counter(env) + 1;
    set_auction_counter(env, counter);
    counter
}

pub fn set_auction(env: &Env, auction: &stellai_lib::Auction) {
    env.storage().instance().set(&DataKey::Auction(auction.auction_id), auction);
}

pub fn get_auction(env: &Env, auction_id: u64) -> Option<stellai_lib::Auction> {
    env.storage().instance().get(&DataKey::Auction(auction_id))
}

/* ---------------- HELPERS ---------------- */

#[allow(dead_code)]
pub fn calculate_royalty(price: i128, bps: u32) -> i128 {
    price * bps as i128 / 10_000
}
