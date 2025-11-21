# Build Instructions

This document describes how to build and run the Marrakesh marketplace simulation framework.

## Prerequisites

### Required

- **Rust toolchain** (version 1.70.0 or later)
  - Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify installation: `rustc --version`

## Building

### Standard Build

Build the project in debug mode:

```bash
cargo build
```

The resulting binary will be located at `target/debug/marrakesh`.

### Release Build

For optimized performance (recommended for simulations):

```bash
cargo build --release
```

The optimized binary will be located at `target/release/marrakesh`.


## Running

### Basic Execution

Run the simulation (defaults to running the `s_mrg_boost` scenario):

```bash
cargo run
```

Or use the release binary directly:

```bash
./target/release/marrakesh
```

### Command Line Options

The program accepts various command-line arguments:

#### Run Scenarios

Run a specific scenario:

```bash
cargo run -- <scenario_name>
```

Run a scenario multiple times with different random seeds:

```bash
cargo run -- <scenario_name> <iterations>
```

Run all scenarios:

```bash
cargo run -- all
```

Run all scenarios multiple times:

```bash
cargo run -- all <iterations>
```

#### Test Mode

Run internal test cases:

```bash
cargo run -- test
```

#### Verbose Auction Logging

Enable verbose auction logging:

```bash
cargo run -- <scenario_name> --verbose auction
```

### Available Scenarios

To see available scenarios, run an invalid scenario name:

```bash
cargo run -- invalid_scenario
```


### Performance

- Always use `--release` builds when you need performance
- The first build may take several minutes as dependencies are compiled
- Subsequent builds are incremental and much faster
