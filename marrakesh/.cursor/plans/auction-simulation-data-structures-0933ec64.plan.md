<!-- 0933ec64-2aca-46f0-9e99-612974a44bff 50fff80a-716a-4366-8ac6-5d193e6034bd -->
# Auction Simulation Data Structures

## Implementation Plan

Create the foundational data structures for the auction/marketplace simulation system.

### Files to Create/Modify

1. **src/main.rs** - Main entry point with global vectors and hardcoded campaigns
2. **src/types.rs** - Core data structures and enums

### Data Structures

#### Enums

- `ChargeType`: `FIXED_COST` | `FIRST_PRICE`
- `CampaignType`: `FIXED_IMPRESSIONS` | `FIXED_BUDGET`

#### Structs

**Impression**:

- `seller_id: i32`
- `charge_type: ChargeType`
- `fixed_cost_cpm: f64` (only relevant when charge_type is FIXED_COST)
- `best_other_bid_cpm: f64`
- `floor_cpm: f64`
- `win_bid_cpm: f64`
- `win_campaign_id: i32`
- `value_to_campaign_id: [f64; 10]` (array of 10 floats)

**Campaign**:

- `campaign_id: i32`
- `campaign_rnd: u64`
- `total_cost: f64`
- `pacing: f64` (default 1.0)
- `campaign_type: CampaignType`
- Conditional fields based on campaign_type:
- `FIXED_IMPRESSIONS`: `total_impressions_target: i32`
- `FIXED_BUDGET`: `total_budget_target: f64`

**Seller**:

- `seller_id: i32`
- `seller_name: String`
- `charge_type: ChargeType`
- `fixed_cost_cpm: f64` (only relevant when charge_type is FIXED_COST)

### Global State

- Global `Vec<Impression>` for impressions (dynamically filled)
- Global `Vec<Campaign>` for campaigns (initially with 2 hardcoded campaigns)

### Implementation Details

- Use Rust enums with variants for ChargeType and CampaignType
- Use enum variants with associated data for Campaign to handle conditional fields (FIXED_IMPRESSIONS vs FIXED_BUDGET)
- Initialize global vectors as static mutable or use lazy_static/once_cell if needed, or keep them in main for now
- Add two hardcoded Campaign instances in main.rs initialization

### To-dos

- [ ] Create src/types.rs with ChargeType and CampaignType enums
- [ ] Define Impression struct with all required fields
- [ ] Define Campaign struct with enum-based conditional fields
- [ ] Define Seller struct with required fields
- [ ] Set up global vectors in main.rs and add two hardcoded campaigns