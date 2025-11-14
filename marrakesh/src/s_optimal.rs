/// This is a simple scenario that uses first price bidding on HB supply.
///
/// It uses a fixed budget campaign that converges on the marginal utility of spend.
///
/// Its two variants show different supply scenarios:
///
/// - First variant: Scarce supply (1000 HB impressions)
///
/// - Second variant: Abundant supply (10000 HB impressions)

use crate::simulationrun::Marketplace;
use crate::sellers::{SellerType, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::competition::CompetitionGeneratorParametrizedLogNormal;
use crate::floors::FloorGeneratorLogNormal;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "optimal",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(hb_impressions: usize, campaign_type: CampaignType) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign (ID is automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        campaign_type,  // campaign_type - either multiplicative pacing or optimal bidding
        ConvergeTarget::TOTAL_BUDGET { target: 20.0 },  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        hb_impressions,  // num_impressions
        CompetitionGeneratorParametrizedLogNormal::new(10.0),  // competition_generator
        FloorGeneratorLogNormal::new(0.1, 3.0),  // floor_generator
    );

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 0.7),   // value_to_campaign_multiplier_dist
    );
    let impressions = Impressions::new(&sellers, &impressions_params);

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace {
        campaigns,
        sellers,
        impressions,
    };

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A with multiplicative pacing
    let num_impressions = 10000;
    let simulation_converge_a = prepare_simulationconverge(
        num_impressions,
        CampaignType::MULTIPLICATIVE_PACING,
    );
    let stats_a = simulation_converge_a.run_variant("Running with multiplicative pacing", scenario_name, "multiplicative", 100, logger);
    
    // Run variant B with optimal bidding
    let simulation_converge_b = prepare_simulationconverge(
        num_impressions,
        CampaignType::OPTIMAL,
    );
    let stats_b = simulation_converge_b.run_variant("Running with optimal bidding", scenario_name, "optimal", 100, logger);
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (multiplicative pacing) uses MULTIPLICATIVE_PACING with TOTAL_BUDGET
    // Variant B (optimal bidding) uses OPTIMAL with TOTAL_BUDGET
    
    logln!(logger, LogEvent::Scenario, "");
    
    // Compare: Variant A (multiplicative pacing) vs Variant B (optimal bidding)
    let msg = format!(
        "Variant A (Multiplicative pacing) total buyer charge: {:.2}, Variant B (Optimal bidding) total buyer charge: {:.2}",
        stats_a.overall_stat.total_buyer_charge,
        stats_b.overall_stat.total_buyer_charge
    );
    logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    
    let msg = format!(
        "Variant A (Multiplicative pacing) total value: {:.2}, Variant B (Optimal bidding) total value: {:.2}",
        stats_a.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    
    let msg = format!(
        "Variant A (Multiplicative pacing) profitability (supply_cost vs buyer_charge): {:.2} vs {:.2}",
        stats_a.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_buyer_charge
    );
    logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    
    let msg = format!(
        "Variant B (Optimal bidding) profitability (supply_cost vs buyer_charge): {:.2} vs {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_b.overall_stat.total_buyer_charge
    );
    logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    
    Ok(())
}

