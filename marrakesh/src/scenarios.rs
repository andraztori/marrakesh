use std::error::Error;
use crate::logger::Logger;

/// Verbosity levels for scenario execution
/// Ordering: None < Summary < Full
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No output
    None,
    /// Print final stats for each variant, but not iterations
    Summary,
    /// Print everything including iterations
    Full,
}

/// Function type for scenario entry functions
pub type ScenarioFn = fn(verbosity: Verbosity, logger: &mut Logger) -> Result<(), Box<dyn Error>>;

/// Entry in the scenario catalog
#[derive(Clone)]
pub struct ScenarioEntry {
    pub short_name: &'static str,
    pub description: &'static str,
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
//     description: "desc",
//     run: function,
// });

