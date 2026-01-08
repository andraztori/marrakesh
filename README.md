# Marrakesh

A marketplace simulation framework for studying auction dynamics, pricing mechanisms, and bidding strategies in digital advertising markets.

## Overview

Marrakesh is a research tool designed to study marketplace phenomena under the assumption that **optimal pacing is already achieved**. This design choice allows researchers to isolate and study other marketplace dynamics without the complexity of pacing optimization.

### Key Features

- **Two-party marketplace model**: Sellers (supply) and Campaigns (demand) with distinct objectives
- **Multiple pricing models**: Fixed-price and first-price auction sellers
- **Various bidding strategies**: Multiplicative pacing, optimal bidding, cheater/last look, and Median Bidding (also known as ALB - Auction Level Bid)
- **Convergence framework**: Automatic calibration to optimal pacing and boost factors
- **Deterministic simulations**: Seeded random number generation for reproducibility
- **Comprehensive logging**: Structured logging with multiple receivers and event types
- **Scenario framework**: Structured experimentation with validation

## Quick Start

See [BUILD.md](BUILD.md) for detailed build instructions.

## Core Concepts

### Optimal Pacing Assumption

Marrakesh assumes campaigns have access to perfect pacing algorithms. This is not a limitation but a deliberate design choice that allows researchers to focus on marketplace dynamics rather than pacing optimization.

The simulation is run multiple times and the convergence mechanism is used to converge to one single pacing constant for each participant (buy or sell side). The convergence mechanism in Marrakesh is a **simulation setup tool**, not a pacing algorithm to be studied.

### Marketplace Model

**Sellers (Supply Side)**:
- Offer impressions with different pricing models (fixed price or first-price auction)
- Can use boost factors to influence bid values
- May converge boost factors to meet revenue targets

**Campaigns (Demand Side)**:
- Have objectives (impression targets, budget constraints, or average value targets)
- Use different bidding strategies
- Compete for impressions through auctions
- Operate under optimal pacing (assumed)
- Can converge on multiple targets simultaneously (e.g., impressions and average value)

### Bidding Strategies

1. **Multiplicative Pacing**: Simple bid calculation `bid = pacing × value × seller_boost_factor`
2. **Optimal Bidding**: Uses sigmoid functions to model win probability and finds optimal bids based on marginal utility of spend. This is sometimes called "Max Margin Bidding", however we are not in a regime of billing the shadow price, so this is just an optimization method to go from shadow price to actual bid.
3. **Cheater/Last Look**: Strategic bidding that exploits competition knowledge by bidding just above the competition
4. **Median Bidding** (ALB): Bids at the predicted offset point if the pacing bid exceeds it, otherwise doesn't bid

### Bid Determination and Margins

The framework supports different approaches to determining net and gross bid values:
- **No Margin**: Net bid equals gross bid (no margin applied)
- **Fixed Margin (Optimal)**: Gross bid is adjusted to achieve target margin: `gross_bid = optimized_bid / (1 - margin)`
- **Fixed Margin (Unoptimal)**: Net bid is reduced to achieve target margin: `net_bid = optimized_bid × (1 - margin)`

The `basic_margin` scenario demonstrates these approaches and validates that optimal margin application yields better value while maintaining the same publisher payout and advertiser charge.

### Convergence

The system uses iterative feedback loops to find optimal pacing and boost factors:
- Campaigns converge control variables (pacing multipliers) to meet impression, budget, or average value targets
- Sellers converge boost factors to balance supply costs with target costs
- Convergence ensures campaigns and sellers operate optimally before observation
- Uses proportional controllers for smooth adjustments and constant controllers for fixed values

## Usage Examples

### Running Scenarios

```bash
# Run a single scenario
cargo run --release scarcity_and_abundance

# Run a scenario multiple times with different seeds
cargo run --release scarcity_and_abundance 10

# Run all scenarios
cargo run --release all

# Run all scenarios multiple times
cargo run --release all 5
```

### Verbose Logging

```bash
# Enable verbose auction logging
cargo run --release basic_bidding_strategies --verbose auction
```

## Output

### Log Files

Simulation logs are organized in the `log/` directory:
- `log/<scenario_name>/scenario.log` - Scenario-level summaries
- `log/<scenario_name>/iterations-<variant>.log` - Per-iteration data
- `log/<scenario_name>/variant-<variant>.log` - Final variant results
- `log/<scenario_name>/auctions-<variant>-iter<iteration>.csv` - Detailed auction data
- `log/summary.log` - Validation summary across all scenarios

## Documentation

- **[BUILD.md](BUILD.md)** - Detailed build instructions and troubleshooting
- **[architecture.md](architecture.md)** - Comprehensive architecture and design documentation

## Research Capabilities

With optimal pacing assumed, researchers can study:

- **Pricing Mechanisms**: Fixed-price vs. auction-based sellers
- **Bidding Strategies**: Comparison of different bidding approaches (multiplicative, optimal, cheater, median bidding)
- **Margin Application**: Study of optimal vs. unoptimal margin application methods and their impact on value capture
- **Marketplace Design**: Impact of floors, competition thresholds, and rules
- **Supply and Demand Dynamics**: How supply composition affects outcomes
- **Value Distribution**: How value is distributed between parties
- **Convergence Strategies**: Fixed vs. dynamic boost factors for sellers
