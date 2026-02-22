use soroban_sdk::{Address, BytesN, Env};

use super::{
    repository::LeaseRepository,
    types::{LeaseError, LeaseTerm, LeaseStatus},
};

/// Minimum lease length in ledgers (~5 minutes at 5 s/ledger).
const MIN_LEASE_DURATION_LEDGERS: u32 = 60;

/// Encapsulates all lease business rules.
///
/// The manager is the single entry-point for contract methods and
/// `ExecutionHub` permission checks. It delegates all storage I/O to
/// `LeaseRepository` and never calls `env.storage()` directly.
pub struct LeaseManager {
    repo: LeaseRepository,
    env: Env,
}

impl LeaseManager {
    pub fn new(env: Env) -> Self {
        Self {
            repo: LeaseRepository::new(env.clone()),
            env,
        }
    }

    // ------------------------------------------------------------------
    // Agent registration
    // ------------------------------------------------------------------

    /// Register `owner` as the lessor for `agent_id`.
    ///
    /// Should be called once when an agent is listed on the marketplace.
    /// Returns `Unauthorized` if the agent is already registered to a
    /// different address.
    pub fn register_agent(
        &self,
        agent_id: &BytesN<32>,
        owner: &Address,
    ) -> Result<(), LeaseError> {
        owner.require_auth();

        if let Some(existing) = self.repo.load_agent_owner(agent_id) {
            if &existing != owner {
                return Err(LeaseError::Unauthorized);
            }
        }

        self.repo.save_agent_owner(agent_id, owner);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Lease lifecycle
    // ------------------------------------------------------------------

    /// Create a new lease granting `lessee` execution rights on `agent_id`
    /// for the ledger range `[start_ledger, end_ledger)`.
    ///
    /// # Rules
    /// * Caller must be the registered agent owner.
    /// * `end_ledger > start_ledger + MIN_LEASE_DURATION_LEDGERS`.
    /// * No active lease may already exist for the same `(agent_id, lessee)`.
    pub fn create_lease(
        &self,
        agent_id: &BytesN<32>,
        lessor: &Address,
        lessee: &Address,
        start_ledger: u32,
        end_ledger: u32,
    ) -> Result<LeaseTerm, LeaseError> {
        lessor.require_auth();

        // Ownership check
        let owner = self
            .repo
            .load_agent_owner(agent_id)
            .ok_or(LeaseError::Unauthorized)?;
        if &owner != lessor {
            return Err(LeaseError::Unauthorized);
        }

        // Duration guard
        if end_ledger <= start_ledger
            || (end_ledger - start_ledger) < MIN_LEASE_DURATION_LEDGERS
        {
            return Err(LeaseError::InvalidLeaseTerm);
        }

        // Duplicate guard: reject if an Active lease already exists
        if let Some(existing) = self.repo.load_lease(agent_id, lessee) {
            if existing.status == LeaseStatus::Active {
                return Err(LeaseError::LeaseAlreadyExists);
            }
        }

        let term = LeaseTerm {
            lessor: lessor.clone(),
            lessee: lessee.clone(),
            start_ledger,
            end_ledger,
            status: LeaseStatus::Active,
        };

        self.repo.save_lease(agent_id, lessee, &term);
        self.repo.increment_lease_count(agent_id);

        Ok(term)
    }

    /// Revoke an active lease before it expires.
    ///
    /// Only the original lessor may revoke. The lease record is retained
    /// with status `Revoked` for audit purposes.
    pub fn revoke_lease(
        &self,
        agent_id: &BytesN<32>,
        lessor: &Address,
        lessee: &Address,
    ) -> Result<(), LeaseError> {
        lessor.require_auth();

        let term = self
            .repo
            .load_lease(agent_id, lessee)
            .ok_or(LeaseError::LeaseNotFound)?;

        if &term.lessor != lessor {
            return Err(LeaseError::Unauthorized);
        }

        self.repo.revoke_lease(agent_id, lessee);
        Ok(())
    }

    /// Settle expiry for a lease whose `end_ledger` has passed.
    ///
    /// Anyone may call this to sync on-chain status; it is also called
    /// lazily by `check_lease_permission` to keep status current.
    pub fn settle_expiry(
        &self,
        agent_id: &BytesN<32>,
        lessee: &Address,
    ) -> Result<(), LeaseError> {
        let term = self
            .repo
            .load_lease(agent_id, lessee)
            .ok_or(LeaseError::LeaseNotFound)?;

        let current = self.env.ledger().sequence();

        if term.status == LeaseStatus::Active && current >= term.end_ledger {
            self.repo.expire_lease(agent_id, lessee);
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Permission check — called by ExecutionHub
    // ------------------------------------------------------------------

    /// Returns `Ok(())` when `lessee` is permitted to execute `agent_id`
    /// at the current ledger, applying lazy expiry settlement.
    ///
    /// # Errors
    /// | Condition | Error |
    /// |---|---|
    /// | No lease on record | `LeaseNotFound` |
    /// | Lease revoked | `LeaseRevoked` |
    /// | `current_ledger >= end_ledger` | `LeaseExpired` |
    /// | `current_ledger < start_ledger` | `LeaseExpired` (not yet active) |
    pub fn check_lease_permission(
        &self,
        agent_id: &BytesN<32>,
        lessee: &Address,
    ) -> Result<(), LeaseError> {
        let mut term = self
            .repo
            .load_lease(agent_id, lessee)
            .ok_or(LeaseError::LeaseNotFound)?;

        let current = self.env.ledger().sequence();

        // Lazy expiry settlement
        if term.status == LeaseStatus::Active && current >= term.end_ledger {
            self.repo.expire_lease(agent_id, lessee);
            term.status = LeaseStatus::Expired;
        }

        match term.status {
            LeaseStatus::Revoked => Err(LeaseError::LeaseRevoked),
            LeaseStatus::Expired => Err(LeaseError::LeaseExpired),
            LeaseStatus::Active => {
                if !term.is_valid_at(current) {
                    Err(LeaseError::LeaseExpired)
                } else {
                    Ok(())
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Read-only queries
    // ------------------------------------------------------------------

    pub fn get_lease(
        &self,
        agent_id: &BytesN<32>,
        lessee: &Address,
    ) -> Option<LeaseTerm> {
        self.repo.load_lease(agent_id, lessee)
    }

    pub fn get_lease_count(&self, agent_id: &BytesN<32>) -> u64 {
        self.repo.get_lease_count(agent_id)
    }
}