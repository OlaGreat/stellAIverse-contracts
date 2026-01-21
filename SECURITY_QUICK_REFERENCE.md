# Security Hardening Quick Reference Card

**Project**: StellAIverse  
**Framework**: Soroban  
**Date**: January 21, 2026

---

## 🔐 Critical Security Patterns

### Authentication
```rust
// ALWAYS require authentication on state changes
pub fn sensitive_fn(env: Env, caller: Address, ...) {
    caller.require_auth();
    // ... proceed with operation ...
}
```

### Authorization
```rust
// ALWAYS verify ownership/permission
if agent.owner != caller {
    panic!("Unauthorized: only agent owner can perform this");
}
```

### Safe Arithmetic
```rust
// ALWAYS use checked operations
let safe_id = counter.checked_add(1).expect("Overflow");
let safe_mul = amount.checked_mul(percentage).expect("Overflow");
let safe_sub = price.checked_sub(fee).expect("Underflow");
```

### Input Validation
```rust
// ALWAYS validate before using
if input_string.len() > MAX_STRING_LENGTH {
    panic!("Input exceeds maximum length");
}
if numeric_value < MIN_VALUE || numeric_value > MAX_VALUE {
    panic!("Value out of valid range");
}
```

### Replay Protection
```rust
// For sensitive operations, validate nonce
let stored_nonce = get_nonce(&env, agent_id);
if provided_nonce <= stored_nonce {
    panic!("Replay protection: invalid nonce");
}
```

---

## 📊 Constants Reference

```rust
// String limits
pub const MAX_STRING_LENGTH: usize = 256;

// Array limits
pub const MAX_CAPABILITIES: usize = 32;
pub const MAX_PROVIDERS: usize = 100;

// Numeric limits
pub const PRICE_UPPER_BOUND: i128 = i128::MAX / 2;
pub const PRICE_LOWER_BOUND: i128 = 0;
pub const MAX_ROYALTY_PERCENTAGE: u32 = 10000;  // 100%

// Duration limits
pub const MAX_DURATION_DAYS: u64 = 36500;  // ~100 years
pub const MAX_AGE_SECONDS: u64 = 31_536_000;  // ~1 year

// Storage limits
pub const MAX_HISTORY_SIZE: usize = 1000;
pub const MAX_QUERY_LIMIT: u32 = 500;

// Rate limiting
pub const RATE_LIMIT_WINDOW: u64 = 60;  // seconds
pub const MAX_OPS_PER_WINDOW: u32 = 100;
```

---

## 🛡️ Security Checklist

Before committing code:

- [ ] All state-modifying functions call `require_auth()`
- [ ] All resource modifications verify `caller == owner`
- [ ] All arithmetic uses `checked_add/mul/sub`
- [ ] All user inputs validated against size limits
- [ ] All numeric inputs checked for valid ranges
- [ ] Rate limiting implemented on sensitive ops
- [ ] Events emitted for important state changes
- [ ] Error messages are descriptive
- [ ] No unchecked loops or unlimited data structures
- [ ] Init function checks for re-initialization

---

## 🎯 Common Patterns

### Pattern 1: Secure State Update
```rust
pub fn update_resource(env: Env, id: u64, owner: Address, new_value: String) {
    // Auth
    owner.require_auth();
    
    // Input validation
    if id == 0 {
        panic!("Invalid ID");
    }
    if new_value.len() > MAX_STRING_LENGTH {
        panic!("Value too long");
    }
    
    // Get and verify ownership
    let mut resource = get_resource(&env, id);
    if resource.owner != owner {
        panic!("Unauthorized: only owner can update");
    }
    
    // Update and store
    resource.value = new_value;
    resource.nonce = resource.nonce.checked_add(1).expect("Nonce overflow");
    store_resource(&env, resource);
    
    // Emit event
    env.events().publish((Symbol::new(&env, "updated"),), (id, owner));
}
```

### Pattern 2: Rate-Limited Operation
```rust
const RATE_LIMIT_WINDOW: u64 = 60;
const MAX_OPS: u32 = 100;

pub fn rate_limited_operation(env: Env, agent_id: u64) {
    check_rate_limit(&env, agent_id, MAX_OPS, RATE_LIMIT_WINDOW);
    // ... perform operation ...
}

fn check_rate_limit(env: &Env, id: u64, max_ops: u32, window: u64) {
    let key = format!("rate_{}", id);
    let now = env.ledger().timestamp();
    
    let (last_reset, count): (u64, u32) = env.storage()
        .instance()
        .get(&Symbol::new(env, &key))
        .unwrap_or((now, 0));
    
    if now > last_reset + window {
        env.storage().instance().set(&Symbol::new(env, &key), &(now, 1));
    } else if count >= max_ops {
        panic!("Rate limit exceeded");
    } else {
        env.storage().instance().set(&Symbol::new(env, &key), &(last_reset, count + 1));
    }
}
```

### Pattern 3: Double-Spend Prevention
```rust
pub fn claim_resource(env: Env, id: u64, owner: Address) {
    owner.require_auth();
    
    // Check if already claimed
    let lock_key = format!("claimed_{}", id);
    if env.storage().instance().has(&Symbol::new(&env, &lock_key)) {
        panic!("Already claimed");
    }
    
    // Mark as claimed BEFORE transferring
    env.storage().instance().set(&Symbol::new(&env, &lock_key), &true);
    
    // Transfer resource
    transfer_to(&env, owner, get_amount(id));
}
```

---

## 🚨 Common Mistakes to Avoid

| ❌ DON'T | ✅ DO |
|---------|------|
| `counter + 1` | `counter.checked_add(1).expect(...)` |
| No auth check | `caller.require_auth()` |
| Accept any input | Validate size/range |
| Unlimited loops | Cap iterations |
| Re-claimable | Use lock mechanism |
| Store then check | Check then store |
| No events | Emit for audit trail |
| Generic errors | Descriptive error messages |
| Re-initializable | Check if already init |
| No rate limits | Implement on sensitive ops |

---

## 📋 Testing Checklist

Verify these test scenarios:

- [ ] **Auth Test**: Unauthorized caller rejected
- [ ] **Owner Test**: Non-owner cannot modify resource
- [ ] **Replay Test**: Same nonce rejected twice
- [ ] **Overflow Test**: Max value + 1 panics
- [ ] **Size Test**: Oversized input rejected
- [ ] **Rate Test**: Exceeding limit blocked
- [ ] **Double-Spend**: Can't claim twice
- [ ] **Re-Init**: Second init panics
- [ ] **Invalid ID**: ID=0 rejected
- [ ] **Range Test**: Out-of-range values rejected

---

## 🔍 Code Review Questions

Ask before approving PRs:

1. Does this modify state? If yes, does it require auth?
2. Does this access a resource? If yes, does it verify ownership?
3. Does this do arithmetic? If yes, are checked operations used?
4. Does this accept user input? If yes, is it validated?
5. Is this idempotent or does it need a nonce?
6. Could this be called excessively? If yes, add rate limiting?
7. Could this grow unbounded? If yes, add size limits?
8. Does this need an event for the audit trail?
9. Are error messages descriptive?
10. Could this double-spend or be exploited?

---

## 📚 Documentation Map

| Document | Purpose |
|----------|---------|
| `AUDIT_CHECKLIST.md` | Comprehensive audit readiness guide |
| `SECURITY_ISSUES_AND_FIXES.md` | Detailed security vulnerabilities and fixes |
| `SECURITY_BEST_PRACTICES.md` | Development guidelines and patterns |
| `HARDENING_SUMMARY.md` | Executive summary of all work done |
| This file | Quick reference for developers |

---

## 🎓 Key Concepts

### Nonce
A monotonically increasing counter that prevents replay attacks. Each agent has a nonce that increments on every state change.

### Rate Limiting
Restricting the number of operations per time window. Prevents spam and DoS attacks.

### Bounds Checking
Validating that numeric and string values are within acceptable ranges before using them.

### Atomic Operations
Either all state changes succeed, or none do. No partial updates.

### Double-Spend Prevention
Using flags/locks to ensure an action can only be performed once.

### Fail-Safe
Panicking on error rather than silently continuing with bad data.

---

## 🔗 Storage Key Patterns

```rust
// Use consistent prefixes for organization
const AGENT_KEY_PREFIX: &str = "agent_";
const LISTING_KEY_PREFIX: &str = "listing_";
const RATE_LIMIT_KEY_PREFIX: &str = "rate_";
const NONCE_KEY_PREFIX: &str = "nonce_";
const CLAIM_LOCK_PREFIX: &str = "claim_";

// Construct keys safely
let agent_key = format!("{}{}", AGENT_KEY_PREFIX, agent_id);
let nonce_key = format!("{}{}", NONCE_KEY_PREFIX, agent_id);
let claim_key = format!("{}{}", CLAIM_LOCK_PREFIX, request_id);
```

---

## 📞 When in Doubt

1. **Authentication**: Always call `require_auth()` for state changes
2. **Validation**: Always validate user inputs
3. **Arithmetic**: Always use `checked_*` operations
4. **Safety**: When uncertain, panic with descriptive error
5. **Logging**: When important, emit an event
6. **Testing**: When unsure, write a test

---

## 🚀 Pre-Deployment Verification

```bash
# 1. Does code compile?
cargo build --all

# 2. Do tests pass?
cargo test --all

# 3. Any clippy warnings?
cargo clippy --all

# 4. All auth checks in place?
grep -r "require_auth" contracts/ | wc -l

# 5. Safe arithmetic used?
grep -r "checked_add\|checked_mul" contracts/ | wc -l

# 6. Input validation present?
grep -r "panic!" contracts/ | grep -i "length\|range\|invalid" | wc -l
```

---

**Version**: 1.0  
**Last Updated**: January 21, 2026  
**Status**: Ready for Development
