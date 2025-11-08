# Marrakesh: Auction/Marketplace Simulation - Architecture Documentation

## Overview

**Marrakesh** is an auction simulation system written in Rust that models a marketplace where advertising campaigns bid for impressions from sellers. The system simulates different bidding strategies, charge types, and uses adaptive pacing to help campaigns meet their targets (either a fixed number of impressions or a fixed budget).

### Key Objectives
- Simulate auctions between campaigns and impressions
- Support multiple charge types (fixed cost vs. first-price auction)
- Track campaign performance (cost, value, impressions obtained)
- Automatically adjust campaign pacing to meet targets through iterative convergence
- Provide detailed statistics on campaigns, sellers, and overall marketplace performance

---

## Design Philosophy: Ideal Pacing Assumption

**Important**: This simulation operates under the assumption of **ideal/optimal pacing**. The convergence loop exists to automatically find the optimal pacing factors that allow campaigns to exactly meet their targets (impressions or budget).

### Why This Matters

The goal of this simulation is **not** to experiment with different pacing strategies or algorithms. Instead, the purpose is to:

1. **Observe bidding strategy behavior** - Study how different bidding approaches perform when pacing is already optimized
2. **Test pricing strategies** - Experiment with different pricing models and charge types
3. **Evaluate marketplace management** - Analyze how marketplace rules and mechanisms affect outcomes

By converging to ideal pacing first, we eliminate pacing as a variable and can focus on observing the behavior of:
- Different bidding strategies
- Various charge types (FIXED_COST vs FIRST_PRICE)
- Marketplace dynamics and efficiency
- Supply and demand interactions
- Value capture and distribution

The convergence loop is essentially a **simulation setup mechanism** that ensures we're testing campaigns under optimal conditions, allowing us to isolate and study other marketplace dynamics without the confounding effects of sub-optimal pacing.

---

## Project Structure

```
marrakesh/
├── Cargo.toml                 # Project dependencies (rand, rand_distr)
├── src/
│   ├── main.rs               # Entry point, initialization, Impressions container
│   ├── types.rs              # Core data structures and auction logic
│   ├── simulationrun.rs      # Simulation execution and statistics
│   └── converge.rs           # Adaptive pacing convergence loop
├── .cargo/
│   └── config.toml           # Rust build configuration
└── .vscode/
    ├── launch.json           # Debugging configuration
    └── settings.json         # IDE settings
```

---

## Core Concepts

### 1. Sellers
**Sellers** offer impressions for sale with specific charge models:
- **FIXED_COST**: Seller charges a fixed CPM (cost per thousand impressions) regardless of bid
- **FIRST_PRICE**: Seller charges the winning bid amount (standard auction model)

Each seller specifies:
- `seller_id`: Unique identifier (0-based index)
- `seller_name`: Human-readable name
- `charge_type`: FIXED_COST or FIRST_PRICE
- `num_impressions`: Number of impressions to generate

### 2. Impressions
**Impressions** are individual advertising opportunities generated from sellers. Each impression has:
- `seller_id`: Which seller is offering this impression
- `charge_type`: Inherited from seller
- `best_other_bid_cpm`: Competing demand outside the simulation (for FIRST_PRICE auctions)
- `floor_cpm`: Minimum acceptable bid (for FIRST_PRICE auctions)
- `value_to_campaign_id`: Array of values (one per campaign) representing how valuable this impression is to each campaign

**Value Generation**: All impression values are generated using deterministic random distributions (Normal distribution with mean=5, stddev=3) to ensure reproducible results across runs.

### 3. Campaigns
**Campaigns** are advertisers that bid on impressions to meet specific goals:
- **FIXED_IMPRESSIONS**: Campaign wants to obtain a specific number of impressions
- **FIXED_BUDGET**: Campaign wants to spend a specific total budget

Each campaign has:
- `campaign_id`: Unique identifier (0-based index)
- `campaign_name`: Human-readable name
- `campaign_rnd`: Random seed for future extensions
- `campaign_type`: FIXED_IMPRESSIONS or FIXED_BUDGET with target value

**Maximum Campaigns**: The system supports up to 10 campaigns (defined by `MAX_CAMPAIGNS` constant).

### 4. Campaign Parameters (Pacing)
**Pacing** controls how aggressively a campaign bids:
- `pacing = 1.0`: Campaign bids its true value for impressions
- `pacing > 1.0`: Campaign bids more aggressively (overbids)
- `pacing < 1.0`: Campaign bids more conservatively (underbids)

The pacing value is dynamically adjusted through the convergence loop to help campaigns meet their targets.

---

## Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. INITIALIZATION (main.rs)                                 │
├─────────────────────────────────────────────────────────────┤
│ • Create Campaigns with targets                             │
│ • Create Sellers with charge types                          │
│ • Generate Impressions from Sellers (with random values)    │
│ • Initialize CampaignParams (pacing = 1.0 for all)         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. CONVERGENCE LOOP (converge.rs)                          │
├─────────────────────────────────────────────────────────────┤
│ For each iteration (max 100):                               │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ a) Run all auctions → SimulationRun                 │  │
│   └─────────────────────────────────────────────────────┘  │
│                       ▼                                      │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ b) Generate statistics → SimulationStat             │  │
│   └─────────────────────────────────────────────────────┘  │
│                       ▼                                      │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ c) Adjust pacing for each campaign:                 │  │
│   │    • If actual < target: increase pacing            │  │
│   │    • If actual > target: decrease pacing            │  │
│   │    • If within 1% tolerance: keep constant          │  │
│   └─────────────────────────────────────────────────────┘  │
│                       ▼                                      │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ d) Print campaign statistics                        │  │
│   └─────────────────────────────────────────────────────┘  │
│                       ▼                                      │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ e) Check convergence:                               │  │
│   │    • If no pacing changes: BREAK                    │  │
│   │    • Otherwise: continue to next iteration          │  │
│   └─────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. FINAL RESULTS (main.rs)                                  │
├─────────────────────────────────────────────────────────────┤
│ • Run final simulation with converged pacing                │
│ • Generate complete statistics                              │
│ • Print detailed campaign, seller, and overall stats        │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Algorithms

### Auction Logic (`Impression::run_auction` in types.rs)

For each impression, the auction proceeds as follows:

```
1. COLLECT BIDS
   For each campaign:
     bid = campaign_pacing × impression.value_to_campaign_id[campaign_id]
   
   Select highest bid as winning_bid_cpm

2. DETERMINE WINNER
   IF no bids submitted:
     winner = NO_DEMAND
   
   ELSE IF winning_bid < best_other_bid_cpm:
     winner = OTHER_DEMAND  (lost to external competition)
   
   ELSE IF winning_bid < floor_cpm:
     winner = BELOW_FLOOR  (bid too low)
   
   ELSE:
     winner = Campaign {campaign_id, virtual_cost, buyer_charge}

3. CALCULATE COSTS
   Three types of costs are calculated:
   
   a) supply_cost: What the seller receives
      - FIXED_COST: Always fixed_cost_cpm (regardless of winner)
      - FIRST_PRICE: winning_bid_cpm (or 0 if no valid winner)
   
   b) virtual_cost: What the marketplace tracks as cost
      - Always equal to winning_bid_cpm
   
   c) buyer_charge: What the campaign pays
      - Always equal to winning_bid_cpm
   
   All costs are converted from CPM to actual cost by dividing by 1000
```

### Adaptive Pacing Algorithm (`SimulationConverge::run` in converge.rs)

The system uses a proportional feedback control to adjust campaign pacing:

```
For each campaign in each iteration:

1. CALCULATE ERROR
   target = campaign target (impressions or budget)
   actual = campaign actual (impressions obtained or budget spent)
   error_ratio = |target - actual| / target

2. APPLY TOLERANCE
   tolerance = target × 1%
   
   IF actual within [target - tolerance, target + tolerance]:
     No adjustment needed (converged)
     pacing_changed = false

3. PROPORTIONAL ADJUSTMENT
   adjustment_factor = min(error_ratio × 10%, 10%)
   
   IF actual < target:
     pacing ← pacing × (1 + adjustment_factor)  # Bid more aggressively
   
   ELSE IF actual > target:
     pacing ← pacing × (1 - adjustment_factor)  # Bid more conservatively

4. CHECK CONVERGENCE
   IF no pacing changed for any campaign:
     Convergence achieved → BREAK loop
   
   Maximum iterations: 100
```

**Key Features:**
- **Proportional**: Larger errors cause larger adjustments
- **Capped**: Maximum adjustment of 10% per iteration prevents overshooting
- **No minimum**: Allows fine-grained adjustments even near target
- **1% tolerance**: Prevents oscillation when very close to target

---

## Cost Model

The system tracks three distinct cost types to model realistic marketplace dynamics:

### Supply Cost
What the **seller receives** for the impression.
- **FIXED_COST sellers**: Always receive their fixed_cost_cpm, even if no valid winner
- **FIRST_PRICE sellers**: Receive the winning bid (or 0 if no valid winner)

### Virtual Cost
What the **marketplace tracks** as the cost of the impression.
- Always equals the winning bid
- Used for internal accounting and optimization

### Buyer Charge
What the **campaign pays** for the impression.
- Always equals the winning bid
- Used to track campaign spending against budgets

**Note**: In the current implementation, virtual_cost and buyer_charge are identical, but keeping them separate allows for future extensions (e.g., marketplace fees, discounts).

---

## Statistics System (`simulationrun.rs`)

The statistics system provides comprehensive insights into simulation performance:

### Campaign Statistics (`CampaignStat`)
For each campaign:
- `impressions_obtained`: Number of impressions won
- `total_supply_cost`: Sum of supply costs for won impressions
- `total_virtual_cost`: Sum of virtual costs for won impressions
- `total_buyer_charge`: Sum of buyer charges for won impressions
- `total_value`: Sum of impression values for won impressions

### Seller Statistics (`SellerStat`)
For each seller:
- `impressions_sold`: Number of impressions sold (vs. offered)
- `total_supply_cost`: Total revenue received by seller
- `total_virtual_cost`: Total virtual cost of sold impressions
- `total_buyer_charge`: Total charged to buyers

### Overall Statistics (`OverallStat`)
Marketplace-wide metrics:
- `below_floor_count`: Impressions where bids were below floor
- `other_demand_count`: Impressions lost to external competition
- `no_bids_count`: Impressions with no bids
- `total_supply_cost`: Total paid to all sellers
- `total_virtual_cost`: Total virtual cost across all impressions
- `total_buyer_charge`: Total charged to all campaigns
- `total_value`: Total value obtained by all winning campaigns

---

## Module Descriptions

### `main.rs`
**Purpose**: Entry point and initialization

**Key Components**:
- `Impressions` struct: Container for all impressions
- `Impressions::new()`: Generates impressions from sellers with random values
- `main()`: Initializes campaigns, sellers, impressions, runs convergence, outputs results

**Random Generation**: Uses deterministic seed (999) for reproducibility with normal distributions:
- `best_other_bid_cpm`: Normal(mean=10, stddev=3)
- `floor_cpm`: Normal(mean=10, stddev=3)
- `value_to_campaign_id`: Normal(mean=5, stddev=3)

### `types.rs`
**Purpose**: Core data structures and auction logic

**Key Structures**:
- `Impression`: Advertising opportunity with values and constraints
- `Campaign`: Advertiser with target goals
- `Seller`: Supply source with charge model
- `Winner`: Enum representing auction outcome
- `AuctionResult`: Complete auction result with winner and costs

**Key Methods**:
- `Impression::run_auction()`: Executes auction logic for single impression
- `Campaign::get_bid()`: Calculates bid based on pacing and value
- `Campaigns::add()`: Safely adds campaigns with validation (max 10)
- `Sellers::add()`: Adds sellers with auto-generated IDs

### `simulationrun.rs`
**Purpose**: Simulation execution and statistics generation

**Key Structures**:
- `SimulationRun`: Container for all auction results
- `CampaignParams`: Container for campaign pacing parameters
- `SimulationStat`: Complete statistics for campaigns, sellers, and overall

**Key Methods**:
- `SimulationRun::new()`: Runs auctions for all impressions
- `CampaignParams::new()`: Initializes pacing to 1.0 for all campaigns
- `SimulationStat::new()`: Calculates all statistics from auction results
- `SimulationStat::printout()`: Outputs formatted statistics
- `SimulationStat::printout_campaigns()`: Compact campaign-only output for iterations

### `converge.rs`
**Purpose**: Adaptive pacing convergence

**Key Structure**:
- `SimulationConverge`: Unit struct providing convergence logic

**Key Method**:
- `SimulationConverge::run()`: Executes convergence loop with adaptive pacing adjustments

**Algorithm**: Proportional feedback control with 1% tolerance and max 10% adjustment per iteration

---

## Index-Based Design Pattern

The system uses a **consistent index-based pattern** throughout:

```rust
// All IDs are usize and match vector indices
campaigns.campaigns[campaign_id]        // Direct access by ID
sellers.sellers[seller_id]              // Direct access by ID
campaign_params.params[campaign_id]     // Matched by index
simulation_run.results[impression_idx]  // Matched by index to impressions
```

**Benefits**:
- O(1) lookup performance
- Type safety (usize prevents negative indices)
- Automatic ID assignment via `vec.len()`
- No hash maps or separate ID management needed

**Invariant**: IDs are assigned sequentially starting at 0 and never change, ensuring index stability.

---

## Configuration

### Cargo Dependencies
- `rand = "0.8"`: Random number generation
- `rand_distr = "0.4"`: Normal distribution sampling

### Build Profiles (`.cargo/config.toml`)
- **dev**: Fast compilation, debug info, no optimization
- **release**: Full optimization, no debug info, minimal code units
- **test**: Debug info, overflow checks, fast compilation
- **bench**: Full optimization with LTO for benchmarking

### Debug Configuration (`.vscode/launch.json`)
- Uses CodeLLDB for Rust debugging
- Separate configurations for executable and tests

---

## Example Scenario

**Initial Setup**:
- Campaign 0: Wants 1000 impressions (FIXED_IMPRESSIONS)
- Campaign 1: Wants to spend $20 (FIXED_BUDGET)
- Seller "MRG": 1000 impressions at fixed $10 CPM
- Seller "HB": 10,000 impressions via first-price auction

**Convergence Process**:
```
Iteration 1: pacing = 1.0
  Campaign 0: 512 impressions (need 1000) → increase pacing
  Campaign 1: $8.32 spent (need $20) → increase pacing

Iteration 5: pacing adjusted upward
  Campaign 0: 876 impressions → keep increasing
  Campaign 1: $15.42 spent → keep increasing

Iteration 15: Converged
  Campaign 0: 991 impressions (within 1% of 1000) ✓
  Campaign 1: $19.81 spent (within 1% of $20) ✓
  
  No more pacing changes → Convergence achieved
```

**Final Results**:
- Total impressions sold: 2,806 / 11,000 offered
- Campaign 0 achieved 99.1% of impression target
- Campaign 1 achieved 99% of budget target
- Marketplace efficiently matched supply and demand

---

## Future Extensions

Potential areas for enhancement:

1. **Advanced Bidding Strategies**: Implement different bidding algorithms (second-price, VCG)
2. **Dynamic Value Models**: Allow impression values to change over time
3. **Budget Pacing**: More sophisticated pacing algorithms (PID control, ML-based)
4. **Multi-dimensional Constraints**: Support both budget AND impression constraints
5. **Real-time Simulation**: Add time-based dynamics and seasonality
6. **Reporting**: Export statistics to CSV/JSON for analysis
7. **Optimization**: Parallel auction execution for large-scale simulations
8. **Marketplace Fees**: Add platform fees between virtual_cost and buyer_charge

---

## Testing

The project includes unit tests for core functionality:

- `test_get_bid`: Verifies bid calculation with different pacing values
- `test_get_bid_with_different_campaign_id`: Tests campaign-specific values
- `test_get_bid_with_zero_pacing`: Edge case with pacing = 0

**Run tests**:
```bash
cargo test
```

**Run with output**:
```bash
cargo test -- --nocapture
```

---

## Build and Run

**Build**:
```bash
cargo build          # Debug build
cargo build --release  # Optimized build
```

**Run**:
```bash
cargo run            # Run debug build
cargo run --release  # Run optimized build
```

**Check without building**:
```bash
cargo check
```

---

## Design Philosophy

1. **Type Safety**: Extensive use of enums and structs to prevent invalid states
2. **Immutability**: Most data is immutable after creation, mutable only for pacing
3. **Separation of Concerns**: Clear module boundaries (types, simulation, convergence)
4. **Performance**: Index-based lookups, pre-allocated vectors, zero-copy references
5. **Reproducibility**: Deterministic RNG for consistent simulation results
6. **Clarity**: Descriptive names, comprehensive documentation, logical organization

---

*Last Updated: 2025-11-08*

