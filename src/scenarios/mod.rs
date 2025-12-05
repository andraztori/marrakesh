use std::error::Error;
use crate::logger::Logger;

/// Function type for scenario entry functions
pub type ScenarioFn = fn(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn Error>>;

/// Entry in the scenario catalog
#[derive(Clone)]
pub struct ScenarioEntry {
    pub short_name: &'static str,
    pub run: ScenarioFn,
}

// Create an inventory collection for scenario entries
inventory::collect!(ScenarioEntry);

/// Get all registered scenarios from the catalog
pub fn get_scenario_catalog() -> Vec<ScenarioEntry> {
    inventory::iter::<ScenarioEntry>
        .into_iter()
        .map(|entry| entry.clone())
        .collect()
}

// Users can register scenarios directly using inventory::submit!
// Example:
// inventory::submit!(scenarios::ScenarioEntry {
//     short_name: "name",
//     run: function,
// });

// Scenario modules
pub mod scarcity_and_abundance;
pub mod basic_bidding_strategies;
pub mod supply_simple_boost;
pub mod supply_controlled_boost;
pub mod supply_controlled_boost_2;
pub mod median_bidder;
pub mod viewability;
pub mod value_groups;

