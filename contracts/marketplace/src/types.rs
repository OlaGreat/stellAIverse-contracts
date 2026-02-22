use soroban_sdk::{contracttype, Address, BytesN};

use common_utils::IStorageKey;

// ---------------------------------------------------------------------------
// LeaseStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a lease.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LeaseStatus {
    /// Lease is active and the lessee may execute the agent.
    Active,
    /// Lease term has elapsed; execution is blocked.
    Expired,
    /// Lessor revoked the lease before expiry.
    Revoked,
}

// ---------------------------------------------------------------------------
// LeaseTerm
// ---------------------------------------------------------------------------

/// Full on-chain record of a single lease agreement.
///
/// Stored under `DataKey::Lease(agent_id, lessee)` in persistent storage.
#[contracttype]
#[derive(Clone, Debug)]
pub struct LeaseTerm {
    /// The address that owns the agent (lessor).
    pub lessor: Address,
    /// The address that has been granted temporary execution rights.
    pub lessee: Address,
    /// Soroban ledger sequence number when the lease becomes active.
    pub start_ledger: u32,
    /// Soroban ledger sequence number when the lease expires (exclusive).
    pub end_ledger: u32,
    /// Current lifecycle state.
    pub status: LeaseStatus,
}

impl LeaseTerm {
    /// Returns `true` when `current_ledger` is within the active window
    /// and the lease has not been revoked.
    pub fn is_valid_at(&self, current_ledger: u32) -> bool {
        self.status == LeaseStatus::Active
            && current_ledger >= self.start_ledger
            && current_ledger < self.end_ledger
    }
}

// ---------------------------------------------------------------------------
// DataKey
// ---------------------------------------------------------------------------

/// Storage keys for the marketplace leasing module.
///
/// All keys are scoped to `(agent_id, lessee)` pairs so multiple concurrent
/// leases on the same agent are supported.
#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    /// The lease record for a specific `(agent_id, lessee)` pair.
    Lease(BytesN<32>, Address),
    /// Count of active leases ever created for an agent (monotonic).
    LeaseCount(BytesN<32>),
    /// The owner / lessor of an agent.
    AgentOwner(BytesN<32>),
}

impl IStorageKey for DataKey {}

// ---------------------------------------------------------------------------
// LeaseError
// ---------------------------------------------------------------------------

/// Typed errors returned by leasing operations.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LeaseError {
    /// Caller is not the agent owner.
    Unauthorized,
    /// A lease already exists for this `(agent_id, lessee)` pair.
    LeaseAlreadyExists,
    /// No lease found for the `(agent_id, lessee)` pair.
    LeaseNotFound,
    /// The lease window is invalid (e.g. `end_ledger <= start_ledger`).
    InvalidLeaseTerm,
    /// The lease has expired.
    LeaseExpired,
    /// The lease was revoked by the lessor.
    LeaseRevoked,
}