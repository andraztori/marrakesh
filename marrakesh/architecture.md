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
- How optimal bidding strategies compare to simple multiplicative pacing
- How strategic bidding (cheating) affects marketplace outcomes

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

**Global Random Seed**:
- A global `RAND_SEED` (`AtomicU64`) enables reproducible multiple simulation runs
- Each seeding site XORs the global seed with local seeds for reproducibility
- The seed can be set per iteration to enable multiple runs of the same scenario with different random sequences
- This allows statistical analysis across multiple runs while maintaining reproducibility

---

## Marketplace Model

### Three-Party System

Marrakesh models a marketplace with three distinct parties, each with different objectives and constraints:

1. **Sellers** (Supply Side)
   - Offer impressions with different pricing models
   - May use fixed pricing or auction-based pricing
   - Have finite inventory
   - Seek to maximize revenue
   - Can apply boost factors to influence bid values

2. **Campaigns** (Demand Side)
   - Have specific objectives (impression targets or budget constraints)
   - Value impressions differently based on context
   - Compete for impressions through bidding
   - Operate under optimal pacing (assumed)
   - Use different bidding strategies (multiplicative pacing, optimal bidding, strategic cheating)

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

Campaigns bid based on their bidding strategy. See the "Campaign Types and Bidding Strategies" section below for detailed descriptions of each strategy.

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

**Competition Generation**:
- Uses `CompetitionGeneratorTrait` for extensible competition modeling
- `CompetitionGeneratorNone`: No competition (for fixed-price sellers)
- `CompetitionGeneratorParametrizedLogNormal`: Generates realistic competition using lognormal distributions
- Implements rejection sampling to ensure realistic sigmoid parameters (win probability at zero bid < 5%)
- Uses `base_impression_value` as the sigmoid offset for realistic modeling

---

## Seller Pricing Models

### Two Seller Types

Sellers operate under one of two pricing models:

1. **First Price Auction** (`FIRST_PRICE`):
   - Charges the winning bid amount
   - Generates competition data (`ImpressionCompetition`) for each impression
   - No boost factor (boost is always 1.0)
   - Models auction-based pricing

2. **Fixed Price** (`FIXED_PRICE`):
   - Charges a fixed CPM regardless of winning bid
   - Can use boost factors to influence bid values
   - Does not generate competition data
   - Simple, predictable pricing model

### Seller Convergence Strategies

Sellers can use different convergence strategies for their boost factors:

1. **No Convergence** (`NONE`):
   - Boost factor remains constant at `default_value`
   - Simple, predictable behavior
   - Useful for baseline comparisons

2. **Total Cost Convergence** (`TOTAL_COST`):
   - Boost factor converges to balance total supply cost with target total cost
   - Uses proportional controller for smooth adjustments
   - Enables sellers to optimize revenue while maintaining fixed pricing
   - Allows sellers to influence demand to meet revenue targets

### Seller Boost Factors

Boost factors allow sellers to influence how campaigns value their impressions:
- Applied multiplicatively to campaign bids: `bid = pacing × value × boost_factor`
- Fixed boost: Set once and remains constant (via `NONE` strategy)
- Dynamic boost: Converges to balance seller economics (via `TOTAL_COST` strategy)
- Enables sellers to adjust pricing strategy without changing base cost structure

---

## Campaign Objectives and Constraints

### Two Constraint Types

Campaigns operate under one of two constraint models:

1. **Fixed Impressions** (`TOTAL_IMPRESSIONS`): Campaign wants to obtain exactly N impressions
   - Pacing adjusts to bid more/less aggressively to hit the target
   - Useful for reach-based campaigns
   - Tests how impression constraints affect bidding behavior

2. **Fixed Budget** (`TOTAL_BUDGET`): Campaign wants to spend exactly B dollars
   - Pacing adjusts to spend faster/slower to hit the target
   - Useful for budget-constrained campaigns
   - Tests how budget constraints affect impression acquisition



These models represent the fundamental trade-offs in advertising:
- **Reach vs. Efficiency**: Fixed impressions prioritizes reach; fixed budget prioritizes efficiency
- **Different optimization objectives**: Impression targets optimize for volume; budget targets optimize for cost control

### Campaign Types and Bidding Strategies

Campaigns can use one of three bidding strategies:

1. **Multiplicative Pacing** (`MULTIPLICATIVE_PACING`, `CampaignMultiplicativePacing`):
   - Simple bid calculation: `bid = pacing × value × seller_boost_factor`
   - Pacing multiplier is adjusted to meet campaign targets
   - Works with both impression and budget constraints
   - Straightforward and easy to understand

2. **Optimal Bidding** (`OPTIMAL`, `CampaignOptimalBidding`):
   - Uses sigmoid functions to model win probability
   - Calculates marginal utility of spend from pacing: `marginal_utility = 1.0 / pacing`
   - Uses Newton-Raphson method to find optimal bid via `marginal_utility_of_spend_inverse`
   - Requires competition data (sigmoid parameters) to function
   - More sophisticated, theoretically optimal approach
   - Currently works with budget constraints only

3. **Cheater/Last Look** (`CHEATER`, `CampaignCheaterLastLook`):
   - Bids `value × pacing × seller_boost_factor`
   - If bid exceeds competition, reduces to `competition.bid_cpm + 0.00001`
   - Models strategic bidding that exploits knowledge of competition
   - Simulates second-price auction behavior by bidding just above competition

4. **Max Margin** (`MAX_MARGIN`, `CampaignMaxMargin`):
   - Finds the bid that maximizes expected margin: `P(win) × (value - bid)`
   - Uses bisection method to find the bid where the derivative of margin is zero
   - Requires competition data (sigmoid parameters)
   - Balances win probability against cost to maximize net value

### Convergence Mechanism

The system uses an **iterative feedback loop** to find optimal pacing and boost factors:
- Run auctions with current pacing and boost factors
- Measure actual performance vs. targets
- Adjust pacing/boost proportionally to error using proportional controllers
- Repeat until convergence (no changes in an iteration)

**Campaign Convergence**:
- Campaigns converge their pacing multipliers to meet impression or budget targets
- Uses `ControllerProportional` for smooth adjustments
- Each convergence strategy (`ConvergeTotalImpressions`, `ConvergeTotalBudget`) has its own convergence logic
- Convergence tracks the number of iterations taken to converge, stored in `SimulationStat`

**Seller Convergence**:
- Sellers with `TOTAL_COST` strategy converge boost factors to balance supply costs with target costs
- Sellers with `NONE` strategy maintain constant boost factors
- First-price sellers don't use boost factors

This is not a pacing algorithm to be studied—it's a **simulation calibration tool** that ensures campaigns and sellers operate at their optimal point, allowing clean observation of other marketplace dynamics.

### Convergence Architecture

The system uses a **strategy pattern** for convergence with trait-based dynamic dispatch:

**Core Traits**:
- `ConvergingVariables`: Unified trait for convergence parameters (used by both campaigns and sellers)
- `ConvergeAny<T>`: Generic trait for convergence strategies, parameterized by statistic type
  - Methods: `converge_iteration`, `get_converging_variable`, `create_converging_variables`, `converge_target_string`
  - `get_converging_variable` has a default implementation that delegates to the controller

**Convergence Parameters**:
- `ConvergingSingleVariable`: Concrete type storing a single `f64` value (pacing or boost)
- All current implementations use `ConvergingSingleVariable` for their convergence parameters

**Campaign Convergence Strategies**:
- `ConvergeTotalImpressions`: Converges pacing to meet impression targets
- `ConvergeTotalBudget`: Converges pacing to meet budget targets


**Seller Convergence Strategies**:
- `ConvergeNone`: No convergence, maintains constant boost factor
- `ConvergeTotalCost`: Converges boost factor to meet total cost targets

**Design Benefits**:
- Separation of concerns: Campaign/seller logic separate from convergence logic
- Extensibility: New convergence strategies can be added without modifying campaign/seller code
- Flexibility: Same convergence strategy can work with different campaign/seller types
- Uniform interface: All convergence strategies work with `ConvergingVariables` trait objects

---

## Floor and Competition Generation

### Floor Generation

Floors represent minimum price requirements for impressions. The system uses trait-based floor generation:

**FloorGeneratorTrait**:
- `generate_floor(base_impression_value, rng) -> f64`: Generates floor CPM based on impression value

**Implementations**:
- `FloorGeneratorFixed`: Always returns a fixed floor value
- `FloorGeneratorLogNormal`: Generates floors using lognormal distribution relative to impression value
  - Parameterized by relative ratio and standard deviation
  - Creates realistic floor distributions that scale with impression value

### Competition Generation

Competition data models external competing demand. The system uses trait-based competition generation:

**CompetitionGeneratorTrait**:
- `generate_competition(base_impression_value, rng) -> Option<ImpressionCompetition>`
- Returns `None` if no competition should be generated

**Implementations**:
- `CompetitionGeneratorNone`: Always returns `None` (no competition)
- `CompetitionGeneratorParametrizedLogNormal`: Generates realistic competition data
  - Samples competing bid from lognormal distribution
  - Generates sigmoid parameters for win probability modeling
  - Uses rejection sampling to ensure realistic parameters (win probability at zero bid < 5%)
  - Uses `base_impression_value` as sigmoid offset for realistic modeling

---

## Sigmoid Functions and Optimal Bidding

### Sigmoid Model

The system uses sigmoid functions to model win probability in auctions:

**Sigmoid Structure**:
- `offset`: The bid value where win probability is 0.5
- `scale`: Controls the steepness of the sigmoid curve
- `value`: The value of the impression to the campaign

**Win Probability**:
- `get_probability(bid)`: Returns win probability for a given bid
- Formula: `1.0 / (1 + exp(-(bid - offset) * scale))`

**Marginal Utility**:
- `m(bid)`: Marginal utility of spend at a given bid
- `m_prime(bid)`: Derivative of marginal utility (rate of change)
- Used in optimal bidding to find the bid that maximizes utility

**Inverse Marginal Utility**:
- `marginal_utility_of_spend_inverse(y_target)`: Finds the bid that achieves a target marginal utility
  - Uses Newton-Raphson method with damping and backtracking for stability
  - Falls back to `marginal_utility_of_spend_inverse_numerical_2` if Newton-Raphson fails
- `marginal_utility_of_spend_inverse_numerical_2(y_target, min_x)`: Numerical inverse using bisection
  - Uses bisection method between `min_x` and `100.0` to find the root of `m(x) = y_target`
  - Ensures the result respects the minimum bid constraint (`min_x`, typically the floor price)
  - More robust than Newton-Raphson for edge cases
  - Returns `None` if no solution found in the search range

### Optimal Bidding Algorithm

Optimal bidding uses the following process:

1. Calculate marginal utility of spend from pacing: `marginal_utility = 1.0 / pacing`
2. Calculate impression value: `value = seller_boost_factor × impression.value`
3. Initialize sigmoid with competition parameters and value
4. Find optimal bid: `bid = sigmoid.marginal_utility_of_spend_inverse_numerical_2(marginal_utility, floor_cpm)`
   - Uses bisection method with `floor_cpm` as the minimum bid constraint
   - Ensures bids respect floor prices
5. Return bid (or `None` if calculation fails)

This approach ensures campaigns bid optimally given their budget constraints and competition, while respecting minimum bid requirements.

### Max Margin Bidding Algorithm

Max margin bidding finds the bid that maximizes the expected margin:
`margin(bid) = P(win|bid) × (full_price - bid)`

Where `full_price` is the maximum willingness to pay (`pacing × value`).

The algorithm:
1. Calculates `full_price` based on pacing and value
2. Uses `max_margin_bid_bisection` to find the bid where the derivative of the margin function is zero
3. Solves `scale × (1 - P(bid)) × (full_price - bid) - 1 = 0`
4. Returns the optimal bid for maximizing immediate margin

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
- How does optimal bidding compare to multiplicative pacing?
- What is the impact of strategic bidding (cheating) on marketplace outcomes?

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
2. **Define Demand**: Create campaigns with objectives (impressions or budget) and bidding strategies
3. **Generate Supply**: Create impressions with valuations, floors, and competition data
4. **Initialize Convergence**: 
   - Campaign pacing starts at 1.0 (true value)
   - Seller boost factors start at 1.0 (no boost) or specified default
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
- Campaign bidding strategies (multiplicative pacing, optimal bidding, cheater)
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

**Scenario Execution**:
- Scenarios can be run individually by name: `cargo run -- <scenario_name> [iterations]`
- Or all scenarios can be run: `cargo run -- all [iterations]`
- Optional `iterations` parameter runs each scenario multiple times with different random seeds
- Each iteration uses its iteration number as the global `RAND_SEED` for reproducibility
- When running multiple iterations, each scenario completes all its iterations before moving to the next scenario

**Example Scenarios**:
- `s_one.rs`: Basic marketplace dynamics with multiple campaigns and sellers
- `s_mrg_boost.rs`: Effect of seller boost factors on marketplace outcomes
- `s_mrg_dynamic_boost.rs`: Comparison of fixed vs. dynamic boost strategies
- `s_optimal.rs`: Comparison of multiplicative pacing, optimal bidding, cheater, and max margin strategies
- `s_maxmargin_equality.rs`: Comparison of Optimal Bidding and Max Margin strategies (formerly `s_experiment.rs`). This scenario proves the equality of Max Margin and Optimal Bidding strategies.

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
   - Revenue received (total_provided_value)
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

## Logging System

### Structured Logging

The system uses a structured logging framework with event-based filtering and multiple receivers:

**Log Event Types**:
- `Impression`: Impression data (base values, competition parameters)
- `Auction`: Full auction data (impression data, all bids, auction results)
- `Simulation`: Per-iteration simulation data
- `Convergence`: Convergence information (iteration counts, convergence messages)
- `Variant`: Final converged simulation results for a variant
- `Scenario`: Comparisons between variants, scenario summaries
- `Validation`: Validation results (pass/fail messages, validation checks)

**Log Receivers**:
- `ConsoleReceiver`: Writes to stdout for real-time monitoring
- `FileReceiver`: Writes to files organized by scenario and variant

**Log File Organization**:
- Logs are organized in `log/<scenario_name>/` directories
- `iterations-<variant_name>.log`: Per-iteration simulation and convergence data
- `variant-<variant_name>.log`: Final variant results
- `auctions-<variant_name>-iter<iteration_number>.csv`: Detailed auction data for each iteration
  - Contains full impression data (competition and floor)
  - Lists all bidders for each impression (irrespective of winning)
  - Includes auction result (winner, bid amount, etc.)
  - One dense line per auction in CSV format
  - Only logs value for the first campaign to reduce file size

**Event Hierarchy**:
Log events follow a hierarchy where higher-level events also receive lower-level messages:
- `Simulation` → also receives `Convergence`, `Variant`, `Scenario`, `Validation`
- `Convergence` → also receives `Variant`, `Scenario`, `Validation`
- `Variant` → also receives `Scenario`, `Validation`
- `Scenario` → also receives `Validation`
- `Validation` → only receives validation messages
- `Impression` and `Auction` → standalone events (no hierarchy)

This allows fine-grained control over what gets logged where, enabling detailed analysis while keeping log files manageable.

---

## Visualization and Analysis

### Chart Generation

The system includes comprehensive chart generation capabilities:

**Histogram Generation** (`charts.rs`):
- Generates histograms for impression parameters (base value, floors, competing bids)
- Generates histograms for competition parameters (sigmoid offsets/scales)
- Creates side-by-side comparisons (prediction vs. actual)
- Saves charts to `charts/` directory
- High-resolution output (1600x1200 or 2400x1200)

**Sigmoid Visualization**:
- Visualizes sigmoid functions: `get_probability()`, `m()`, `m_prime()`
- Visualizes inverse marginal utility: `marginal_utility_of_spend_inverse()`
- Marks critical points (offset, error regions)
- Helps debug optimal bidding calculations

**Chart Features**:
- Scaled titles and legends
- Mean lines (vertical, black)
- Consistent color schemes
- Professional presentation

---

## Design Principles

### Separation of Concerns

The system separates:
- **Impression and auction logic** (`impressions.rs`): Core auction mechanics, impression generation, winner determination
- **Campaign logic** (`campaigns.rs`): Campaign types, bidding strategies, campaign convergence
- **Seller logic** (`sellers.rs`): Seller types, pricing models, seller convergence
- **Simulation execution** (`simulationrun.rs`): Running auctions, calculating statistics, marketplace structure
- **Convergence logic** (`converge.rs`): Finding optimal pacing and boost factors, convergence strategies
- **Competition generation** (`competition.rs`): Generating competition data for impressions
- **Floor generation** (`floors.rs`): Generating floor prices for impressions
- **Sigmoid functions** (`sigmoid.rs`): Win probability and marginal utility calculations
- **Visualization** (`charts.rs`): Chart and histogram generation
- **Scenarios** (`s_*.rs`): Experimental setups and validations
- **Initialization** (`main.rs`): Setting up experiments and scenario execution
- **Logging** (`logger.rs`): Structured logging with multiple receivers

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
- `ConvergeAny<T>`: Generic convergence strategy trait parameterized by statistic type
- `CompetitionGeneratorTrait`: Extensible competition generation
- `FloorGeneratorTrait`: Extensible floor generation
- Dynamic dispatch via trait objects enables different implementations while maintaining uniform interfaces

This allows new campaign and seller types to be added without modifying core simulation logic.

### Strategy Pattern for Convergence

The convergence system uses the strategy pattern:
- Campaigns and sellers hold a `Box<dyn ConvergeAny<T>>` converger
- Convergence logic is separated from campaign/seller logic
- New convergence strategies can be added independently
- All strategies work with `ConvergingVariables` trait objects for uniform interface
- Enables flexible combination of campaign/seller types with convergence strategies

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
- Implement `CampaignTrait` with new bid calculation methods
- Add campaign-specific bidding logic
- Experiment with strategic bidding
- Examples: `CampaignMultiplicativePacing`, `CampaignOptimalBidding`, `CampaignCheaterLastLook`

### New Pricing Models
- Add new seller types beyond fixed cost and first-price
- Implement second-price auctions
- Add marketplace fees or discounts
- Extend boost factor strategies

### New Convergence Strategies
- Implement `ConvergeAny<T>` for new convergence logic
- Add new constraint types (time-based, inventory-based)
- Experiment with different control algorithms

### New Competition/Floor Models
- Implement `CompetitionGeneratorTrait` for new competition models
- Implement `FloorGeneratorTrait` for new floor models
- Add realistic market dynamics

### New Metrics
- Add custom statistics to `CampaignStat`, `SellerStat`, `OverallStat`
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
- Compare bidding strategies (pacing vs. optimal vs. strategic)

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
- ✅ Bidding strategy comparison
- ✅ Competition and floor modeling

The framework does not focus on:
- ❌ Pacing algorithm development
- ❌ Real-time decision making
- ❌ User behavior modeling
- ❌ Complex multi-dimensional optimization

---

*This document focuses on conceptual understanding. For implementation details, see the code and inline documentation.*
