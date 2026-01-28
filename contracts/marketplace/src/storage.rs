use soroban_sdk::{Env, Address, contracttype};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    PaymentToken,
    RoyaltyBps,
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

pub fn set_royalty_bps(env: &Env, bps: u32) {
    env.storage().instance().set(&DataKey::RoyaltyBps, &bps);
}

pub fn get_royalty_bps(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::RoyaltyBps).unwrap()
}

/* ---------------- HELPERS ---------------- */

pub fn calculate_royalty(price: i128, bps: u32) -> i128 {
    price * bps as i128 / 10_000
}
