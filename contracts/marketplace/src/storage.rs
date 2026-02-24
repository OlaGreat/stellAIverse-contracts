use soroban_sdk::{ Env, Address, contracttype, Vec };

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
    Lease(u64),
    LeaseCounter,
    LeaseExtensionCounter,
    LeaseExtensionRequest(u64),
    LeaseHistory(u64, u64),
    LesseeLeases(Address),
    LessorLeases(Address),
    LeaseConfig,
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
    env.storage().instance().set(&DataKey::ApprovalCounter, &counter);
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

#[allow(dead_code)]
pub fn delete_approval(env: &Env, approval_id: u64) {
    env.storage().instance().remove(&DataKey::Approval(approval_id));

    // Clean up approval history
    let mut history_index = 0;
    while env.storage().instance().has(&DataKey::ApprovalHistory(approval_id, history_index)) {
        env.storage().instance().remove(&DataKey::ApprovalHistory(approval_id, history_index));
        history_index += 1;
    }
}

/* ---------------- LEASE LIFECYCLE ---------------- */

#[derive(Clone)]
#[contracttype]
pub struct LeaseConfig {
    pub deposit_bps: u32,
    pub early_termination_penalty_bps: u32,
}

pub fn get_lease_config(env: &Env) -> LeaseConfig {
    env.storage()
        .instance()
        .get(&DataKey::LeaseConfig)
        .unwrap_or_else(|| LeaseConfig {
            deposit_bps: stellai_lib::DEFAULT_LEASE_DEPOSIT_BPS,
            early_termination_penalty_bps: stellai_lib::DEFAULT_EARLY_TERMINATION_PENALTY_BPS,
        })
}

pub fn set_lease_config(env: &Env, config: &LeaseConfig) {
    env.storage().instance().set(&DataKey::LeaseConfig, config);
}

pub fn get_lease_counter(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::LeaseCounter).unwrap_or(0)
}

pub fn increment_lease_counter(env: &Env) -> u64 {
    let c = get_lease_counter(env) + 1;
    env.storage().instance().set(&DataKey::LeaseCounter, &c);
    c
}

pub fn get_lease_extension_counter(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::LeaseExtensionCounter).unwrap_or(0)
}

pub fn increment_lease_extension_counter(env: &Env) -> u64 {
    let c = get_lease_extension_counter(env) + 1;
    env.storage().instance().set(&DataKey::LeaseExtensionCounter, &c);
    c
}

pub fn set_lease(env: &Env, lease: &stellai_lib::LeaseData) {
    env.storage().instance().set(&DataKey::Lease(lease.lease_id), lease);
}

pub fn get_lease(env: &Env, lease_id: u64) -> Option<stellai_lib::LeaseData> {
    env.storage().instance().get(&DataKey::Lease(lease_id))
}

pub fn set_lease_extension_request(env: &Env, req: &stellai_lib::LeaseExtensionRequest) {
    env.storage().instance().set(&DataKey::LeaseExtensionRequest(req.extension_id), req);
}

pub fn get_lease_extension_request(env: &Env, extension_id: u64) -> Option<stellai_lib::LeaseExtensionRequest> {
    env.storage().instance().get(&DataKey::LeaseExtensionRequest(extension_id))
}

pub fn add_lease_history(env: &Env, lease_id: u64, entry: &stellai_lib::LeaseHistoryEntry) {
    let idx = get_lease_history_count(env, lease_id);
    env.storage().instance().set(&DataKey::LeaseHistory(lease_id, idx), entry);
}

pub fn get_lease_history_count(env: &Env, lease_id: u64) -> u64 {
    let mut i = 0;
    while env.storage().instance().has(&DataKey::LeaseHistory(lease_id, i)) {
        i += 1;
    }
    i
}

pub fn get_lease_history_entry(env: &Env, lease_id: u64, index: u64) -> Option<stellai_lib::LeaseHistoryEntry> {
    env.storage().instance().get(&DataKey::LeaseHistory(lease_id, index))
}

pub fn lessee_leases_append(env: &Env, lessee: &Address, lease_id: u64) {
    let key = DataKey::LesseeLeases(lessee.clone());
    let mut vec: Vec<u64> = env.storage().instance().get(&key).unwrap_or_else(|| Vec::new(env));
    vec.push_back(lease_id);
    env.storage().instance().set(&key, &vec);
}

pub fn lessor_leases_append(env: &Env, lessor: &Address, lease_id: u64) {
    let key = DataKey::LessorLeases(lessor.clone());
    let mut vec: Vec<u64> = env.storage().instance().get(&key).unwrap_or_else(|| Vec::new(env));
    vec.push_back(lease_id);
    env.storage().instance().set(&key, &vec);
}

pub fn get_lessee_lease_ids(env: &Env, lessee: &Address) -> Vec<u64> {
    env.storage()
        .instance()
        .get(&DataKey::LesseeLeases(lessee.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_lessor_lease_ids(env: &Env, lessor: &Address) -> Vec<u64> {
    env.storage()
        .instance()
        .get(&DataKey::LessorLeases(lessor.clone()))
        .unwrap_or_else(|| Vec::new(env))
}
