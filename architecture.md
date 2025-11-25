# Marrakesh: Conceptual Architecture

## Purpose and Scope

Marrakesh is a **marketplace simulation framework** designed to study auction dynamics, pricing mechanisms, and bidding strategies in digital advertising markets. Unlike many simulation systems that focus on optimizing pacing algorithms, Marrakesh assumes **optimal pacing is already achieved** and uses this assumption to isolate and study other marketplace phenomena.

### Core Research Questions

The framework enables investigation of:
- How different charge models (fixed pricing vs. auction-based) affect market efficiency
- How bidding strategies perform under optimal pacing conditions
- How marketplace rules and constraints shape supply and demand interactions
- How value is distributed between sellers, buyers, and the marketplace itself

---

## Fundamental Assumptions

### Optimal Pacing Assumption

**Critical Design Decision**: The simulation assumes campaigns have access to perfect pacing algorithms that can adjust bid multipliers to exactly meet their targets. This is not a limitation but a deliberate choice.

**Why This Matters**:
- Pacing optimization can be considered a separate problem from marketplace optimization
- By removing pacing as a variable, we can cleanly observe other market dynamics

The convergence mechanism in Marrakesh is **not** a pacing algorithm to be studied—it's a **simulation setup tool** that ensures campaigns operate at their optimal pacing point, allowing researchers to focus on other aspects of marketplace behavior.

### Deterministic Randomness

The simulation uses seeded random number generation to ensure reproducibility. 

**Global Random Seed**:
- A global `RAND_SEED` (`AtomicU64`) enables reproducible multiple simulation runs
- The seed can be set per iteration to enable multiple runs of the same scenario with different random sequences
- This allows analysis across multiple runs while maintaining reproducibility

---

## Marketplace Model

### Two-Party System

Marrakesh models a marketplace with two distinct parties, each with different objectives and constraints:

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

The marketplace itself is the framework that facilitates transactions between sellers and campaigns. It tracks metrics (supply cost, virtual cost, buyer charge), enforces rules (floors, competing demand thresholds), and observes overall market efficiency, but it is not an active participant with its own objectives or constraints.

### Cost Accounting Model

The system tracks three distinct cost metrics to model realistic marketplace economics:

- **Supply Cost**: What sellers actually receive
- **Virtual Cost**: What the marketplace tracks internally (currently `winning_bid_cpm / 1000.0`)
- **Buyer Charge**: What campaigns actually pay (currently `winning_bid_cpm / 1000.0`)

In the current implementation, virtual cost and buyer charge are identical (both equal the winning bid converted from CPM to actual cost), but the separation allows for future modeling of marketplace fees, margins, discounts, or other platform mechanisms.

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

The auction outcomes:
1. **LOST**: Bid is below seller's floor price or below competing external demand
2. **Campaign wins**: Valid bid that passes all checks
3. **NO_DEMAND**: No campaigns participated

This models realistic marketplace constraints where campaigns compete not just with each other, but also with:
- External demand sources (modeled via `ImpressionCompetition`)
- Seller minimum price requirements
- Platform rules and policies

### Fractional Auctions

Marrakesh supports two auction types: **Standard** and **Fractional Internal Auction**. Fractional auctions are a simulation mechanism designed to improve convergence stability and speed.

**How Fractional Auctions Work**:
- Instead of a single winner taking the entire impression, multiple campaigns can win fractions of an impression
- All campaigns with bids above the minimum CPM threshold (floor or competition) are considered winners
- Win fractions are calculated using a softmax function based on bid CPM values: `win_fraction_i = exp(bid_cpm_i) / Σ exp(bid_cpm_j)`
- Each fractional winner receives a proportional share of the impression based on their win fraction
- Supply costs, virtual costs, and buyer charges are weighted by win fractions when aggregating statistics

**Benefits of Fractional Auctions**:
- **Improved Convergence Stability**: By allowing multiple campaigns to share impressions, the system reduces the impact of discrete auction outcomes on convergence. Small changes in pacing don't cause dramatic shifts in win/loss patterns.
- **Faster Convergence**: The smoother gradient provided by fractional allocations helps the convergence algorithm find optimal pacing values more quickly, reducing the number of iterations needed.
- **Better Evaluation of Equivalent Configurations**: Fractional auctions enable more accurate comparison of equivalent campaign setups. For example, in the `s_value_groups` scenario, fractional auctions demonstrate that two campaigns with half budget each behave equivalently to one campaign with double budget when they share the same value group. This equivalence would be harder to observe with standard auctions due to discrete win/loss outcomes.

**When to Use Fractional Auctions**:
- Scenarios where multiple campaigns compete for the same supply and value that supply equally and you want to evaluate their equivalence
- Cases where convergence stability is important and you want to reduce oscillation in pacing adjustments
- Research questions that benefit from smoother gradients in the optimization space

**When to Use Standard Auctions**:
- Scenarios that need to model realistic discrete auction outcomes
- Cases where you want to study the impact of discrete win/loss patterns on campaign behavior
- Research questions focused on auction mechanics rather than convergence behavior

### Impression Competition

Impressions may include optional `ImpressionCompetition` data that models external competing demand. This includes:
- `bid_cpm`: The competing bid that must be exceeded
- Win rate prediction parameters (sigmoid offset/scale) for modeling predicted win probabilities
- Win rate actual parameters (sigmoid offset/scale) for modeling actual win probabilities

Sellers that use first-price auctions generate competition data; fixed-cost sellers do not.

**Competition Generation**:
- Uses `CompetitionGeneratorTrait` for extensible competition modeling
- `CompetitionGeneratorNone`: No competition (for fixed-price sellers)
- `CompetitionGeneratorLogNormal`: Generates realistic competition using lognormal distributions
- Implements rejection sampling to ensure realistic sigmoid parameters (win probability at zero bid < 5%)
- Uses `base_impression_value` as the sigmoid offset for realistic modeling

---

## Seller Pricing Models

### Seller Architecture

All sellers use the `SellerGeneral` structure, which combines several independent components:
- **Convergence Target** (`ConvergeTargetAny<T>`): Defines what to converge to (total cost or none)
- **Convergence Controller** (`ConvergeController`): Defines how to converge (proportional, constant)
- **Competition Generator** (`CompetitionGeneratorTrait`): Generates competition data for impressions
- **Floor Generator** (`FloorGeneratorTrait`): Generates floor prices for impressions
- **Charger** (`SellerCharger`): Defines the pricing model (first-price or fixed-price)

This design allows flexible combination of any convergence strategy, competition model, floor model, and pricing model.

**Seller Addition Methods**:
- `Sellers::add()`: Standard method that takes seller type, convergence strategy, and generators as parameters
  - Automatically constructs `SellerGeneral` internally
  - Sets seller_id automatically based on current collection size
- `Sellers::add_advanced()`: Advanced method that accepts a pre-constructed `Box<dyn SellerTrait>`
  - Allows full control over seller construction
  - Automatically sets seller_id via downcasting to `SellerGeneral`
  - Useful for scenarios requiring custom seller configurations or advanced controller parameters

### Two Seller Pricing Models

Sellers operate under one of two pricing models (implemented as `SellerCharger` trait objects):

1. **First Price Auction** (`FIRST_PRICE`, `SellerChargerFirstPrice`):
   - Charges the winning bid amount: `supply_cost = buyer_win_cpm`
   - Generates competition data (`ImpressionCompetition`) for each impression
   - Boost factor can be used but typically remains at 1.0
   - Models auction-based pricing

2. **Fixed Price** (`FIXED_PRICE`, `SellerChargerFixedPrice`):
   - Charges a fixed CPM regardless of winning bid: `supply_cost = fixed_cost_cpm`
   - Can use boost factors to influence bid values
   - Does not generate competition data (uses `CompetitionGeneratorNone`)
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

Boost factors allow sellers to influence how campaigns value their impressions. The application method depends on the campaign's bidding strategy:

**Multiplicative Boost** (used by most bidding strategies):
- Applied multiplicatively to campaign bids: `bid = pacing × value × boost_factor`
- Used by: MULTIPLICATIVE_PACING, OPTIMAL, CHEATER, MAX_MARGIN, ALB
- Boost factor scales the entire bid proportionally

**Additive Boost** (used by MULTIPLICATIVE_ADDITIVE):
- Applied additively to campaign bids: `bid = pacing × value + boost_factor`
- Used by: MULTIPLICATIVE_ADDITIVE campaign type
- Boost factor adds a fixed amount to the bid, independent of pacing and value
- Useful for studying how additive vs. multiplicative boost factors affect marketplace dynamics

**Boost Factor Strategies**:
- Fixed boost: Set once and remains constant (via `NONE` strategy)
- Dynamic boost: Converges to balance seller economics (via `TOTAL_COST` strategy)
- Enables sellers to adjust pricing strategy without changing base cost structure

---

## Campaign Objectives and Constraints

### Constraint Types

Campaigns operate under one of four constraint models:

1. **Fixed Impressions** (`TOTAL_IMPRESSIONS`): Campaign wants to obtain exactly N impressions
   - Pacing adjusts to bid more/less aggressively to hit the target
   - Useful for reach-based campaigns
   - Tests how impression constraints affect bidding behavior

2. **Fixed Budget** (`TOTAL_BUDGET`): Campaign wants to spend exactly B dollars
   - Pacing adjusts to spend faster/slower to hit the target
   - Useful for budget-constrained campaigns
   - Tests how budget constraints affect impression acquisition

3. **Average Value** (`AVG_VALUE`): Campaign wants to achieve a specific average value per impression
   - Target is specified as `avg_impression_value_to_campaign` 
   - Pacing adjusts to bid more/less aggressively to achieve the target average value
   - Useful for quality-focused campaigns (e.g., viewability targets)
   - Calculates actual as `total_value / impressions_obtained`

4. **No Constraint** (`NONE`): Campaign uses fixed pacing with no convergence
   - Pacing remains constant at the specified `default_pacing` value
   - Useful for baseline comparisons and testing fixed bidding strategies

These models represent the fundamental trade-offs in advertising:
- **Reach vs. Efficiency**: Fixed impressions prioritizes reach; fixed budget prioritizes efficiency
- **Different optimization objectives**: Impression targets optimize for volume; budget targets optimize for cost control
- **Quality vs. Quantity**: Average value targets optimize for quality metrics while maintaining volume

**Dual-Target Campaigns**: The `MAX_MARGIN_DOUBLE_TARGET` campaign type can converge on two targets simultaneously (e.g., total impressions AND average value), using independent pacing variables for each target.

### Campaign Architecture

Campaigns use one of two structures depending on their convergence requirements:

1. **CampaignSimple**: For campaigns with a single convergence target
   - Combines three independent components:
     - **Convergence Target** (`ConvergeTargetAny<T>`): Defines what to converge to (impressions, budget, average value, or none)
     - **Convergence Controller** (`ConvergeController`): Defines how to converge (proportional, constant)
     - **Bidder** (`CampaignBidder`): Defines the bidding strategy
   - Used by most campaign types (MULTIPLICATIVE_PACING, MULTIPLICATIVE_ADDITIVE, OPTIMAL, CHEATER, MAX_MARGIN, ALB)

2. **CampaignDouble**: For campaigns with dual convergence targets
   - Combines four independent components:
     - **Primary Convergence Target** (`ConvergeTargetAny<T>`): First target (e.g., total impressions)
     - **Secondary Convergence Target** (`ConvergeTargetAny<T>`): Second target (e.g., average value)
     - **Convergence Controller** (`ConvergeControllerDouble`): Dual-target controller (e.g., `ConvergeDoubleProportionalController`)
     - **Bidder** (`CampaignBidder`): Defines the bidding strategy
   - Used by `MAX_MARGIN_DOUBLE_TARGET` campaign type
   - Manages two independent pacing variables (lambda and mu) for dual-target convergence

This design allows flexible combination of any convergence target(s), controller, and bidding strategy.

### Campaign Types and Bidding Strategies

Campaigns can use one of seven bidding strategies (implemented as `CampaignBidder` trait objects):

1. **Multiplicative Pacing** (`MULTIPLICATIVE_PACING`, `CampaignBidderMultiplicative`):
   - Simple bid calculation: `bid = pacing × value × seller_boost_factor`
   - Pacing multiplier is adjusted to meet campaign targets
   - Works with both impression and budget constraints
   - Straightforward and easy to understand

2. **Multiplicative Additive** (`MULTIPLICATIVE_ADDITIVE`, `CampaignBidderMultiplicativeAdditive`):
   - Bid calculation uses additive seller boost: `bid = pacing × value + seller_boost_factor`
   - Similar to multiplicative pacing but applies seller boost factor additively rather than multiplicatively
   - Useful for studying how additive vs. multiplicative boost factors affect marketplace dynamics
   - Works with both impression and budget constraints

3. **Optimal Bidding** (`OPTIMAL`, `CampaignBidderOptimal`):
   - Uses sigmoid functions to model win probability
   - Calculates marginal utility of spend from pacing: `marginal_utility = 1.0 / pacing`
   - Uses bisection method (`marginal_utility_of_spend_inverse_numerical_2`) to find optimal bid
   - Requires competition data (sigmoid parameters)
   - More sophisticated, theoretically optimal approach
   - Works with any convergence target (impressions or budget), but requires competition data

4. **Cheater/Last Look** (`CHEATER`, `CampaignBidderCheaterLastLook`):
   - Calculates maximum affordable bid: `max_affordable_bid = pacing × value × seller_boost_factor`
   - Calculates minimum winning bid as `max(floor_cpm, competition.bid_cpm) + 0.00001`
   - Bids the minimum winning bid if affordable, otherwise doesn't bid
   - Models strategic bidding that exploits perfect knowledge of competition
   - Simulates second-price auction behavior by bidding just above competition

5. **Max Margin** (`MAX_MARGIN`, `BidderMaxMargin`):
   - Finds the bid that maximizes expected margin: `P(win) × (full_price - bid)`
   - Where `full_price = pacing × value × seller_boost_factor`
   - Uses bisection method to find the bid where the derivative of margin is zero
   - Requires competition data (sigmoid parameters)
   - Balances win probability against cost to maximize net value
   - Actually the simulation has shown that this approach is equal to Optimal Bidding

6. **ALB (Auction Level Bid)** (`ALB`, `CampaignBidderALB`):
   - Uses multiplicative pacing to calculate initial bid
   - Only bids if the pacing bid is above the predicted offset point
   - If bidding, bids at the predicted offset point (or floor if floor is higher)
   - Requires competition data (for predicted offset)
   - Research observation: ALB improves vs. multiplicative bidding when there is abundance of impressions, but is worse when there is scarcity and high fill rates

7. **Max Margin Double Target** (`MAX_MARGIN_DOUBLE_TARGET`, `BidderMaxMargin`):
   - Uses the same max margin bidding strategy as `MAX_MARGIN`
   - Converges on two targets simultaneously using `CampaignDouble` structure
   - Requires two convergence targets (e.g., total impressions and average value)
   - Uses `ConvergeDoubleProportionalController` to manage dual convergence
   - Useful for campaigns with multiple objectives (e.g., reach and quality targets)
### Convergence Mechanism

The system uses an **iterative feedback loop** to find optimal pacing and boost factors:
- Run auctions with current pacing and boost factors
- Measure actual performance vs. targets
- Adjust pacing/boost proportionally to error using proportional controllers
- Check if any pacing or boost factors changed
- Repeat until convergence (no changes in any pacing or boost factor in an iteration)
- If maximum iterations reached without convergence, log a warning

**Campaign Convergence**:
- Campaigns converge their pacing multipliers to meet impression or budget targets
- Uses `ConvergeControllerProportional` (which wraps `ControllerProportional`) for smooth adjustments
- Each convergence target (`ConvergeTargetTotalImpressions`, `ConvergeTargetTotalBudget`) defines what to converge to
- The controller (`ConvergeControllerProportional`) handles how to converge
- Convergence tracks the number of iterations taken to converge, stored in `SimulationStat`

**Seller Convergence**:
- Sellers with `TOTAL_COST` strategy converge boost factors to balance supply costs with target costs
- Sellers with `NONE` strategy maintain constant boost factors
- First-price sellers don't use boost factors

This is not a pacing algorithm to be studied—it's a **simulation calibration tool** that ensures campaigns and sellers operate at their optimal point, allowing clean observation of other marketplace dynamics.

### Convergence Architecture

The system uses a **strategy pattern** for convergence with trait-based dynamic dispatch:

**Core Traits and Types**:
- `ControllerState`: Trait for controller state (replaces `ConvergingVariables`)
- `ControllerStateSingleVariable`: Concrete type storing a single `f64` value (pacing or boost)
- `ConvergeTargetAny<T>`: Generic trait for convergence targets, parameterized by statistic type
  - Methods: `get_actual_and_target`, `converge_target_string`
  - Provides the target value and actual value for convergence calculations
- `ConvergeController`: Trait for controlling convergence behavior
  - Methods: `next_controller_state`, `get_control_variable`, `create_controller_state`, `controller_string`
  - Handles the actual convergence logic and state management

**Controller Implementations**:
- `ConvergeControllerConstant`: Constant controller that maintains a fixed value
  - Used for campaigns/sellers with `ConvergeNone` target
  - Initializes controller state with the specified default value
- `ConvergeControllerProportional`: Proportional controller that adjusts based on error between target and actual
  - Uses `ControllerProportional` internally for the control algorithm
  - Default parameters: tolerance_fraction=0.005 (0.5%), max_adjustment_factor=0.2 (20%), proportional_gain=0.1 (10%)
  - Initializes controller state with pacing/boost = 1.0
  - Provides `new_advanced()` method for custom parameter configuration:
    - `tolerance_fraction`: Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    - `max_adjustment_factor`: Maximum adjustment factor (e.g., 0.5 = 50%)
    - `proportional_gain`: Proportional gain (e.g., 1.0 = 100% of error)
  - Useful for scenarios requiring more aggressive convergence (e.g., additive bidding strategies)
- `ConvergeDoubleProportionalController`: Dual-target proportional controller for campaigns with two convergence targets
  - Uses two independent `ControllerProportional` instances
  - Primary controller uses default parameters
  - Secondary controller uses advanced parameters: tolerance_fraction=0.005, max_adjustment_factor=0.5, proportional_gain=1.0
  - Manages dual controller state (`ControllerStateDualVariable`) with two independent pacing variables
  - Used by `CampaignDouble` for dual-target convergence

**Campaign Convergence Targets**:
- `ConvergeTargetTotalImpressions`: Target is total impressions obtained
- `ConvergeTargetTotalBudget`: Target is total budget spent (uses `total_buyer_charge`)
- `ConvergeTargetAvgValue`: Target is average value per impression (uses `total_value / impressions_obtained`)
  - Target value is specified as `avg_impression_value_to_campaign` (scaled by 1000 when instantiated)
  - Useful for quality-focused campaigns (e.g., viewability targets)
- `ConvergeNone`: No target (constant pacing, with configurable default value)

**Seller Convergence Targets**:
- `ConvergeNone`: No target (constant boost factor, with configurable default value)
- `ConvergeTargetTotalCost`: Target is total cost (uses `total_virtual_cost` from seller statistics)

**State Containers**:
- `CampaignControllerStates`: Container for campaign controller states (vector of `Box<dyn ControllerState>`)
- `SellerControllerStates`: Container for seller controller states (vector of `Box<dyn ControllerState>`)
- Both use the unified `ControllerStateSingleVariable` type internally

**Design Benefits**:
- Clear separation: Convergence targets define what to converge to, controllers define how to converge
- Extensibility: New convergence targets and controllers can be added independently
- Flexibility: Any combination of target and controller can be used
- Uniform interface: All campaigns and sellers work with `ControllerState` trait objects

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
- `CompetitionGeneratorLogNormal`: Generates realistic competition data
  - Samples competing bid from lognormal distribution
  - Generates sigmoid parameters for win probability modeling
  - Uses rejection sampling to ensure realistic parameters (win probability at zero bid < 5%)
  - Uses `base_impression_value` as sigmoid offset for realistic modeling

#### Building Realistic Competitive Markets

The `CompetitionGeneratorLogNormal` implementation uses several key considerations to generate competition that resembles real-world auction dynamics:

**1. Base Value as 50% Win Rate Point**:
- The `impression_base_value` parameter is used as the value at which 50% win rate occurs
- This value becomes the **offset** (center point) of the logistic curve representing the actual win probability
- The **scale** parameter is then randomized from a lognormal distribution to create variation in the steepness of the win rate curve
- This approach ensures that higher-value impressions naturally have higher competition thresholds

**2. Rejection Sampling for Realistic Low-Bid Behavior**:
- Simple random sampling of sigmoid parameters can lead to unrealistic scenarios where win probability at minimal CPM (e.g., $0.0001) is too high
- To address this, the system uses **rejection sampling**: it repeatedly samples offset and scale parameters until the resulting sigmoid has a win probability at zero bid ≤ 2% (0.02)
- This ensures that very low bids have near-zero win probability, which matches real-world auction behavior where minimal bids rarely win

**3. Sampling Competing Bids**:
- Once acceptable sigmoid parameters are found, the system samples an actual competing bid from the logistic distribution defined by these parameters
- This sampled bid (`bid_cpm`) represents the external competing demand that must be exceeded to win the auction
- The bid is clipped to be non-negative to ensure realistic auction constraints

**4. Perturbation for Imperfect Modeling**:
- In real systems, win rate prediction models are imperfect and have estimation errors
- To simulate this, the system applies **multiplicative lognormal noise** to the actual sigmoid parameters
- This creates `win_rate_prediction_sigmoid_offset` and `win_rate_prediction_sigmoid_scale` that differ from the actual parameters
- The noise distributions are:
  - Offset noise: lognormal(mean=1.0, stddev=0.1)
  - Scale noise: lognormal(mean=1.0, stddev=0.05)
- This allows testing of bidding strategies under conditions where predicted win rates don't perfectly match actual win rates

**5. Debugging Information**:
- The system retains both the actual parameters (`win_rate_actual_sigmoid_offset`, `win_rate_actual_sigmoid_scale`) and the perturbed prediction parameters
- While the actual parameters could theoretically be discarded after sampling the competing bid, they are kept for debugging and analysis purposes
- This enables researchers to:
  - Compare predicted vs. actual win rates
  - Understand the impact of prediction errors on bidding strategies
  - Validate that the competition generation process produces realistic distributions
  - Debug issues with optimal bidding algorithms that depend on win rate predictions

This multi-step process ensures that the generated competition data reflects realistic auction dynamics while providing the flexibility to study the impact of prediction errors and modeling imperfections on bidding strategies.

---

## Sigmoid Functions and Optimal Bidding

### Sigmoid Model

The system uses sigmoid functions to model win probability in auctions:

**Sigmoid Structure**:
- `offset`: The bid value where win probability is 0.5
- `scale`: Controls the steepness of the sigmoid curve
- `value`: The value of the impression to the campaign

**Key functions**:
- `get_probability(bid)`: Returns win probability for a given bid
  - Formula: `1.0 / (1 + exp(-(bid - offset) * scale))`
- `m(bid)`: Marginal utility of spend at a given bid
  - Formula: `(value * scale * (1 - s(bid))) / (scale * bid * (1 - s(bid)) + 1)`
  - Where `s(bid) = get_probability(bid)`
- `m_prime(bid)`: Derivative of marginal utility (rate of change)
  - Formula: `-value * scale² * (1 - s(bid)) / (scale * bid * (1 - s(bid)) + 1)²`
- `marginal_utility_of_spend_inverse_numerical_2(y_target, min_x)`: Numerical inverse using bisection
  - Uses bisection method to find the root of `m(x) = y_target`
  - Searches between `min_x` and an expanding upper bound (starts at 1000.0)
  - Handles edge cases (both bounds negative, both positive, etc.)
  - Ensures the result respects the minimum bid constraint (`min_x`, typically the floor price)
  - Returns `None` if no solution found in the search range
- `max_margin_bid_bisection(full_price, min_x)`: Finds bid that maximizes expected margin
  - Solves for bid where derivative of margin is zero: `scale * (1 - P(bid)) * (full_price - bid) - 1 = 0`
  - Uses bisection method between `min_x` and `full_price`

### Optimal Bidding Algorithm

Optimal bidding uses the following process:

1. Calculate marginal utility of spend from pacing: `marginal_utility = 1.0 / pacing`
2. Calculate impression value: `value = seller_boost_factor × impression.value`
3. Initialize sigmoid with competition parameters and value
4. Find optimal bid: `bid = sigmoid.marginal_utility_of_spend_inverse_numerical_2(marginal_utility, floor_cpm.max(0.0))`
   - Uses bisection method with `floor_cpm` as the minimum bid constraint
   - Handles edge cases (both bounds negative, both positive, etc.)
   - Ensures bids respect floor prices
5. Return bid (or `None` if calculation fails or bid is below floor)

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
- How does ALB (Auction Level Bid) perform compared to other strategies?
- How do max margin and optimal bidding compare (they appear to be equivalent)?

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
   - Campaign pacing starts at 1.0 for proportional controllers, or at specified default for constant controllers
   - Seller boost factors start at 1.0 for proportional controllers, or at specified default for constant controllers
   - `SimulationConverge` encapsulates marketplace and initial controller states
   - `CampaignControllerStates` and `SellerControllerStates` are created via `create_controller_state()` for each campaign/seller

### Convergence Phase

Iteratively adjust pacing and boost factors until campaigns and sellers meet their targets:
- Run full auction simulation with current controller states
- Calculate performance metrics for campaigns and sellers
- Use `next_controller_state()` to calculate new controller states based on targets
- Adjust campaign pacing based on impression/budget targets using `ConvergeControllerProportional`
- Adjust seller boost factors based on cost balance (for dynamic boost sellers) using `ConvergeControllerProportional`
- Repeat until convergence (no changes in an iteration)

This phase ensures campaigns and sellers operate optimally before observation begins. The `SimulationConverge` struct manages this process, encapsulating the marketplace and controller states.

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
- `HBabundance` (from `s_one.rs`): Basic marketplace dynamics with multiple campaigns and sellers, comparing scarce vs. abundant supply scenarios
- `MRGboost` (from `s_mrg_boost.rs`): Effect of seller boost factors on marketplace outcomes
- `MRGdynamicboost` (from `s_mrg_dynamic_boost.rs`): Comparison of fixed vs. dynamic boost strategies, including multiplicative and additive bidding strategies
  - Variant A: Fixed boost (no convergence) with MULTIPLICATIVE_PACING
  - Variant B: Dynamic boost with MULTIPLICATIVE_PACING
  - Variant C: Dynamic boost with MULTIPLICATIVE_ADDITIVE (uses advanced controller parameters)
- `viewability` (from `s_viewability.rs`): Demonstrates dual-target convergence using MAX_MARGIN_DOUBLE_TARGET
  - Converges on both total impressions and average value targets simultaneously
- `various` (from `s_various.rs`): Comparison of all bidding strategies (multiplicative pacing, optimal bidding, cheater, max margin, ALB)
- `maxmargin_equality` (from `s_maxmargin_equality.rs`): Comparison of Optimal Bidding and Max Margin strategies. This scenario demonstrates that Max Margin and Optimal Bidding appear to be equivalent when configured correctly.

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

There are some development-time tools an visualizations in charts.rs. They can be mostly ignored.

---

## Design Principles

### Separation of Concerns

The system separates:
- **Impression and auction logic** (`impressions.rs`): Core auction mechanics, impression generation, winner determination
- **Campaign logic** (`campaigns.rs`): Campaign trait, `CampaignSimple` and `CampaignDouble` structures, campaign container
- **Campaign bidding strategies** (`campaign_bidders.rs`): Bidding strategy implementations (multiplicative, multiplicative additive, optimal, max margin, cheater, ALB)
- **Campaign convergence targets** (`campaign_targets.rs`): Campaign convergence target implementations (impressions, budget, average value, none)
- **Seller logic** (`sellers.rs`): Seller trait, `SellerGeneral` structure, seller container
- **Seller charging strategies** (`seller_chargers.rs`): Pricing model implementations (first-price, fixed-price)
- **Seller convergence targets** (`seller_targets.rs`): Seller convergence target implementations
- **Simulation execution** (`simulationrun.rs`): Running auctions, calculating statistics, marketplace structure
- **Convergence logic** (`converge.rs`): Finding optimal pacing and boost factors, convergence targets, controller state management
- **Controller logic** (`controllers.rs`): Controller implementations (proportional, constant, double proportional), unified controller state types
- **Controller core** (`controller_core.rs`): Core proportional controller algorithm with configurable parameters
- **Competition generation** (`competition.rs`): Generating competition data for impressions
- **Floor generation** (`floors.rs`): Generating floor prices for impressions
- **Sigmoid functions** (`sigmoid.rs`): Win probability and marginal utility calculations
- **Visualization** (`charts.rs`): Chart and histogram generation
- **Scenarios** (`s_*.rs`): Experimental setups and validations
- **Scenario framework** (`scenarios.rs`): Scenario registration and catalog system
- **Initialization** (`main.rs`): Setting up experiments and scenario execution
- **Logging** (`logger.rs`): Structured logging with multiple receivers
- **Utilities** (`utils.rs`): Random number generation, distributions, helper functions

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
- `CampaignBidder`: Trait for bidding strategies (used by `CampaignSimple` and `CampaignDouble`)
- `SellerCharger`: Trait for pricing models (used by `SellerGeneral`)
- `ConvergeTargetAny<T>`: Generic convergence target trait parameterized by statistic type
- `ConvergeController`: Trait for controlling convergence behavior (single-target campaigns)
- `ConvergeControllerDouble`: Trait for controlling dual-target convergence behavior (used by `CampaignDouble`)
- `ControllerState`: Trait for controller state representation (unified for campaigns and sellers)
- `CompetitionGeneratorTrait`: Extensible competition generation
- `FloorGeneratorTrait`: Extensible floor generation
- Dynamic dispatch via trait objects enables different implementations while maintaining uniform interfaces

This allows new bidding strategies, pricing models, convergence targets, and controllers to be added without modifying core simulation logic.

### Strategy Pattern for Convergence

The convergence system uses the strategy pattern with two layers:
- **Convergence Targets** (`ConvergeTargetAny<T>`): Define what to converge to (e.g., total impressions, total budget)
  - Campaigns and sellers hold a `Box<dyn ConvergeTargetAny<T>>` to define their convergence target
  - Provides `get_actual_and_target()` to compare current state with target
- **Convergence Controllers** (`ConvergeController`): Define how to converge (e.g., proportional control, constant)
  - Campaigns and sellers hold a `Box<dyn ConvergeController>` to control convergence behavior
  - Handles state management and adjustment logic
- All controllers work with `ControllerState` trait objects for uniform interface
- Enables flexible combination of targets and controllers (e.g., proportional control for budget targets, constant control for fixed pacing)

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
- Implement `CampaignBidder` trait with new bid calculation methods
- Add the new bidder to `CampaignType` enum and `Campaigns::add()` method
- Examples: All campaigns use `CampaignSimple` (or `CampaignDouble` for dual-target campaigns) with different `CampaignBidder` implementations

### New Pricing Models
- Implement `SellerCharger` trait with new pricing logic
- Add the new charger to `SellerType` enum and `Sellers::add()` method
- Examples: All sellers use `SellerGeneral` with different `SellerCharger` implementations
- Future possibilities: second-price auctions, marketplace fees or discounts

### New Convergence Strategies
- Implement `ConvergeTargetAny<T>` for new convergence targets
- Implement `ConvergeController` for new control algorithms
- Add new constraint types (time-based, inventory-based)
- Experiment with different control algorithms (PID, adaptive, etc.)

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
