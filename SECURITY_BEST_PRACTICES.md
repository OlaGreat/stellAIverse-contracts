# Soroban Smart Contract Security Best Practices

**Project**: StellAIverse  
**Framework**: Soroban (Stellar)  
**Date**: January 21, 2026  
**Purpose**: Guidelines for secure contract development and audit preparation

---

## 1. Authentication & Authorization

### 1.1 Always Require Authentication

Every function that modifies state must authenticate the caller.

```rust
#[contractimpl]
impl MyContract {
    pub fn sensitive_operation(env: Env, caller: Address, data: String) {
        // ✅ GOOD: Require authentication
        caller.require_auth();
        
        // ... perform operation ...
    }
    
    // ❌ BAD: Missing authentication
    pub fn unsafe_operation(env: Env, data: String) {
        // Anyone can call this!
    }
}
```

### 1.2 Verify Ownership/Authorization

Never trust a caller just because they're authenticated. Verify they have permission.

```rust
#[contractimpl]
impl MyContract {
    pub fn update_agent(env: Env, agent_id: u64, owner: Address, new_data: String) {
        owner.require_auth();
        
        // ✅ GOOD: Verify ownership
        let agent = get_agent(env, agent_id);
        if agent.owner != owner {
            panic!("Unauthorized: only owner can update");
        }
        
        // ... perform update ...
    }
    
    // ❌ BAD: No ownership check
    pub fn update_agent_bad(env: Env, agent_id: u64, owner: Address, new_data: String) {
        owner.require_auth();
        // Assumes caller is owner - NOT VERIFIED
    }
}
```

### 1.3 Implement Role-Based Access Control

For contracts with multiple privilege levels, use clear role definitions.

```rust
const ADMIN_KEY: &str = "admin";

fn verify_admin(env: &Env, caller: &Address) {
    let admin: Address = env.storage()
        .instance()
        .get(&Symbol::new(env, ADMIN_KEY))
        .expect("Admin not set");
    
    if caller != &admin {
        panic!("Unauthorized: caller is not admin");
    }
}

#[contractimpl]
impl MyContract {
    pub fn init(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
    }
    
    pub fn admin_operation(env: Env, admin: Address, data: String) {
        admin.require_auth();
        verify_admin(&env, &admin);
        // ... perform admin operation ...
    }
}
```

### 1.4 Prevent Re-initialization

Ensure `init` functions can only be called once.

```rust
pub fn init_contract(env: Env, admin: Address) {
    // ✅ GOOD: Check if already initialized
    let existing_admin = env.storage().instance().get::<_, Address>(&Symbol::new(&env, ADMIN_KEY));
    if existing_admin.is_some() {
        panic!("Contract already initialized");
    }
    
    admin.require_auth();
    env.storage().instance().set(&Symbol::new(&env, ADMIN_KEY), &admin);
}
```

---

## 2. Safe Arithmetic Operations

### 2.1 Always Use Checked Arithmetic

Unchecked arithmetic can silently overflow/underflow. Use `checked_*` methods.

```rust
// ❌ BAD: Unchecked arithmetic
pub fn bad_increment(counter: u64) -> u64 {
    counter + 1  // Could overflow silently!
}

// ✅ GOOD: Checked arithmetic
pub fn good_increment(counter: u64) -> u64 {
    counter.checked_add(1).expect("Counter overflow")
}

// Apply to all arithmetic:
let safe_add = a.checked_add(b).expect("Overflow");
let safe_sub = a.checked_sub(b).expect("Underflow");
let safe_mul = a.checked_mul(b).expect("Overflow");
let safe_div = a.checked_div(b).expect("Division by zero");
```

### 2.2 Validate Numeric Ranges

Check that numeric inputs are within acceptable ranges.

```rust
pub const PRICE_MIN: i128 = 0;
pub const PRICE_MAX: i128 = i128::MAX / 2;
pub const PERCENTAGE_MAX: u32 = 10000;  // 100%

pub fn validate_price(price: i128) {
    if price < PRICE_MIN || price > PRICE_MAX {
        panic!("Price out of valid range");
    }
}

pub fn validate_percentage(percentage: u32) {
    if percentage > PERCENTAGE_MAX {
        panic!("Percentage exceeds 100%");
    }
}
```

### 2.3 Be Careful with Type Conversions

When converting between types, check for valid ranges.

```rust
// ✅ GOOD: Explicit range checking on conversion
pub fn safe_cast_to_u32(value: u64) -> u32 {
    if value > u32::MAX as u64 {
        panic!("Value too large for u32");
    }
    value as u32
}

// ✅ GOOD: Use try_from
pub fn safe_cast_2(value: u64) -> Result<u32, _> {
    u32::try_from(value)
}
```

---

## 3. Input Validation

### 3.1 Define and Enforce Size Limits

All user inputs should have maximum sizes defined.

```rust
// In shared module
pub const MAX_STRING_LENGTH: usize = 256;
pub const MAX_ARRAY_SIZE: usize = 1000;
pub const MAX_CAPABILITIES: usize = 32;

// In contract
pub fn create_agent(env: Env, name: String, capabilities: Vec<String>) {
    // ✅ GOOD: Validate all inputs
    if name.len() > shared::MAX_STRING_LENGTH {
        panic!("Name too long");
    }
    
    if capabilities.len() > shared::MAX_CAPABILITIES {
        panic!("Too many capabilities");
    }
    
    for cap in &capabilities {
        if cap.len() > shared::MAX_STRING_LENGTH {
            panic!("Capability name too long");
        }
    }
    
    // ... rest of function ...
}
```

### 3.2 Validate Enum Values

When accepting enum-like u32 values, validate they're in acceptable range.

```rust
// ✅ GOOD: Validate before using
pub fn create_listing(env: Env, listing_type: u32) {
    if listing_type > 2 {  // Only types 0, 1, 2 valid
        panic!("Invalid listing type");
    }
    
    let listing_type_enum = match listing_type {
        0 => ListingType::Sale,
        1 => ListingType::Lease,
        2 => ListingType::Auction,
        _ => panic!("Invalid type"),
    };
}
```

### 3.3 Check ID Parameters

ID parameters should not be zero (usually indicates uninitialized).

```rust
pub fn get_agent(env: Env, agent_id: u64) {
    // ✅ GOOD: Validate ID
    if agent_id == 0 {
        panic!("Invalid agent ID: must be greater than 0");
    }
    
    // ... fetch and return agent ...
}
```

---

## 4. Replay Attack Prevention

### 4.1 Implement Nonce Tracking

For sensitive operations, track a nonce to prevent replay.

```rust
#[derive(Clone)]
#[contracttype]
pub struct Agent {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub nonce: u64,  // ✅ Include nonce
    // ... other fields ...
}

pub fn execute_action(env: Env, agent_id: u64, nonce: u64, action: String) {
    // ✅ GOOD: Check nonce increases
    let stored_nonce = get_action_nonce(&env, agent_id);
    
    if nonce <= stored_nonce {
        panic!("Replay attack detected: nonce too low");
    }
    
    // ... perform action ...
    
    // ✅ GOOD: Store new nonce
    store_action_nonce(&env, agent_id, nonce);
}
```

### 4.2 Increment Nonce on State Changes

Whenever state changes, increment the nonce.

```rust
pub fn update_agent(env: Env, agent_id: u64, owner: Address, new_name: String) {
    owner.require_auth();
    
    let mut agent = get_agent(&env, agent_id);
    if agent.owner != owner {
        panic!("Not authorized");
    }
    
    agent.name = new_name;
    agent.updated_at = env.ledger().timestamp();
    
    // ✅ GOOD: Increment nonce
    agent.nonce = agent.nonce.checked_add(1).expect("Nonce overflow");
    
    store_agent(&env, agent);
}
```

---

## 5. Rate Limiting & DoS Prevention

### 5.1 Limit Query Results

Never return unbounded amounts of data.

```rust
pub fn get_history(env: Env, agent_id: u64, limit: u32) -> Vec<String> {
    // ✅ GOOD: Enforce maximum limit
    if limit > 500 {
        panic!("Limit exceeds maximum (500)");
    }
    
    if limit == 0 {
        panic!("Limit must be at least 1");
    }
    
    let history = fetch_history(&env, agent_id);
    
    // Return only requested amount
    let mut result = Vec::new(&env);
    for i in 0..std::cmp::min(limit as usize, history.len()) {
        result.push_back(history.get(i).unwrap());
    }
    result
}
```

### 5.2 Cap Data Structure Sizes

Prevent unbounded growth of storage collections.

```rust
pub fn add_to_history(env: Env, agent_id: u64, event: String) {
    let mut history: Vec<String> = get_history_vec(&env, agent_id);
    
    // ✅ GOOD: Cap size
    if history.len() >= 10000 {
        panic!("History limit reached");
    }
    
    history.push_back(event);
    store_history(&env, agent_id, history);
}
```

### 5.3 Implement Operation Rate Limiting

Limit how many operations an address can perform per time window.

```rust
const RATE_LIMIT_WINDOW: u64 = 60;  // seconds
const MAX_OPS_PER_WINDOW: u32 = 100;

pub fn check_rate_limit(env: &Env, agent_id: u64, key_prefix: &str) {
    let now = env.ledger().timestamp();
    let rate_key = format!("{}{}", key_prefix, agent_id);
    
    let (last_reset, count): (u64, u32) = env.storage()
        .instance()
        .get(&Symbol::new(env, &rate_key))
        .unwrap_or((now, 0));
    
    if now > last_reset + RATE_LIMIT_WINDOW {
        // Reset window
        env.storage().instance().set(&Symbol::new(env, &rate_key), &(now, 1));
    } else if count >= MAX_OPS_PER_WINDOW {
        panic!("Rate limit exceeded");
    } else {
        env.storage().instance().set(&Symbol::new(env, &rate_key), &(last_reset, count + 1));
    }
}
```

---

## 6. State Management

### 6.1 Use Atomic Operations

State updates should be atomic - all succeed or all fail.

```rust
// ✅ GOOD: Atomic update
pub fn buy_agent(env: Env, buyer: Address, agent_id: u64) {
    buyer.require_auth();
    
    let mut agent = get_agent(&env, agent_id);
    let listing = get_listing(&env, agent_id);
    
    // Make all changes in memory first
    agent.owner = buyer.clone();
    listing.active = false;
    
    // Then store atomically
    store_agent(&env, agent);
    store_listing(&env, listing);
    
    // Emit event (side effect happens last)
    emit_event(...);
}

// ❌ BAD: Non-atomic - could partially succeed
pub fn buy_agent_bad(env: Env, buyer: Address, agent_id: u64) {
    store_agent(&env, agent);  // What if this succeeds but next fails?
    store_listing(&env, listing);  // Then agent state is inconsistent
}
```

### 6.2 Prevent Double-Spending

Use flags or locks to prevent operations from being performed twice.

```rust
const STAKE_CLAIMED_KEY: &str = "stake_claimed_";

pub fn claim_stake(env: Env, request_id: u64, owner: Address) {
    owner.require_auth();
    
    let request = get_request(&env, request_id);
    if request.owner != owner {
        panic!("Not authorized");
    }
    
    // ✅ GOOD: Check if already claimed
    let claimed_key = format!("{}{}", STAKE_CLAIMED_KEY, request_id);
    if env.storage().instance().has(&Symbol::new(&env, &claimed_key)) {
        panic!("Stake already claimed");
    }
    
    // ✅ GOOD: Mark as claimed before transferring (atomic)
    env.storage().instance().set(&Symbol::new(&env, &claimed_key), &true);
    
    // Transfer tokens
    transfer_stake(&env, owner, request.stake_amount);
}
```

---

## 7. Event Emission

### 7.1 Emit Events for All Important Operations

Events provide an audit trail and help with monitoring.

```rust
pub fn mint_agent(env: Env, owner: Address, name: String) -> u64 {
    owner.require_auth();
    
    let agent_id = 1u64;  // Generate ID
    
    // ... store agent ...
    
    // ✅ GOOD: Emit event
    env.events().publish(
        (Symbol::new(&env, "agent_minted"),),  // Event name
        (agent_id, owner, name, env.ledger().timestamp()),  // Event data
    );
    
    agent_id
}
```

### 7.2 Include Relevant Data in Events

Events should include enough information for analysis.

```rust
// ❌ BAD: Missing context
env.events().publish((Symbol::new(&env, "action"),), ());

// ✅ GOOD: Include relevant data
env.events().publish(
    (Symbol::new(&env, "action_executed"),),
    (agent_id, action_type, executor, timestamp, result)
);
```

---

## 8. Storage Best Practices

### 8.1 Use Consistent Key Naming

Organize storage keys with clear prefixes.

```rust
const AGENT_KEY_PREFIX: &str = "agent_";
const LISTING_KEY_PREFIX: &str = "listing_";
const RATE_LIMIT_KEY_PREFIX: &str = "rate_";

fn get_agent_key(agent_id: u64) -> String {
    format!("{}{}", AGENT_KEY_PREFIX, agent_id)
}

// Usage
let key = get_agent_key(123);
let agent = env.storage().instance().get(&key);
```

### 8.2 Minimize Storage Access

Batch reads and writes to reduce gas.

```rust
// ❌ BAD: Multiple reads and writes
pub fn update_multiple(env: Env, agent_id: u64) {
    let mut agent = get_agent(&env, agent_id);
    agent.field1 = value1;
    store_agent(&env, agent);  // Write 1
    
    let mut agent = get_agent(&env, agent_id);  // Read again!
    agent.field2 = value2;
    store_agent(&env, agent);  // Write 2
}

// ✅ GOOD: Single read, modify, single write
pub fn update_multiple_good(env: Env, agent_id: u64) {
    let mut agent = get_agent(&env, agent_id);
    agent.field1 = value1;
    agent.field2 = value2;
    store_agent(&env, agent);  // Single write
}
```

---

## 9. Error Handling

### 9.1 Use Descriptive Error Messages

Panic messages should clearly explain what went wrong.

```rust
// ❌ BAD: Unclear error
if !authorized {
    panic!("ERROR");
}

// ✅ GOOD: Descriptive error
if !authorized {
    panic!("Unauthorized: only agent owner can update");
}

// ✅ GOOD: Even better - specific error codes
if !authorized {
    panic!("SEC_ERR_001: Caller is not the authorized owner");
}
```

### 9.2 Fail Fast

Validate inputs and check preconditions early, before any state changes.

```rust
// ✅ GOOD: Validate first, then execute
pub fn update_agent(env: Env, agent_id: u64, owner: Address, name: String) {
    // Validation phase (no state changes)
    owner.require_auth();
    
    if agent_id == 0 {
        panic!("Invalid agent ID");
    }
    
    if name.len() > MAX_STRING_LENGTH {
        panic!("Name too long");
    }
    
    let agent = get_agent(&env, agent_id);
    if agent.owner != owner {
        panic!("Not authorized");
    }
    
    // Execution phase (now safe to modify state)
    agent.name = name;
    store_agent(&env, agent);
}
```

---

## 10. Testing Checklist

### Security Tests to Implement

- [ ] **Authentication**: Unauthorized caller cannot call protected functions
- [ ] **Authorization**: Non-owner cannot modify resource
- [ ] **Replay Protection**: Cannot replay transaction with old nonce
- [ ] **Arithmetic**: Overflow panics (doesn't silently overflow)
- [ ] **Input Validation**: Oversized input rejected
- [ ] **Rate Limiting**: Exceeding limit is rejected
- [ ] **DoS Prevention**: Query limits enforced
- [ ] **Initialization**: Re-initialization blocked
- [ ] **Double-Spend**: Can't claim stake twice
- [ ] **Atomicity**: Partial failures don't leave inconsistent state

### Test Example

```rust
#[test]
fn test_unauthorized_update() {
    let env = TestEnv::new();
    let owner = Address::random(&env);
    let attacker = Address::random(&env);
    
    let agent_id = mint_agent(&env, owner, "test");
    
    // ✅ Should panic when attacker tries to update
    assert_error(
        update_agent(&env, agent_id, attacker, "hacked"),
        "Unauthorized"
    );
}

#[test]
fn test_replay_protection() {
    let env = TestEnv::new();
    let executor = Address::random(&env);
    let agent_id = 1u64;
    
    // First execution with nonce=1 succeeds
    execute_action(&env, agent_id, 1, "action");
    
    // Replaying with nonce=1 fails
    assert_error(
        execute_action(&env, agent_id, 1, "action"),
        "Replay protection"
    );
    
    // Nonce=2 succeeds
    execute_action(&env, agent_id, 2, "action");
}
```

---

## 11. Code Review Checklist

Before submitting for audit, verify:

- [ ] **All state modifications require `require_auth()`**
- [ ] **All resource modifications verify ownership**
- [ ] **All arithmetic uses `checked_*` operations**
- [ ] **All numeric inputs are validated**
- [ ] **All string/array inputs have size limits**
- [ ] **All sensitive operations have rate limiting**
- [ ] **Query operations return limited results**
- [ ] **Storage collections have size caps**
- [ ] **Init functions cannot be called twice**
- [ ] **Double-spend is prevented**
- [ ] **Events are emitted for audit trail**
- [ ] **Error messages are descriptive**

---

## 12. Pre-Audit Verification

Run through this before submitting to auditors:

```bash
# 1. Check for panics in expected paths
cargo test --all 2>&1 | grep "panicked at"

# 2. Run clippy for lint warnings
cargo clippy --all 2>&1 | grep "warning:"

# 3. Check code coverage
cargo tarpaulin --out Html

# 4. Verify no unsafe code used
cargo tree | grep -i "unsafe"

# 5. Check for hardcoded values
grep -r "const.*=.*[0-9]\{5,\}" contracts/

# 6. Review all panic! calls for proper messages
grep -r "panic!" contracts/ | wc -l
```

---

## References

- [Soroban Documentation](https://soroban.stellar.org/)
- [Rust Arithmetic Safety](https://doc.rust-lang.org/std/num/struct.Wrapping.html)
- [Smart Contract Security Best Practices](https://consensys.net/diligence/blog/)
- [OWASP Smart Contract Security](https://owasp.org/)

---

**Version**: 1.0  
**Last Updated**: January 21, 2026  
**Status**: Reference Documentation for Developers
