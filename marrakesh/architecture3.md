# Marrakesh: Conceptual Architecture

## Purpose and Scope

Marrakesh is a **marketplace simulation framework** designed to study auction dynamics, pricing mechanisms, and bidding strategies in digital advertising markets. Unlike many simulation systems that focus on optimizing pacing algorithms, Marrakesh assumes **optimal pacing is already achieved** and uses this assumption to isolate and study other marketplace phenomena.

### Core Research Questions

The framework enables investigation of:
- How different charge models (fixed pricing vs. auction-based) affect market efficiency
- How bidding strategies perform under optimal pacing conditions
- How marketplace rules and constraints shape supply and demand interactions
- How value is distributed between sellers, buyers, and the marketplace itself
- How different campaign objectives (impression targets vs. budget constraints) interact

---

## Fundamental Assumptions

### Optimal Pacing Assumption

**Critical Design Decision**: The simulation assumes campaigns have access to perfect pacing algorithms that can adjust bid multipliers to exactly meet their targets. This is not a limitation but a deliberate choice.

**Why This Matters**:
- Pacing optimization can be considered a separate problem from marketplace optimization
- The interesting research questions lie in pricing, bidding strategies, and marketplace design
- By removing pacing as a variable, we can cleanly observe other dynamics
- Real-world systems typically have pacing solutions; the question is what happens after pacing is solved

The convergence mechanism in Marrakesh is **not** a pacing algorithm to be studied—it's a **simulation setup tool** that ensures campaigns operate at their optimal pacing point, allowing researchers to focus on other aspects of marketplace behavior.

### Deterministic Randomness

The simulation uses seeded random number generation to ensure reproducibility. This allows:
- Exact replication of experiments
- Controlled comparison between different configurations
- Debugging and validation of results
- Scientific rigor in experimentation

---

## Marketplace Model

### Three-Party System

Marrakesh models a marketplace with three distinct parties, each with different objectives and constraints:

1. **Sellers** (Supply Side)
   - Offer impressions with different pricing models
   - May use fixed pricing or auction-based pricing
   - Have finite inventory
   - Seek to maximize revenue

2. **Campaigns** (Demand Side)
   - Have specific objectives (impression targets or budget constraints)
   - Value impressions differently based on context
   - Compete for impressions through bidding
   - Operate under optimal pacing (assumed)

3. **Marketplace** (Platform)
   - Facilitates transactions
   - Tracks multiple cost metrics (supply cost, virtual cost, buyer charge)
   - May have rules (floors, competing demand thresholds)
   - Observes overall market efficiency

### Cost Accounting Model

The system tracks three distinct cost metrics to model realistic marketplace economics:

- **Supply Cost**: What sellers actually receive
- **Virtual Cost**: What the marketplace tracks internally
- **Buyer Charge**: What campaigns actually pay

In the current implementation, virtual cost and buyer charge are identical, but the separation allows for future modeling of marketplace fees, margins, discounts, or other platform mechanisms.

---

## Auction Mechanics

### Impression Valuation

Each impression has a **value array**—one value per campaign—representing how valuable that impression is to each potential buyer. This models the reality that:
- Different campaigns have different targeting criteria
- Context matters (time, location, user characteristics)
- Campaign objectives vary (brand awareness vs. conversions)

### Bidding Process

Campaigns bid based on:
- Their valuation of the impression
- Their current pacing multiplier (optimized to meet targets)
- The seller boost factor (applied by sellers to adjust bid values)

The bid formula currently is: `bid = pacing × value × seller_boost_factor`. This will evolve as simluation gets more sophisticated.

The seller boost factor allows for experimentation of what happens when seller can influence how buyer is valuing certain impression. This mainly can happen when we control both sides of the market and want to satisfy sellers to which we have promissed certain revenues. The other option of seller influencing the buyer is by setting the floors.

### Winner Determination

The auction uses a **first-price sealed-bid** model with additional constraints:
- Bids must exceed seller floor prices (`floor_cpm`) - checked first
- Bids must exceed competing external demand (`bid_cpm` from `ImpressionCompetition`) - if competition data exists
- Highest valid bid wins

The auction logic checks constraints in order:
1. **BELOW_FLOOR**: Bid is below seller's floor price
2. **OTHER_DEMAND**: Bid is below competing external demand (if competition data exists)
3. **Campaign wins**: Valid bid that passes all checks
4. **NO_DEMAND**: No campaigns participated

This models realistic marketplace constraints where campaigns compete not just with each other, but also with:
- External demand sources (modeled via `ImpressionCompetition`)
- Seller minimum price requirements
- Platform rules and policies

### Impression Competition

Impressions may include optional `ImpressionCompetition` data that models external competing demand. This includes:
- `bid_cpm`: The competing bid that must be exceeded
- Win rate prediction parameters (sigmoid offset/scale) for modeling predicted win probabilities
- Win rate actual parameters (sigmoid offset/scale) for modeling actual win probabilities

Sellers that use first-price auctions generate competition data; fixed-cost sellers do not.

---

## Seller Pricing Models

### Three Seller Types

Sellers operate under one of three pricing models:

1. **Fixed Cost with Fixed Boost** (`FIXED_COST_FIXED_BOOST`): 
   - Charges a fixed CPM regardless of winning bid
   - Uses a fixed boost factor (typically 1.0, but can be set)
   - Boost factor does not converge
   - Simple, predictable pricing model

2. **Fixed Cost with Dynamic Boost** (`FIXED_COST_DYNAMIC_BOOST`):
   - Charges a fixed CPM regardless of winning bid
   - Uses a dynamic boost factor that converges
   - Boost factor adjusts to balance total supply cost with total virtual cost - supply is thus forcing buying to the demand side
   - Enables sellers to optimize revenue while maintaining fixed pricing

3. **First Price Auction** (`FIRST_PRICE`):
   - Charges the winning bid amount
   - Generates competition data (`ImpressionCompetition`) for each impression
   - No boost factor (boost is always 1.0)
   - Models auction-based pricing

### Seller Boost Factors

Boost factors allow sellers to influence how campaigns value their impressions:
- Applied multiplicatively to campaign bids: `bid = pacing × value × boost_factor`
- Fixed boost: Set once and remains constant
- Dynamic boost: Converges to balance seller economics
- Enables sellers to adjust pricing strategy without changing base cost structure

---

## Campaign Objectives and Constraints

### Two Constraint Types

Campaigns operate under one of two constraint models:

1. **Fixed Impressions**: Campaign wants to obtain exactly N impressions
   - Pacing adjusts to bid more/less aggressively to hit the target
   - Useful for reach-based campaigns
   - Tests how impression constraints affect bidding behavior

2. **Fixed Budget**: Campaign wants to spend exactly B dollars
   - Pacing adjusts to spend faster/slower to hit the target
   - Useful for budget-constrained campaigns
   - Tests how budget constraints affect impression acquisition

These two models represent the fundamental trade-offs in advertising:
- **Reach vs. Efficiency**: Fixed impressions prioritizes reach; fixed budget prioritizes efficiency
- **Different optimization objectives**: Impression targets optimize for volume; budget targets optimize for cost control

### Convergence Mechanism

The system uses an **iterative feedback loop** to find optimal pacing and boost factors:
- Run auctions with current pacing and boost factors
- Measure actual performance vs. targets
- Adjust pacing/boost proportionally to error using proportional controllers
- Repeat until convergence (no changes in an iteration)

**Campaign Convergence**:
- Campaigns converge their pacing multipliers to meet impression or budget targets
- Uses `ControllerProportional` for smooth adjustments
- Each campaign type has its own convergence logic

**Seller Convergence**:
- Sellers with dynamic boost factors converge to balance supply costs with virtual costs
- Fixed boost sellers maintain constant boost factors
- First-price sellers don't use boost factors

This is not a pacing algorithm to be studied—it's a **simulation calibration tool** that ensures campaigns and sellers operate at their optimal point, allowing clean observation of other marketplace dynamics.

### Convergence Architecture

The system uses trait-based dynamic dispatch for convergence:
- `CampaignConverge` trait: Defines convergence parameter interface for campaigns
- `CampaignConvergePacing`: Concrete convergence parameter storing pacing multiplier
- `SellerConverge` trait: Defines convergence parameter interface for sellers
- `SellerConvergeBoost`: Concrete convergence parameter storing boost factor
- `CampaignConverges` / `SellerConverges`: Containers holding convergence parameters for all campaigns/sellers

This design allows each campaign and seller type to have its own convergence logic while maintaining a uniform interface.

---

## Research Capabilities

### What Can Be Studied

With optimal pacing assumed, researchers can focus on:

**Pricing Mechanisms**:
- How do fixed-price sellers compare to auction-based sellers?
- What happens when sellers mix pricing models?
- How do floor prices affect market efficiency?

**Bidding Strategies**:
- How do different valuation models affect outcomes?
- What happens when campaigns have correlated vs. uncorrelated valuations?
- How do campaign objectives (impressions vs. budget) affect bidding?

**Marketplace Design**:
- How do floors and competing demand thresholds affect outcomes?
- What is the impact of marketplace rules on efficiency?
- How is value distributed across parties?

**Supply and Demand Dynamics**:
- How does supply composition affect campaign performance?
- What happens when supply is scarce vs. abundant?
- How do campaigns compete when objectives conflict?

### What Cannot Be Studied

The framework explicitly does **not** study:
- Pacing algorithms (assumed optimal)
- Real-time pacing adjustments (pacing is pre-converged)
- Pacing strategy comparisons (not the research focus)
- Sub-optimal pacing scenarios (outside scope)

---

## Simulation Workflow

### Setup Phase

1. **Define Marketplace**: Create sellers with pricing models and inventory
2. **Define Demand**: Create campaigns with objectives (impressions or budget)
3. **Generate Supply**: Create impressions with valuations and constraints
4. **Initialize Convergence**: 
   - Campaign pacing starts at 1.0 (true value)
   - Seller boost factors start at 1.0 (no boost)
   - `SimulationConverge` encapsulates marketplace and initial convergence state

### Convergence Phase

Iteratively adjust pacing and boost factors until campaigns and sellers meet their targets:
- Run full auction simulation with current convergence parameters
- Calculate performance metrics for campaigns and sellers
- Adjust campaign pacing based on impression/budget targets
- Adjust seller boost factors based on cost balance (for dynamic boost sellers)
- Repeat until convergence (no changes in an iteration)

This phase ensures campaigns and sellers operate optimally before observation begins. The `SimulationConverge` struct manages this process, encapsulating the marketplace and convergence state.

### Observation Phase

Once converged, analyze:
- Campaign performance metrics
- Seller revenue and fill rates
- Marketplace efficiency
- Value distribution
- Supply/demand balance

### Experimentation

Researchers can then vary:
- Seller pricing models (fixed cost, first price, boost strategies)
- Campaign objectives and constraints (impressions vs. budget)
- Impression valuations and competition data
- Marketplace rules (floors, thresholds)
- Supply composition
- Boost factor strategies

And observe how these changes affect outcomes under optimal pacing and boost conditions.

### Scenario Framework

The system includes a scenario framework for structured experimentation:
- Scenarios are registered via `inventory::submit!` macro
- Each scenario defines variants to compare
- Scenarios include validation logic to verify expected behavior
- Logging is organized by scenario and variant for easy analysis

Example scenarios:
- `s_one.rs`: Basic marketplace dynamics
- `s_mrg_boost.rs`: Effect of seller boost factors
- `s_mrg_dynamic_boost.rs`: Comparison of fixed vs. dynamic boost strategies

---

## Statistical Framework

### Three Levels of Analysis

The system provides statistics at three levels:

1. **Campaign Level**: Performance of individual campaigns
   - Impressions obtained vs. targets
   - Costs (supply, virtual, buyer)
   - Value obtained
   - Efficiency metrics

2. **Seller Level**: Performance of individual sellers
   - Impressions sold vs. offered
   - Revenue received
   - Fill rates
   - Pricing model effectiveness

3. **Marketplace Level**: Overall system performance
   - Total transactions
   - Unfilled inventory (below floor, other demand, no bids)
   - Aggregate costs and value
   - Market efficiency metrics

This multi-level view allows researchers to understand:
- Individual participant behavior
- Market-wide dynamics
- Interactions between levels

---

## Design Principles

### Separation of Concerns

The system separates:
- **Impression and auction logic** (impressions.rs): Core auction mechanics, impression generation, winner determination
- **Campaign logic** (campaigns.rs): Campaign types, bidding strategies, campaign convergence
- **Seller logic** (sellers.rs): Seller types, pricing models, seller convergence
- **Simulation execution** (simulationrun.rs): Running auctions, calculating statistics, marketplace structure
- **Convergence logic** (converge.rs): Finding optimal pacing and boost factors
- **Scenarios** (s_*.rs): Experimental setups and validations
- **Initialization** (main.rs): Setting up experiments and scenario execution

This allows each component to be understood, tested, and modified independently.

### Index-Based Identity

All entities use vector indices as IDs, ensuring:
- O(1) lookups
- Type safety (no negative IDs)
- Automatic ID assignment
- Simple relationships between entities

This design choice prioritizes simplicity and performance over flexibility.

### Trait-Based Polymorphism

The system uses Rust traits for extensibility:
- `CampaignTrait`: Defines campaign interface (bidding, convergence, statistics)
- `SellerTrait`: Defines seller interface (pricing, impression generation, convergence)
- `CampaignConverge` / `SellerConverge`: Define convergence parameter interfaces
- Dynamic dispatch via trait objects enables different implementations while maintaining uniform interfaces

This allows new campaign and seller types to be added without modifying core simulation logic.

### Deterministic Execution

The entire simulation is deterministic through:
- Seeded random number generation
- Fixed iteration limits
- Explicit convergence criteria

This enables reproducible research and controlled experimentation.

---

## Extensibility

The framework is designed to be extended without modifying core logic:

### New Bidding Strategies
- Implement different bid calculation methods
- Add campaign-specific bidding logic
- Experiment with strategic bidding

### New Pricing Models
- Add new seller types beyond fixed cost and first-price
- Implement second-price auctions
- Add marketplace fees or discounts
- Extend boost factor strategies

### New Constraints
- Add multi-dimensional constraints (both impressions AND budget)
- Implement time-based constraints
- Add inventory constraints

### New Metrics
- Add custom statistics
- Implement efficiency measures
- Track additional performance indicators

### New Marketplace Rules
- Add reserve prices
- Implement bid shading
- Add quality thresholds

---

## Use Cases

### Industry Analysis
- Model real-world marketplace scenarios
- Test pricing strategies before deployment
- Understand supply/demand interactions
- Optimize marketplace rules
- Validate pricing mechanisms
- Develop marketplace optimization techniques

---

## Limitations and Scope

### What Marrakesh Is

- A framework for studying marketplace dynamics under optimal pacing
- A tool for comparing pricing mechanisms and bidding strategies
- A controlled environment for experimentation
- A foundation for building more complex simulations

### What Marrakesh Is Not

- A pacing optimization system (pacing is assumed optimal)
- A real-time marketplace simulator (executes in batch)
- A production system (designed for research, not deployment)
- A complete marketplace model (focuses on core dynamics)

### Scope Boundaries

The framework focuses on:
- ✅ Auction mechanics and winner determination
- ✅ Pricing model comparison
- ✅ Campaign objective optimization
- ✅ Marketplace efficiency analysis

The framework does not focus on:
- ❌ Pacing algorithm development
- ❌ Real-time decision making
- ❌ User behavior modeling
- ❌ Complex multi-dimensional optimization

---

*This document focuses on conceptual understanding. For implementation details, see the code and inline documentation.*

