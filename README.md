# Marrakesh

A marketplace simulation framework for studying auction dynamics, pricing mechanisms, and bidding strategies in digital advertising markets.

## Overview

Marrakesh is a research tool designed to study marketplace phenomena under the assumption that **optimal pacing is already achieved**. This design choice allows researchers to isolate and study other marketplace dynamics without the complexity of pacing optimization.

### Key Features

- **Two-party marketplace model**: Sellers (supply) and Campaigns (demand) with distinct objectives
- **Multiple pricing models**: Fixed-price and first-price auction sellers
- **Various bidding strategies**: Multiplicative pacing, optimal bidding, max margin, and strategic bidding
- **Convergence framework**: Automatic calibration to optimal pacing and boost factors
- **Deterministic simulations**: Seeded random number generation for reproducibility
- **Comprehensive logging**: Structured logging with multiple receivers and event types
- **Scenario framework**: Structured experimentation with validation

## Quick Start

See [BUILD.md](BUILD.md) for detailed build instructions.

## Core Concepts

### Optimal Pacing Assumption

Marrakesh assumes campaigns have access to perfect pacing algorithms. This is not a limitation but a deliberate design choice that allows researchers to Focus on marketplace dynamics rather than pacing optimization

The simulation is run multiple times and the convergence mechanism is used to come for one single pacing constant for each participant (buy or sell side). The convergence mechanism in Marrakesh is a **simulation setup tool**, not a pacing algorithm to be studied.

### Marketplace Model

**Sellers (Supply Side)**:
- Offer impressions with different pricing models (fixed price or first-price auction)
- Can use boost factors to influence bid values
- May converge boost factors to meet revenue targets

**Campaigns (Demand Side)**:
- Have objectives (impression targets or budget constraints)
- Use different bidding strategies
- Compete for impressions through auctions
- Operate under optimal pacing (assumed)

### Bidding Strategies

1. **Multiplicative Pacing**: Simple bid calculation `bid = pacing × value × boost`
2. **Optimal Bidding**: Uses sigmoid functions to model win probability and find optimal bids
3. **Max Margin**: Maximizes expected margin `P(win) × (value - bid)`
4. **Cheater/Last Look**: Strategic bidding that exploits competition knowledge

### Convergence

The system uses iterative feedback loops to find optimal pacing and boost factors:
- Campaigns converge pacing multipliers to meet impression or budget targets
- Sellers converge boost factors to balance supply costs with target costs
- Convergence ensures campaigns and sellers operate optimally before observation

## Usage Examples

### Running Scenarios

```bash
# Run a single scenario
cargo run --release HBabundance

# Run a scenario multiple times with different seeds
cargo run --release HBabundance 10

# Run all scenarios
cargo run --release all

# Run all scenarios multiple times
cargo run --release all 5
```

### Verbose Logging

```bash
# Enable verbose auction logging
cargo run --release various --verbose auction
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
- **Bidding Strategies**: Comparison of different bidding approaches
- **Marketplace Design**: Impact of floors, competition thresholds, and rules
- **Supply and Demand Dynamics**: How supply composition affects outcomes
- **Value Distribution**: How value is distributed between parties

## Project Structure

```
marrakesh/
├── src/                    # Source code
│   ├── main.rs            # Entry point and CLI
│   ├── campaigns.rs       # Campaign types and bidding strategies
│   ├── sellers.rs         # Seller types and pricing models
│   ├── impressions.rs     # Impression generation and auctions
│   ├── converge.rs         # Convergence framework
│   ├── controllers.rs      # Controller implementations
│   ├── scenarios.rs       # Scenario framework
│   ├── logger.rs          # Structured logging
│   ├── charts.rs          # Visualization
│   └── s_*.rs             # Scenario implementations
├── toys/                  # Experimental scripts and tools
├── Cargo.toml            # Project configuration
└── architecture.md       # Architecture documentation
```


