#![no_std]

use soroban_sdk::{Address, Env, Symbol, Vec, Val, String, contracttype};
use stellai_lib::{
    atomic::AtomicTransactionSupport,
    Listing, ListingType,
};
use crate::storage::*;

/// Atomic transaction state for marketplace operations
#[derive(Clone)]
#[contracttype]
pub struct AtomicState {
    pub transaction_id: u64,
    pub step_id: u32,
    pub locked_listing_id: Option<u64>,
    pub escrowed_amount: Option<i128>,
    pub original_listing_state: Option<Listing>,
}

/// Storage key for atomic transaction state
#[derive(Clone)]
#[contracttype]
pub enum AtomicDataKey {
    AtomicState(u64, u32), // (transaction_id, step_id)
    LockedListing(u64),    // listing_id
    EscrowBalance(Address, u64), // (buyer, transaction_id)
}

pub struct MarketplaceAtomicSupport;

impl AtomicTransactionSupport for MarketplaceAtomicSupport {
    /// Prepare phase: validate and lock resources without committing changes
    fn prepare_step(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        function: &Symbol,
        args: &Vec<Val>,
    ) -> bool {
        let function_name = function.to_string();
        
        match function_name.as_str() {
            "validate_listing" => Self::prepare_validate_listing(env, transaction_id, step_id, args),
            "transfer_to_escrow" => Self::prepare_transfer_to_escrow(env, transaction_id, step_id, args),
            "complete_sale" => Self::prepare_complete_sale(env, transaction_id, step_id, args),
            "validate_lease_listing" => Self::prepare_validate_lease_listing(env, transaction_id, step_id, args),
            "create_lease_record" => Self::prepare_create_lease_record(env, transaction_id, step_id, args),
            _ => false, // Unknown function
        }
    }

    /// Commit phase: execute the prepared step
    fn commit_step(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        function: &Symbol,
        args: &Vec<Val>,
    ) -> Val {
        let function_name = function.to_string();
        
        match function_name.as_str() {
            "validate_listing" => Self::commit_validate_listing(env, transaction_id, step_id, args),
            "transfer_to_escrow" => Self::commit_transfer_to_escrow(env, transaction_id, step_id, args),
            "complete_sale" => Self::commit_complete_sale(env, transaction_id, step_id, args),
            "validate_lease_listing" => Self::commit_validate_lease_listing(env, transaction_id, step_id, args),
            "create_lease_record" => Self::commit_create_lease_record(env, transaction_id, step_id, args),
            _ => Val::from_bool(false), // Unknown function
        }
    }

    /// Rollback phase: undo the effects of a committed step
    fn rollback_step(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        rollback_function: &Symbol,
        rollback_args: &Vec<Val>,
    ) -> bool {
        let function_name = rollback_function.to_string();
        
        match function_name.as_str() {
            "unlock_listing" => Self::rollback_unlock_listing(env, transaction_id, step_id, rollback_args),
            "refund_from_escrow" => Self::rollback_refund_from_escrow(env, transaction_id, step_id, rollback_args),
            "revert_sale" => Self::rollback_revert_sale(env, transaction_id, step_id, rollback_args),
            "unlock_lease_listing" => Self::rollback_unlock_lease_listing(env, transaction_id, step_id, rollback_args),
            "delete_lease_record" => Self::rollback_delete_lease_record(env, transaction_id, step_id, rollback_args),
            _ => false, // Unknown rollback function
        }
    }

    /// Check if a step is prepared and ready for commit
    fn is_step_prepared(env: &Env, transaction_id: u64, step_id: u32) -> bool {
        env.storage().instance().has(&AtomicDataKey::AtomicState(transaction_id, step_id))
    }

    /// Get step execution result for dependent steps
    fn get_step_result(env: &Env, transaction_id: u64, step_id: u32) -> Option<Val> {
        let state: Option<AtomicState> = env.storage().instance()
            .get(&AtomicDataKey::AtomicState(transaction_id, step_id));
        
        // Return success indicator for dependent steps
        state.map(|_| Val::from_bool(true))
    }
}

impl MarketplaceAtomicSupport {
    // ============ PREPARE PHASE IMPLEMENTATIONS ============

    fn prepare_validate_listing(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 3 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let agent_id: u64 = args.get(1).unwrap().try_into().unwrap_or(0);
        let expected_price: i128 = args.get(2).unwrap().try_into().unwrap_or(0);

        if listing_id == 0 || agent_id == 0 || expected_price <= 0 {
            return false;
        }

        // Check if listing is already locked
        if env.storage().instance().has(&AtomicDataKey::LockedListing(listing_id)) {
            return false;
        }

        let listing_key = (Symbol::new(env, "listing"), listing_id);
        let listing: Option<Listing> = env.storage().instance().get(&listing_key);

        match listing {
            Some(listing) => {
                if !listing.active || listing.agent_id != agent_id || listing.price != expected_price {
                    return false;
                }

                // Lock the listing
                env.storage().instance().set(&AtomicDataKey::LockedListing(listing_id), &transaction_id);

                // Store atomic state
                let state = AtomicState {
                    transaction_id,
                    step_id,
                    locked_listing_id: Some(listing_id),
                    escrowed_amount: None,
                    original_listing_state: Some(listing),
                };
                env.storage().instance().set(&AtomicDataKey::AtomicState(transaction_id, step_id), &state);

                true
            }
            None => false,
        }
    }

    fn prepare_transfer_to_escrow(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 3 {
            return false;
        }

        let buyer: Address = args.get(0).unwrap().try_into().unwrap();
        let seller: Address = args.get(1).unwrap().try_into().unwrap();
        let amount: i128 = args.get(2).unwrap().try_into().unwrap_or(0);

        if amount <= 0 {
            return false;
        }

        // In a real implementation, you would check token balance here
        // For now, we'll assume the buyer has sufficient balance

        // Store atomic state
        let state = AtomicState {
            transaction_id,
            step_id,
            locked_listing_id: None,
            escrowed_amount: Some(amount),
            original_listing_state: None,
        };
        env.storage().instance().set(&AtomicDataKey::AtomicState(transaction_id, step_id), &state);

        true
    }

    fn prepare_complete_sale(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 3 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let buyer: Address = args.get(1).unwrap().try_into().unwrap();
        let price: i128 = args.get(2).unwrap().try_into().unwrap_or(0);

        if listing_id == 0 || price <= 0 {
            return false;
        }

        // Verify listing is locked by this transaction
        let locked_tx_id: Option<u64> = env.storage().instance()
            .get(&AtomicDataKey::LockedListing(listing_id));
        
        if locked_tx_id != Some(transaction_id) {
            return false;
        }

        // Store atomic state
        let state = AtomicState {
            transaction_id,
            step_id,
            locked_listing_id: Some(listing_id),
            escrowed_amount: Some(price),
            original_listing_state: None,
        };
        env.storage().instance().set(&AtomicDataKey::AtomicState(transaction_id, step_id), &state);

        true
    }

    fn prepare_validate_lease_listing(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 2 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let agent_id: u64 = args.get(1).unwrap().try_into().unwrap_or(0);

        if listing_id == 0 || agent_id == 0 {
            return false;
        }

        // Check if listing is already locked
        if env.storage().instance().has(&AtomicDataKey::LockedListing(listing_id)) {
            return false;
        }

        let listing_key = (Symbol::new(env, "listing"), listing_id);
        let listing: Option<Listing> = env.storage().instance().get(&listing_key);

        match listing {
            Some(listing) => {
                if !listing.active || listing.agent_id != agent_id || listing.listing_type != ListingType::Lease {
                    return false;
                }

                // Lock the listing
                env.storage().instance().set(&AtomicDataKey::LockedListing(listing_id), &transaction_id);

                // Store atomic state
                let state = AtomicState {
                    transaction_id,
                    step_id,
                    locked_listing_id: Some(listing_id),
                    escrowed_amount: None,
                    original_listing_state: Some(listing),
                };
                env.storage().instance().set(&AtomicDataKey::AtomicState(transaction_id, step_id), &state);

                true
            }
            None => false,
        }
    }

    fn prepare_create_lease_record(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 6 {
            return false;
        }

        // Validate all arguments are present and valid
        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let lessee: Address = args.get(1).unwrap().try_into().unwrap();
        let lessor: Address = args.get(2).unwrap().try_into().unwrap();
        let lease_price: i128 = args.get(3).unwrap().try_into().unwrap_or(0);
        let duration_seconds: u64 = args.get(4).unwrap().try_into().unwrap_or(0);
        let deposit_amount: i128 = args.get(5).unwrap().try_into().unwrap_or(0);

        if listing_id == 0 || lease_price <= 0 || duration_seconds == 0 || deposit_amount < 0 {
            return false;
        }

        // Store atomic state
        let state = AtomicState {
            transaction_id,
            step_id,
            locked_listing_id: Some(listing_id),
            escrowed_amount: Some(lease_price + deposit_amount),
            original_listing_state: None,
        };
        env.storage().instance().set(&AtomicDataKey::AtomicState(transaction_id, step_id), &state);

        true
    }

    // ============ COMMIT PHASE IMPLEMENTATIONS ============

    fn commit_validate_listing(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        _args: &Vec<Val>,
    ) -> Val {
        // Validation was done in prepare phase, just return success
        Val::from_bool(true)
    }

    fn commit_transfer_to_escrow(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> Val {
        let buyer: Address = args.get(0).unwrap().try_into().unwrap();
        let amount: i128 = args.get(2).unwrap().try_into().unwrap_or(0);

        // Store escrow balance
        env.storage().instance().set(&AtomicDataKey::EscrowBalance(buyer, transaction_id), &amount);

        Val::from_bool(true)
    }

    fn commit_complete_sale(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> Val {
        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let buyer: Address = args.get(1).unwrap().try_into().unwrap();
        let price: i128 = args.get(2).unwrap().try_into().unwrap_or(0);

        // Mark listing as inactive
        let listing_key = (Symbol::new(env, "listing"), listing_id);
        let mut listing: Listing = env.storage().instance().get(&listing_key).unwrap();
        listing.active = false;
        env.storage().instance().set(&listing_key, &listing);

        // Release escrow
        env.storage().instance().remove(&AtomicDataKey::EscrowBalance(buyer, transaction_id));

        // Unlock listing
        env.storage().instance().remove(&AtomicDataKey::LockedListing(listing_id));

        // Emit event
        env.events().publish(
            (Symbol::new(env, "atomic_sale_completed"),),
            (transaction_id, listing_id, listing.agent_id, buyer, price),
        );

        Val::from_bool(true)
    }

    fn commit_validate_lease_listing(
        env: &Env,
        _transaction_id: u64,
        _step_id: u32,
        _args: &Vec<Val>,
    ) -> Val {
        // Validation was done in prepare phase, just return success
        Val::from_bool(true)
    }

    fn commit_create_lease_record(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> Val {
        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        let lessee: Address = args.get(1).unwrap().try_into().unwrap();
        let lessor: Address = args.get(2).unwrap().try_into().unwrap();
        let lease_price: i128 = args.get(3).unwrap().try_into().unwrap_or(0);
        let duration_seconds: u64 = args.get(4).unwrap().try_into().unwrap_or(0);
        let deposit_amount: i128 = args.get(5).unwrap().try_into().unwrap_or(0);

        // In a real implementation, you would create the lease record here
        // For now, we'll just emit an event

        // Unlock listing
        env.storage().instance().remove(&AtomicDataKey::LockedListing(listing_id));

        // Emit event
        env.events().publish(
            (Symbol::new(env, "atomic_lease_created"),),
            (transaction_id, listing_id, lessee, lessor, lease_price, duration_seconds, deposit_amount),
        );

        Val::from_bool(true)
    }

    // ============ ROLLBACK PHASE IMPLEMENTATIONS ============

    fn rollback_unlock_listing(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 1 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        if listing_id == 0 {
            return false;
        }

        // Unlock the listing
        env.storage().instance().remove(&AtomicDataKey::LockedListing(listing_id));

        // Clean up atomic state
        env.storage().instance().remove(&AtomicDataKey::AtomicState(transaction_id, step_id));

        true
    }

    fn rollback_refund_from_escrow(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 2 {
            return false;
        }

        let buyer: Address = args.get(0).unwrap().try_into().unwrap();
        let amount: i128 = args.get(1).unwrap().try_into().unwrap_or(0);

        if amount <= 0 {
            return false;
        }

        // Remove escrow balance (in real implementation, would refund tokens)
        env.storage().instance().remove(&AtomicDataKey::EscrowBalance(buyer, transaction_id));

        // Clean up atomic state
        env.storage().instance().remove(&AtomicDataKey::AtomicState(transaction_id, step_id));

        true
    }

    fn rollback_revert_sale(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 1 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        if listing_id == 0 {
            return false;
        }

        // Restore original listing state
        let state: Option<AtomicState> = env.storage().instance()
            .get(&AtomicDataKey::AtomicState(transaction_id, step_id));

        if let Some(state) = state {
            if let Some(original_listing) = state.original_listing_state {
                let listing_key = (Symbol::new(env, "listing"), listing_id);
                env.storage().instance().set(&listing_key, &original_listing);
            }
        }

        // Unlock listing
        env.storage().instance().remove(&AtomicDataKey::LockedListing(listing_id));

        // Clean up atomic state
        env.storage().instance().remove(&AtomicDataKey::AtomicState(transaction_id, step_id));

        true
    }

    fn rollback_unlock_lease_listing(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        Self::rollback_unlock_listing(env, transaction_id, step_id, args)
    }

    fn rollback_delete_lease_record(
        env: &Env,
        transaction_id: u64,
        step_id: u32,
        args: &Vec<Val>,
    ) -> bool {
        if args.len() < 1 {
            return false;
        }

        let listing_id: u64 = args.get(0).unwrap().try_into().unwrap_or(0);
        if listing_id == 0 {
            return false;
        }

        // In a real implementation, you would delete the lease record here
        // For now, just unlock the listing

        // Unlock listing
        env.storage().instance().remove(&AtomicDataKey::LockedListing(listing_id));

        // Clean up atomic state
        env.storage().instance().remove(&AtomicDataKey::AtomicState(transaction_id, step_id));

        true
    }
}