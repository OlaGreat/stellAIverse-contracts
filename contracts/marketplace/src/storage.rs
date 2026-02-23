use soroban_sdk::{ Env, Address, contracttype, Symbol, Vec, String };

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    PaymentToken,
    RoyaltyBps,
    Auction(u64),
    AuctionCounter,
    ApprovalConfig,
    ApprovalCounter,
    Approval(u64),
    ApprovalHistory(u64, u64), // (approval_id, history_index)
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
    (price * (bps as i128)) / 10_000
}

/* ---------------- APPROVAL ---------------- */

pub fn set_approval_config(env: &Env, config: &stellai_lib::ApprovalConfig) {
    env.storage().instance().set(&DataKey::ApprovalConfig, config);
}

pub fn get_approval_config(env: &Env) -> stellai_lib::ApprovalConfig {
    env.storage()
        .instance()
        .get(&DataKey::ApprovalConfig)
        .unwrap_or_else(|| stellai_lib::ApprovalConfig {
            threshold: stellai_lib::DEFAULT_APPROVAL_THRESHOLD,
            approvers_required: stellai_lib::DEFAULT_APPROVERS_REQUIRED,
            total_approvers: stellai_lib::DEFAULT_TOTAL_APPROVERS,
            ttl_seconds: stellai_lib::DEFAULT_APPROVAL_TTL_SECONDS,
        })
}

pub fn get_approval_counter(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::ApprovalCounter).unwrap_or(0)
}

pub fn set_approval_counter(env: &Env, counter: u64) {
    env.storage().instance().set(&DataKey::ApprovalCounter, counter);
}

pub fn increment_approval_counter(env: &Env) -> u64 {
    let counter = get_approval_counter(env) + 1;
    set_approval_counter(env, counter);
    counter
}

pub fn set_approval(env: &Env, approval: &stellai_lib::Approval) {
    env.storage().instance().set(&DataKey::Approval(approval.approval_id), approval);
}

pub fn get_approval(env: &Env, approval_id: u64) -> Option<stellai_lib::Approval> {
    env.storage().instance().get(&DataKey::Approval(approval_id))
}

pub fn add_approval_history(env: &Env, approval_id: u64, history: &stellai_lib::ApprovalHistory) {
    let history_index = get_approval_history_count(env, approval_id);
    env.storage().instance().set(&DataKey::ApprovalHistory(approval_id, history_index), history);
}

pub fn get_approval_history_count(env: &Env, approval_id: u64) -> u64 {
    let mut count = 0;
    while env.storage().instance().has(&DataKey::ApprovalHistory(approval_id, count)) {
        count += 1;
    }
    count
}

pub fn get_approval_history(
    env: &Env,
    approval_id: u64,
    index: u64
) -> Option<stellai_lib::ApprovalHistory> {
    env.storage().instance().get(&DataKey::ApprovalHistory(approval_id, index))
}

pub fn delete_approval(env: &Env, approval_id: u64) {
    env.storage().instance().remove(&DataKey::Approval(approval_id));

    // Clean up approval history
    let mut history_index = 0;
    while env.storage().instance().has(&DataKey::ApprovalHistory(approval_id, history_index)) {
        env.storage().instance().remove(&DataKey::ApprovalHistory(approval_id, history_index));
        history_index += 1;
    }
}
