/// This is an experimental scenario comparing Optimal and Max Margin bidding with fixed pacing.
///
/// It compares two bidding strategies using fixed pacing factors:
///
/// - Variant B: Optimal bidding (optimizes marginal utility of spend) with pacing = 1/0.8298
///
/// - Variant D: Max margin bidding (optimizes expected margin) with pacing = 0.8298

use crate::simulationrun::Marketplace;
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::competition::CompetitionGeneratorParametrizedLogNormal;
use crate::floors::{FloorGeneratorFixed, FloorGeneratorLogNormal};
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "experiment",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(hb_impressions: usize, campaign_type: CampaignType, pacing_factor: f64) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign (ID is automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        campaign_type,  // campaign_type
        ConvergeTarget::NONE { default_pacing: pacing_factor },  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
        CompetitionGeneratorParametrizedLogNormal::new(10.0),  // competition_generator
        FloorGeneratorFixed::new(0.0),
    );

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 2.0),   // value_to_campaign_multiplier_dist
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
    let num_impressions = 10000;
    
    // Run variant B with optimal bidding
    // Pacing factor: 1 / 0.8298
    let simulation_converge_b = prepare_simulationconverge(
        num_impressions,
        CampaignType::OPTIMAL,
        0.8298,
    );
    let stats_b = simulation_converge_b.run_variant("Running with optimal bidding", scenario_name, "optimal", 100, logger);
    
    // Run variant D with max margin bidding
    // Pacing factor: 0.8298
    let simulation_converge_d = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN,
        0.8298,
    );
    let stats_d = simulation_converge_d.run_variant("Running with max margin bidding", scenario_name, "max-margin", 100, logger);
    
    logln!(logger, LogEvent::Scenario, "");
    
    // Compare results
    logln!(logger, LogEvent::Scenario, "Comparison:");
    logln!(logger, LogEvent::Scenario, "Optimal Bidding Total Value: {:.2}", stats_b.overall_stat.total_value);
    logln!(logger, LogEvent::Scenario, "Max Margin Bidding Total Value: {:.2}", stats_d.overall_stat.total_value);
    
    Ok(())
}
