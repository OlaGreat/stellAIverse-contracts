use soroban_sdk::{contracttype, Address, Bytes, String, Symbol, Val, Vec};

/// Oracle data entry
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleData {
    pub key: Symbol,
    pub value: i128,
    pub timestamp: u64,
    pub provider: Address,
    pub signature: Option<String>,
    pub source: Option<String>,
}

/// Represents an agent's metadata and state
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub struct Agent {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub model_hash: String,
    pub metadata_cid: String,
    pub capabilities: Vec<String>,
    pub evolution_level: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub nonce: u64,
    pub escrow_locked: bool,
    pub escrow_holder: Option<Address>,
}

/// Rate limiting window for security protection
#[derive(Clone, Copy)]
#[contracttype]
pub struct RateLimit {
    pub window_seconds: u64,
    pub max_operations: u32,
}

/// Represents a marketplace listing
#[derive(Clone)]
#[contracttype]
pub struct Listing {
    pub listing_id: u64,
    pub agent_id: u64,
    pub seller: Address,
    pub price: i128,
    pub listing_type: ListingType,
    pub active: bool,
    pub created_at: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum ListingType {
    Sale = 0,
    Lease = 1,
    Auction = 2,
}

/// Represents an evolution/upgrade request
#[derive(Clone)]
#[contracttype]
pub struct EvolutionRequest {
    pub request_id: u64,
    pub agent_id: u64,
    pub owner: Address,
    pub stake_amount: i128,
    pub status: EvolutionStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum EvolutionStatus {
    Pending = 0,
    InProgress = 1,
    Completed = 2,
    Failed = 3,
}

/// Royalty information for marketplace transactions
#[derive(Clone, Debug)]
#[contracttype]
pub struct RoyaltyInfo {
    pub recipient: Address,
    pub fee: u32,
}

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AuctionType {
    English = 0,
    Dutch = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum AuctionStatus {
    Created = 0,
    Active = 1,
    Ended = 2,
    Cancelled = 3,
    Won = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum PriceDecay {
    Linear = 0,
    Exponential = 1,
}

#[derive(Clone, Copy)]
#[contracttype]
pub struct DutchAuctionConfig {
    pub start_price: i128,
    pub end_price: i128,
    pub duration_seconds: u64,
    pub price_decay: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Auction {
    pub auction_id: u64,
    pub agent_id: u64,
    pub seller: Address,
    pub auction_type: AuctionType,
    pub start_price: i128,
    pub reserve_price: i128,
    pub highest_bidder: Option<Address>,
    pub highest_bid: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub min_bid_increment_bps: u32,
    pub status: AuctionStatus,
}

/// Multi-signature approval configuration for high-value sales
#[derive(Clone)]
#[contracttype]
pub struct ApprovalConfig {
    pub threshold: i128,
    pub approvers_required: u32,
    pub total_approvers: u32,
    pub ttl_seconds: u64,
}

/// Approval status for high-value transactions
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
#[repr(u32)]
pub enum ApprovalStatus {
    Pending = 0,
    Approved = 1,
    Rejected = 2,
    Expired = 3,
    Executed = 4,
}

/// Multi-signature approval for high-value agent sales
#[derive(Clone)]
#[contracttype]
pub struct Approval {
    pub approval_id: u64,
    pub listing_id: Option<u64>,
    pub auction_id: Option<u64>,
    pub buyer: Address,
    pub price: i128,
    pub proposed_at: u64,
    pub expires_at: u64,
    pub status: ApprovalStatus,
    pub required_approvals: u32,
    pub approvers: Vec<Address>,
    pub approvals_received: Vec<Address>,
    pub rejections_received: Vec<Address>,
    pub rejection_reasons: Vec<String>,
}

/// Approval history entry for audit trail
#[derive(Clone)]
#[contracttype]
pub struct ApprovalHistory {
    pub approval_id: u64,
    pub action: String,
    pub actor: Address,
    pub timestamp: u64,
    pub reason: Option<String>,
}

pub struct EvolutionAttestation {
    pub request_id: u64,
    pub agent_id: u64,
    pub oracle_provider: Address,
    pub new_model_hash: String,
    pub attestation_data: Bytes,
    pub signature: Bytes,
    pub timestamp: u64,
    pub nonce: u64,
}

/// State of a lease in its lifecycle.
#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum LeaseState {
    Active = 0,
    ExtensionRequested = 1,
    Terminated = 2,
    Renewed = 3,
}

/// Full lease record: duration, renewal terms, termination conditions, deposit.
#[derive(Clone)]
#[contracttype]
pub struct LeaseData {
    pub lease_id: u64,
    pub agent_id: u64,
    pub listing_id: u64,
    pub lessor: Address,
    pub lessee: Address,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_seconds: u64,
    pub deposit_amount: i128,
    pub total_value: i128,
    pub auto_renew: bool,
    pub lessee_consent_for_renewal: bool,
    pub status: LeaseState,
    pub pending_extension_id: Option<u64>,
}

/// A request to extend an active lease by additional duration.
#[derive(Clone)]
#[contracttype]
pub struct LeaseExtensionRequest {
    pub extension_id: u64,
    pub lease_id: u64,
    pub additional_duration_seconds: u64,
    pub requested_at: u64,
    pub approved: bool,
}

/// Single entry in lease history (for lessee/lessor audit).
#[derive(Clone)]
#[contracttype]
pub struct LeaseHistoryEntry {
    pub lease_id: u64,
    pub action: String,
    pub actor: Address,
    pub timestamp: u64,
    pub details: Option<String>,
}

/// Transaction status in the two-phase commit protocol
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
#[repr(u32)]
pub enum TransactionStatus {
    Initiated = 0,
    Preparing = 1,
    Prepared = 2,
    Committing = 3,
    Committed = 4,
    RollingBack = 5,
    RolledBack = 6,
    Failed = 7,
    TimedOut = 8,
}

/// Individual step in an atomic transaction
#[derive(Clone)]
#[contracttype]
pub struct TransactionStep {
    pub step_id: u32,
    pub contract: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub depends_on: Option<u32>,
    pub rollback_contract: Option<Address>,
    pub rollback_function: Option<Symbol>,
    pub rollback_args: Option<Vec<Val>>,
    pub executed: bool,
    pub result: Option<String>,
}

/// Atomic transaction containing multiple coordinated steps
#[derive(Clone)]
#[contracttype]
pub struct AtomicTransaction {
    pub transaction_id: u64,
    pub initiator: Address,
    pub steps: Vec<TransactionStep>,
    pub status: TransactionStatus,
    pub created_at: u64,
    pub deadline: u64,
    pub prepared_steps: Vec<u32>,
    pub executed_steps: Vec<u32>,
    pub failure_reason: Option<String>,
}

/// Journal entry for transaction recovery and replay
#[derive(Clone)]
#[contracttype]
pub struct TransactionJournalEntry {
    pub transaction_id: u64,
    pub step_id: u32,
    pub action: String,
    pub timestamp: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub state_snapshot: Option<String>,
}

/// Transaction progress event for monitoring
#[derive(Clone)]
#[contracttype]
pub struct TransactionEvent {
    pub transaction_id: u64,
    pub event_type: String,
    pub step_id: Option<u32>,
    pub timestamp: u64,
    pub details: Option<String>,
}
