# Governance Contract

A decentralized governance system (DAO) for the StellAIverse platform, enabling stakeholders to vote on proposals for platform decisions.

## Features

### ✅ Core Functionality

- **Proposal System**: Create proposals with different types (ParameterChange, ContractUpgrade, EmergencyPause)
- **Voting Mechanism**: Cast votes (For, Against, Abstain) with voting power calculations
- **Delegation System**: Delegate voting power to other addresses with re-delegation support
- **Vote Escrow**: Lock tokens for 4-52 weeks to earn 2x-4x voting power multipliers
- **Proposal Execution**: Execute passed proposals with proper threshold checks
- **Event Emission**: Comprehensive events for indexing and UI updates

### ✅ Voting Power Calculation

Voting power formula: `base_tokens + delegated_amount + (escrow_amount × escrow_multiplier)`

- **Base Power**: Token balance
- **Escrow Power**: Locked tokens with multiplier (2x for 4 weeks, 4x for 52 weeks)
- **Delegated Power**: Power delegated TO an address (uses reverse index for efficiency)

### ✅ Proposal Lifecycle

1. **Create**: Proposer deposits tokens and creates proposal
2. **Vote**: Stakeholders cast votes during voting period
3. **Evaluate**: System checks quorum (30%) and approval (66%) thresholds
4. **Execute**: Anyone can execute passed proposals
5. **Complete**: Deposit returned to proposer

### ✅ Thresholds

- **Quorum**: 30% of circulating voting power (configurable)
- **Approval**: 66% of votes cast must be "For" (configurable)
- **Voting Period**: 7-14 days (configurable min/max)

### ✅ Security Features

- Double-vote prevention
- Voting period enforcement
- Threshold validation
- Authorization checks
- Deposit mechanism for proposals

## Contract Structure

```
contracts/governance/
├── src/
│   ├── lib.rs          # Main contract implementation
│   ├── types.rs        # Type definitions (Proposal, VoteType, etc.)
│   ├── storage.rs      # Storage helpers and DataKey enum
│   └── test.rs         # Comprehensive test suite
├── Cargo.toml
└── README.md
```

## Key Functions

### Initialization
- `init_contract()` - Initialize governance contract with configuration

### Proposals
- `create_proposal()` - Create a new proposal (requires deposit)
- `update_proposal_status()` - Update proposal status after voting period
- `execute_proposal()` - Execute a passed proposal
- `get_proposal()` - Query proposal by ID
- `get_active_proposals()` - Get all active proposals

### Voting
- `cast_vote()` - Cast a vote on a proposal
- `get_vote()` - Get vote record for a voter

### Voting Power
- `get_vote_power()` - Get total voting power for an address
- `delegate_voting_power()` - Delegate voting power to another address
- `undelegate_voting_power()` - Remove delegation

### Vote Escrow
- `lock_for_escrow()` - Lock tokens for voting power multiplier
- `unlock_escrow()` - Unlock tokens after lock period ends
- `get_vote_escrow()` - Get escrow information

### Configuration
- `update_circulating_voting_power()` - Update circulating voting power (admin)

## Events

- `ProposalCreated` - Emitted when a proposal is created
- `VoteCast` - Emitted when a vote is cast
- `ProposalPassed` - Emitted when a proposal passes thresholds
- `ProposalExecuted` - Emitted when a proposal is executed
- `VotingPowerDelegated` - Emitted when voting power is delegated
- `VotingPowerUndelegated` - Emitted when delegation is removed
- `VoteEscrowLocked` - Emitted when tokens are locked for escrow
- `VoteEscrowUnlocked` - Emitted when escrow is unlocked

## Testing

Run tests with:
```bash
cargo test --package governance
```

The test suite includes:
- Contract initialization
- Proposal creation
- Voting mechanism
- Delegation system
- Vote escrow
- Proposal status updates
- Double-vote prevention
- Active proposals query

## Implementation Notes

### Delegation System
- Uses reverse index (`DelegatorsTo`) for efficient lookup of delegated power
- Supports re-delegation (delegatees can further delegate)
- Delegated power is calculated from delegator's base + escrow power
- When you delegate, your own voting power decreases by the delegated amount
- Delegation amount is capped at delegator's available power (base + escrow - already delegated)

### Vote Escrow
- Lock durations: 4 weeks (2x multiplier) to 52 weeks (4x multiplier)
- Linear multiplier calculation: `20000 + ((weeks - 4) * 20000) / 48`
- Tokens are transferred to the governance contract and locked until unlock period
- Escrow can be extended by adding more tokens (uses longer lock period)
- Multiplier applies only while tokens are locked (expires after lock_end)

### Circulating Voting Power
- Cached for efficiency
- Should be updated when token supply changes
- Admin function available for manual updates

### Proposal Execution
- Execution uses `env.invoke_contract()` to call target contract functions
- **ParameterChange**: Calls target function with parameter name and value
- **ContractUpgrade**: Calls upgrade function with new contract address
- **EmergencyPause**: Calls pause/unpause function with boolean flag
- Deposit is returned to proposer upon successful execution

## Acceptance Criteria Status

✅ All acceptance criteria from the issue have been implemented:
- Proposal struct with all required fields
- Voting power calculation (base + delegation + escrow)
- Delegation system with re-delegation support
- Voting mechanism with vote types
- Vote escrow with multipliers
- Proposal types (ParameterChange, ContractUpgrade, EmergencyPause)
- Voting thresholds (quorum and approval)
- Proposal execution
- Event emission
- Query functions

## Implementation Details

### Contract Invocation
- Proposal execution uses `env.invoke_contract()` to call target contract functions
- ParameterChange proposals call target contract with parameter name and value
- ContractUpgrade proposals call upgrade function with new contract address
- EmergencyPause proposals call pause/unpause function with boolean flag

### Deposit Mechanism
- Proposers must deposit tokens when creating a proposal
- Deposit is held in the governance contract
- Deposit is returned to proposer upon successful execution
- Default deposit: 1000 tokens (configurable)

## Future Enhancements

- [ ] Add quadratic voting option
- [ ] Implement proposal cancellation mechanism
- [ ] Add proposal timelock for execution delay
- [ ] Implement snapshot mechanism for voting power
- [ ] Add proposal templates for common operations
