/*!
# Agent Leasing Module

Provides time-bounded execution rights delegation for agents listed on
the marketplace.

## Architecture

```
LeaseManager          ← business logic, permission gate
    └── LeaseRepository   ← storage I/O via StorageRepository<DataKey>
            └── PersistentStorageRepository  (from common-utils)
```

## Typical flow

```
1. Agent owner calls `register_agent(agent_id, owner)`.
2. Lessor calls `create_lease(agent_id, lessor, lessee, start, end)`.
3. ExecutionHub calls `check_lease_permission(agent_id, lessee)` before
   every execution — returns Ok or a typed LeaseError.
4. On natural expiry `settle_expiry` is called lazily inside step 3.
5. Lessor may call `revoke_lease` at any time to terminate early.
```

## Storage keys

| Key | Tier | Description |
|---|---|---|
| `Lease(agent_id, lessee)` | Persistent | Full `LeaseTerm` struct |
| `AgentOwner(agent_id)` | Persistent | Owner address |
| `LeaseCount(agent_id)` | Persistent | Monotonic counter |
*/

pub mod manager;
pub mod repository;
pub mod types;

pub use manager::LeaseManager;
pub use types::{DataKey, LeaseError, LeaseTerm, LeaseStatus};