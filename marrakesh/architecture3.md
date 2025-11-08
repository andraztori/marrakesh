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
- Pacing optimization is a well-studied problem with many existing solutions
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

In the current implementation, virtual cost and buyer charge are identical, but the separation allows for future modeling of marketplace fees, discounts, or other platform mechanisms.

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
- Their competitive position

The bid formula: `bid = pacing × value`

This simple model allows complex behavior to emerge from the interaction of multiple campaigns with different objectives.

### Winner Determination

The auction uses a **first-price sealed-bid** model with additional constraints:
- Bids must exceed competing external demand (`best_other_bid_cpm`)
- Bids must exceed seller floor prices (`floor_cpm`)
- Highest valid bid wins

This models realistic marketplace constraints where campaigns compete not just with each other, but also with:
- External demand sources
- Seller minimum price requirements
- Platform rules and policies

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

The system uses an **iterative feedback loop** to find optimal pacing:
- Run auctions with current pacing
- Measure actual performance vs. targets
- Adjust pacing proportionally to error
- Repeat until convergence (within 1% tolerance)

This is not a pacing algorithm to be studied—it's a **simulation calibration tool** that ensures campaigns operate at their optimal point, allowing clean observation of other marketplace dynamics.

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
4. **Initialize Pacing**: Start with neutral pacing (1.0 = true value)

### Convergence Phase

Iteratively adjust pacing until campaigns meet their targets:
- Run full auction simulation
- Calculate performance metrics
- Adjust pacing based on error
- Repeat until convergence

This phase ensures campaigns operate optimally before observation begins.

### Observation Phase

Once converged, analyze:
- Campaign performance metrics
- Seller revenue and fill rates
- Marketplace efficiency
- Value distribution
- Supply/demand balance

### Experimentation

Researchers can then vary:
- Seller pricing models
- Campaign objectives and constraints
- Impression valuations
- Marketplace rules (floors, thresholds)
- Supply composition

And observe how these changes affect outcomes under optimal pacing conditions.

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
- **Data structures** (types.rs): Core entities and their relationships
- **Simulation execution** (simulationrun.rs): Running auctions and calculating statistics
- **Convergence logic** (converge.rs): Finding optimal pacing
- **Initialization** (main.rs): Setting up experiments

This allows each component to be understood, tested, and modified independently.

### Index-Based Identity

All entities use vector indices as IDs, ensuring:
- O(1) lookups
- Type safety (no negative IDs)
- Automatic ID assignment
- Simple relationships between entities

This design choice prioritizes simplicity and performance over flexibility.

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
- Add new charge types beyond fixed cost and first-price
- Implement second-price auctions
- Add marketplace fees or discounts

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

### Academic Research
- Study auction theory in controlled environments
- Test hypotheses about marketplace design
- Compare pricing mechanisms
- Analyze market efficiency

### Industry Analysis
- Model real-world marketplace scenarios
- Test pricing strategies before deployment
- Understand supply/demand interactions
- Optimize marketplace rules

### Algorithm Development
- Test bidding strategies under optimal conditions
- Validate pricing mechanisms
- Develop marketplace optimization techniques
- Benchmark performance

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

## Conclusion

Marrakesh provides a clean, focused framework for studying marketplace dynamics by assuming optimal pacing and isolating other variables. This design choice enables researchers to investigate pricing mechanisms, bidding strategies, and marketplace design without the confounding effects of sub-optimal pacing.

The convergence mechanism serves as a **calibration tool** rather than a research subject, ensuring campaigns operate at their optimal point before observation begins. This allows the framework to answer questions about marketplace design, pricing, and bidding strategies that are difficult to study when pacing is also a variable.

By making explicit assumptions and focusing on specific research questions, Marrakesh provides a powerful tool for understanding how digital advertising marketplaces work when pacing is solved.

---

*This document focuses on conceptual understanding. For implementation details, see the code and inline documentation.*

