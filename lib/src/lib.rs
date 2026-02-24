#![no_std]
pub mod errors;
pub mod audit;
pub mod audit_helpers;

#[cfg(test)]
mod audit_tests;

use soroban_sdk::{ contracttype, symbol_short, Address, Bytes, String, Symbol, Vec };

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
    pub listing_type: ListingType, // Sale, Lease, etc.
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
#[derive(Clone)]
#[contracttype]
pub struct RoyaltyInfo {
    pub recipient: Address,
    pub fee: u32, // 0-10000 representing 0-100%
}

/// Oracle attestation for evolution completion (signed by oracle provider)
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AuctionType {
    English = 0,
    Dutch = 1,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum AuctionStatus {
    Created = 0,
    Active = 1,
    Ended = 2,
    Cancelled = 3,
    Won = 4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
#[repr(u32)]
pub enum PriceDecay {
    Linear = 0,
    Exponential = 1,
}

#[derive(Clone)]
#[contracttype]
pub struct DutchAuctionConfig {
    pub start_price: i128,
    pub end_price: i128,
    pub duration_seconds: u64,
    pub price_decay: PriceDecay,
}

#[derive(Clone)]
#[contracttype]
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
    pub dutch_config_enabled: bool,
    pub dutch_config_start_price: i128,
    pub dutch_config_end_price: i128,
    pub dutch_config_duration_seconds: u64,
    pub dutch_config_price_decay: PriceDecay,
}

/// Multi-signature approval configuration for high-value sales
#[derive(Clone)]
#[contracttype]
pub struct ApprovalConfig {
    pub threshold: i128, // Price threshold in stroops (default: 10,000 USDC equivalent)
    pub approvers_required: u32, // N of M signatures required (default: 2)
    pub total_approvers: u32, // Total number of authorized approvers (default: 3)
    pub ttl_seconds: u64, // Time to live for approvals (default: 7 days = 604800 seconds)
}

/// Approval status for high-value transactions
#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub listing_id: Option<u64>, // For fixed-price sales
    pub auction_id: Option<u64>, // For auction sales
    pub buyer: Address,
    pub price: i128,
    pub proposed_at: u64,
    pub expires_at: u64,
    pub status: ApprovalStatus,
    pub required_approvals: u32,
    pub approvers: Vec<Address>, // All authorized approvers
    pub approvals_received: Vec<Address>, // Addresses that have approved
    pub rejections_received: Vec<Address>, // Addresses that have rejected
    pub rejection_reasons: Vec<String>, // Reasons for rejections
}

/// Approval history entry for audit trail
#[derive(Clone)]
#[contracttype]
pub struct ApprovalHistory {
    pub approval_id: u64,
    pub action: String, // "proposed", "approved", "rejected", "executed"
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

/// Constants for security hardening
// Config
pub const ADMIN_KEY: &str = "admin";
pub const MAX_STRING_LENGTH: u32 = 256;
pub const MAX_ROYALTY_FEE: u32 = 10000;
pub const MAX_DATA_SIZE: u32 = 65536;
pub const MAX_HISTORY_SIZE: u32 = 1000;
pub const MAX_HISTORY_QUERY_LIMIT: u32 = 500;
pub const DEFAULT_RATE_LIMIT_OPERATIONS: u32 = 100;
pub const DEFAULT_RATE_LIMIT_WINDOW_SECONDS: u64 = 60;
pub const MAX_CAPABILITIES: usize = 32;
pub const MAX_ROYALTY_PERCENTAGE: u32 = 10000; // 100%
pub const MIN_ROYALTY_PERCENTAGE: u32 = 0;
pub const SAFE_ARITHMETIC_CHECK_OVERFLOW: u128 = u128::MAX;
pub const PRICE_UPPER_BOUND: i128 = i128::MAX / 2; // Prevent overflow in calculations
pub const PRICE_LOWER_BOUND: i128 = 0; // Prevent negative prices
pub const MAX_DURATION_DAYS: u64 = 36500; // ~100 years max lease duration
pub const MAX_AGE_SECONDS: u64 = 365 * 24 * 60 * 60; // ~1 year max data age
pub const ATTESTATION_SIGNATURE_SIZE: usize = 64; // Ed25519 signature size
pub const MAX_ATTESTATION_DATA_SIZE: usize = 1024; // Max size for attestation data

// Storage keys
pub const EXEC_CTR_KEY: Symbol = symbol_short!("exec_ctr");
pub const REQUEST_COUNTER_KEY: &str = "request_counter";
pub const CLAIM_COOLDOWN_KEY: &str = "claim_cooldown";
pub const MAX_CLAIMS_PER_PERIOD_KEY: &str = "max_claims_per_period";
pub const TESTNET_FLAG_KEY: &str = "testnet_mode";
pub const DEFAULT_COOLDOWN_SECONDS: u64 = 86400; // 24 hours
pub const DEFAULT_MAX_CLAIMS: u32 = 1;
pub const LISTING_COUNTER_KEY: &str = "listing_counter";
pub const PROVIDER_LIST_KEY: &str = "providers";
pub const AGENT_COUNTER_KEY: &str = "agent_counter";
pub const AGENT_KEY_PREFIX: &str = "agent_";
pub const AGENT_LEASE_STATUS_PREFIX: &str = "agent_lease_";
pub const APPROVED_MINTERS_KEY: &str = "approved_minters";
pub const IMPLEMENTATION_KEY: Symbol = symbol_short!("impl_key");
pub const UPGRADE_HISTORY_KEY: Symbol = symbol_short!("up_hist");
pub const IS_PAUSED_KEY: Symbol = symbol_short!("is_paused");

// Approval constants
pub const APPROVAL_CONFIG_KEY: &str = "approval_config";
pub const APPROVAL_COUNTER_KEY: &str = "approval_counter";
pub const APPROVAL_KEY_PREFIX: &str = "approval_";
pub const APPROVAL_HISTORY_KEY_PREFIX: &str = "approval_history_";
pub const DEFAULT_APPROVAL_THRESHOLD: i128 = 10_000_000_000; // 10,000 USDC in stroops (assuming 7 decimals)
pub const DEFAULT_APPROVERS_REQUIRED: u32 = 2; // N of M
pub const DEFAULT_TOTAL_APPROVERS: u32 = 3; // Total authorized approvers
pub const DEFAULT_APPROVAL_TTL_SECONDS: u64 = 604800; // 7 days
