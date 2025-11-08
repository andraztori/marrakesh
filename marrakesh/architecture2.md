# Marrakesh Project Architecture

## Introduction

Marrakesh is a Rust-based auction simulation system that models a digital advertising marketplace. The system simulates how advertising campaigns bid on impressions offered by sellers, with support for different pricing models and automatic optimization of campaign pacing to meet targets.

## System Overview

The simulation consists of four main components:

1. **Data Structures** (`types.rs`) - Core entities: Impressions, Campaigns, Sellers
2. **Simulation Execution** (`simulationrun.rs`) - Running auctions and collecting statistics
3. **Convergence Logic** (`converge.rs`) - Adaptive pacing adjustment to meet campaign targets
4. **Initialization** (`main.rs`) - Setup and orchestration

## Core Entities

### Impression

An `Impression` represents a single advertising opportunity. Each impression contains:

- **seller_id**: Which seller is offering this impression (usize, 0-based index)
- **charge_type**: How the seller charges (FIXED_COST or FIRST_PRICE)
- **best_other_bid_cpm**: Competing bid from outside the simulation (for FIRST_PRICE auctions)
- **floor_cpm**: Minimum acceptable bid price
- **value_to_campaign_id**: Array of values (one per campaign) indicating how valuable this impression is to each campaign

**Key Method**: `run_auction()` - Executes the auction logic for this impression, collecting bids from all campaigns and determining the winner.

### Campaign

A `Campaign` represents an advertiser trying to acquire impressions. Each campaign has:

- **campaign_id**: Unique identifier (usize, 0-based index)
- **campaign_name**: Human-readable name
- **campaign_rnd**: Random seed for future extensions
- **campaign_type**: Either FIXED_IMPRESSIONS (target number of impressions) or FIXED_BUDGET (target spending amount)

**Key Method**: `get_bid()` - Calculates the bid for a given impression based on pacing and value.

### Seller

A `Seller` represents a supply source offering impressions. Each seller has:

- **seller_id**: Unique identifier (usize, 0-based index)
- **seller_name**: Human-readable name
- **charge_type**: FIXED_COST (fixed price per impression) or FIRST_PRICE (auction-based)
- **num_impressions**: How many impressions this seller will generate

### Winner Enum

The auction outcome is represented by the `Winner` enum:

- `Campaign { campaign_id, virtual_cost, buyer_charge }`: A campaign won
- `OTHER_DEMAND`: Lost to external competition (bid below best_other_bid_cpm)
- `BELOW_FLOOR`: Bid was below the floor price
- `NO_DEMAND`: No campaigns participated

### AuctionResult

Each auction produces an `AuctionResult` containing:
- **winner**: The `Winner` enum outcome
- **supply_cost**: What the seller receives (already converted from CPM to actual cost)

## Auction Mechanism

### Bid Calculation

For each impression, campaigns calculate their bid as:

```
bid = pacing × value_to_campaign_id[campaign_id]
```

Where:
- `pacing` is a multiplier that controls bid aggressiveness (starts at 1.0)
- `value_to_campaign_id[campaign_id]` is the impression's value to this specific campaign

### Winner Determination

The auction process follows this logic:

1. Collect bids from all campaigns
2. Find the highest bid
3. Check constraints:
   - If highest bid < best_other_bid_cpm → `OTHER_DEMAND`
   - If highest bid < floor_cpm → `BELOW_FLOOR`
   - If no bids → `NO_DEMAND`
   - Otherwise → `Campaign` wins

### Cost Calculation

Three types of costs are tracked:

1. **supply_cost**: What the seller receives
   - FIXED_COST: Always `fixed_cost_cpm` (even if no valid winner)
   - FIRST_PRICE: The winning bid (or 0 if no valid winner)

2. **virtual_cost**: Marketplace tracking cost (always equals winning bid)

3. **buyer_charge**: What the campaign pays (always equals winning bid)

All costs are converted from CPM (cost per thousand) to actual cost by dividing by 1000.

## Simulation Flow

### Initialization Phase

1. Create `Campaigns` container and add campaigns with their targets
2. Create `Sellers` container and add sellers with their charge types
3. Generate `Impressions` from sellers:
   - For each seller, create `num_impressions` impressions
   - For FIRST_PRICE sellers: Generate random `best_other_bid_cpm` and `floor_cpm` using Normal distribution
   - Generate random `value_to_campaign_id` array for each impression
4. Initialize `CampaignParams` with default pacing = 1.0 for all campaigns

### Convergence Phase

The system iteratively adjusts pacing until campaigns meet their targets:

```
For each iteration (max 100):
  1. Run auctions for all impressions → SimulationRun
  2. Calculate statistics → SimulationStat
  3. For each campaign:
     - Compare actual vs target
     - If below target: increase pacing
     - If above target: decrease pacing
     - If within 1% tolerance: keep constant
  4. Print campaign statistics
  5. If no pacing changes: convergence achieved → break
```

### Pacing Adjustment Algorithm

The pacing adjustment uses proportional feedback:

```
error_ratio = |target - actual| / target
adjustment_factor = min(error_ratio × 10%, 10%)

If actual < target:
  pacing ← pacing × (1 + adjustment_factor)

If actual > target:
  pacing ← pacing × (1 - adjustment_factor)
```

Key features:
- Proportional: Larger errors cause larger adjustments
- Capped at 10% per iteration to prevent overshooting
- 1% tolerance zone prevents oscillation
- No minimum adjustment size allows fine-tuning

### Final Results Phase

After convergence:
1. Run final simulation with converged pacing
2. Generate complete statistics
3. Print detailed reports for campaigns, sellers, and overall marketplace

## Statistics System

### Campaign Statistics

For each campaign:
- Impressions obtained
- Total supply cost
- Total virtual cost
- Total buyer charge
- Total value obtained

### Seller Statistics

For each seller:
- Impressions sold vs. offered
- Total supply cost (revenue)
- Total virtual cost
- Total buyer charge

### Overall Statistics

Marketplace-wide metrics:
- Counts: below floor, other demand, no bids
- Total costs: supply, virtual, buyer
- Total value obtained by all campaigns

## Data Organization

### Index-Based Design

The system uses a consistent pattern where IDs are used as vector indices:

- `campaigns.campaigns[campaign_id]` - Direct access
- `sellers.sellers[seller_id]` - Direct access
- `campaign_params.params[campaign_id]` - Matched by index
- `simulation_run.results[impression_index]` - Matched to impressions by index

**Benefits**:
- O(1) lookup performance
- Type safety (usize prevents negative indices)
- Automatic ID assignment via `vec.len()`
- No hash maps needed

**Invariant**: IDs are assigned sequentially starting at 0 and never change.

### Container Structs

The system uses wrapper structs to encapsulate collections:

- `Campaigns`: Wraps `Vec<Campaign>` with `add()` method
- `Sellers`: Wraps `Vec<Seller>` with `add()` method
- `Impressions`: Wraps `Vec<Impression>` with `new()` method
- `CampaignParams`: Wraps `Vec<CampaignParam>` with `new()` method
- `SimulationRun`: Wraps `Vec<AuctionResult>` with `new()` method

## Random Number Generation

The system uses deterministic random number generation for reproducibility:

- Seed: 999 (hardcoded)
- Distributions:
  - `best_other_bid_cpm`: Normal(mean=10, stddev=3)
  - `floor_cpm`: Normal(mean=10, stddev=3)
  - `value_to_campaign_id`: Normal(mean=5, stddev=3)

This ensures the same input produces the same results across runs.

## Module Responsibilities

### `main.rs`

- Entry point and initialization
- Creates campaigns and sellers
- Generates impressions via `Impressions::new()`
- Orchestrates convergence via `SimulationConverge::run()`
- Outputs final results

### `types.rs`

- Core data structure definitions
- Auction logic (`Impression::run_auction()`)
- Bid calculation (`Campaign::get_bid()`)
- Container implementations (`Campaigns`, `Sellers`)
- Validation (max campaigns check)

### `simulationrun.rs`

- `SimulationRun`: Executes all auctions
- `CampaignParams`: Manages pacing parameters
- `SimulationStat`: Calculates and formats statistics
- Statistics generation and output methods

### `converge.rs`

- `SimulationConverge`: Convergence loop logic
- Adaptive pacing adjustment algorithm
- Iteration management and early termination

## Key Design Decisions

### Why Separate Campaign and CampaignParam?

Campaign data (targets, name, type) is static, while pacing is dynamic and adjusted during convergence. Separating them allows:
- Clear separation of concerns
- Efficient updates (only pacing changes)
- Better encapsulation

### Why Convert Costs in run_auction()?

Costs are converted from CPM to actual cost (divide by 1000) at the auction level, not in statistics. This ensures:
- Consistent cost representation throughout the system
- Statistics work with actual costs, not CPM
- Clear separation between auction mechanics and reporting

### Why Three Cost Types?

Tracking supply_cost, virtual_cost, and buyer_charge separately allows:
- Modeling different marketplace fee structures
- Analyzing value capture by different parties
- Future extensions (e.g., marketplace fees, discounts)

Currently, virtual_cost and buyer_charge are identical, but the separation enables future enhancements.

## Constraints and Limits

- **Maximum Campaigns**: 10 (defined by `MAX_CAMPAIGNS` constant)
- **Maximum Iterations**: 100 (convergence loop)
- **Tolerance**: 1% (pacing adjustment threshold)
- **Max Adjustment**: 10% per iteration (prevents overshooting)

## Example Execution

```
1. Initialize:
   - Campaign 0: Target 1000 impressions
   - Campaign 1: Target $20 budget
   - Seller MRG: 1000 impressions @ $10 CPM fixed
   - Seller HB: 10000 impressions @ first-price auction

2. Generate 11,000 impressions total

3. Convergence (example):
   Iteration 1: pacing=1.0
     Campaign 0: 512 impressions → increase pacing
     Campaign 1: $8.32 spent → increase pacing
   
   Iteration 15: Converged
     Campaign 0: 991 impressions ✓
     Campaign 1: $19.81 spent ✓

4. Final Statistics:
   - Campaign performance
   - Seller revenue
   - Marketplace efficiency metrics
```

## Testing

Unit tests cover:
- Bid calculation with different pacing values
- Bid calculation for different campaign IDs
- Edge cases (zero pacing)

Run tests: `cargo test`

## Build and Run

```bash
cargo build          # Debug build
cargo build --release # Optimized build
cargo run            # Run simulation
cargo check          # Check without building
```

## Dependencies

- `rand = "0.8"`: Random number generation
- `rand_distr = "0.4"`: Normal distribution sampling

## Future Extensions

Potential enhancements:
- Advanced bidding strategies (second-price, VCG)
- Dynamic value models
- More sophisticated pacing algorithms
- Multi-dimensional constraints
- Real-time simulation with time dynamics
- Export statistics to CSV/JSON
- Parallel auction execution
- Marketplace fee structures

