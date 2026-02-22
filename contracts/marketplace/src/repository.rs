use soroban_sdk::{Address, BytesN, Env};

use common_utils::{PersistentStorageRepository, StorageRepository};

use super::types::{DataKey, LeaseTerm, LeaseStatus};

/// TTL constants (in ledgers).
///
/// Soroban ledgers close roughly every 5 seconds, so:
/// * `LEASE_TTL_THRESHOLD` ≈ 30 days of ledger time before auto-bump.
/// * `LEASE_TTL_EXTEND`    ≈ 60 days – keeps lease record accessible for
///   auditing even after expiry.
const LEASE_TTL_THRESHOLD: u32 = 518_400;  // ~30 days
const LEASE_TTL_EXTEND: u32    = 1_036_800; // ~60 days

/// Handles all persistent-storage interactions for the leasing module.
///
/// Every method operates through `PersistentStorageRepository` so the
/// business logic in `LeaseManager` never touches `env.storage()` directly.
pub struct LeaseRepository {
    repo: PersistentStorageRepository,
}

impl LeaseRepository {
    pub fn new(env: Env) -> Self {
        Self {
            repo: PersistentStorageRepository::new(env),
        }
    }

    // ------------------------------------------------------------------
    // LeaseTerm CRUD
    // ------------------------------------------------------------------

    pub fn save_lease(&self, agent_id: &BytesN<32>, lessee: &Address, term: &LeaseTerm) {
        let key = DataKey::Lease(agent_id.clone(), lessee.clone());
        self.repo.set(&key, term);
        self.repo.extend_ttl(&key, LEASE_TTL_THRESHOLD, LEASE_TTL_EXTEND);
    }

    pub fn load_lease(&self, agent_id: &BytesN<32>, lessee: &Address) -> Option<LeaseTerm> {
        self.repo.get(&DataKey::Lease(agent_id.clone(), lessee.clone()))
    }

    pub fn lease_exists(&self, agent_id: &BytesN<32>, lessee: &Address) -> bool {
        self.repo.has(&DataKey::Lease(agent_id.clone(), lessee.clone()))
    }

    // ------------------------------------------------------------------
    // Mutation helpers
    // ------------------------------------------------------------------

    /// Mark a lease as `Expired` in storage (idempotent).
    pub fn expire_lease(&self, agent_id: &BytesN<32>, lessee: &Address) {
        if let Some(mut term) = self.load_lease(agent_id, lessee) {
            term.status = LeaseStatus::Expired;
            self.save_lease(agent_id, lessee, &term);
        }
    }

    /// Mark a lease as `Revoked` in storage (idempotent).
    pub fn revoke_lease(&self, agent_id: &BytesN<32>, lessee: &Address) {
        if let Some(mut term) = self.load_lease(agent_id, lessee) {
            term.status = LeaseStatus::Revoked;
            self.save_lease(agent_id, lessee, &term);
        }
    }

    // ------------------------------------------------------------------
    // Agent ownership
    // ------------------------------------------------------------------

    pub fn save_agent_owner(&self, agent_id: &BytesN<32>, owner: &Address) {
        let key = DataKey::AgentOwner(agent_id.clone());
        self.repo.set(&key, owner);
        self.repo.extend_ttl(&key, LEASE_TTL_THRESHOLD, LEASE_TTL_EXTEND);
    }

    pub fn load_agent_owner(&self, agent_id: &BytesN<32>) -> Option<Address> {
        self.repo.get(&DataKey::AgentOwner(agent_id.clone()))
    }

    // ------------------------------------------------------------------
    // Lease counter (monotonic, used for analytics / indexing)
    // ------------------------------------------------------------------

    pub fn increment_lease_count(&self, agent_id: &BytesN<32>) -> u64 {
        let key = DataKey::LeaseCount(agent_id.clone());
        let count: u64 = self.repo.get(&key).unwrap_or(0);
        let next = count + 1;
        self.repo.set(&key, &next);
        self.repo.extend_ttl(&key, LEASE_TTL_THRESHOLD, LEASE_TTL_EXTEND);
        next
    }

    pub fn get_lease_count(&self, agent_id: &BytesN<32>) -> u64 {
        self.repo.get(&DataKey::LeaseCount(agent_id.clone())).unwrap_or(0)
    }
}