/// This is a simple scenario that uses first price bidding on HB supply.
///
/// It compares different bidding strategies using a fixed budget campaign.
///
/// Its four variants test different bidding approaches:
///
/// - Variant A: Multiplicative pacing (baseline)
///
/// - Variant B: Optimal bidding (optimizes marginal utility of spend)
///
/// - Variant C: Cheater bidding (has perfect information about competition)
///
/// - Variant D: Max margin bidding (optimizes expected margin)


#[allow(unused_imports)]
use crate::simulationrun::Marketplace;
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::competition::CompetitionGeneratorParametrizedLogNormal;
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;

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
        ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 },  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
        CompetitionGeneratorParametrizedLogNormal::new(10.0),  // competition_generator
      floors::FloorGeneratorLogNormal::new(1.0, 3.0),  // floor_generator
    //   floors::FloorGeneratorFixed::new(0.0),
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
    
    // Run variant C with cheater bidding
    let simulation_converge_c = prepare_simulationconverge(
        num_impressions,
        CampaignType::CHEATER,
    );
    let stats_c = simulation_converge_c.run_variant("Running with cheater bidding", scenario_name, "cheater", 100, logger);
    
    // Run variant D with max margin bidding
    let simulation_converge_d = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN,
    );
    let stats_d = simulation_converge_d.run_variant("Running with max margin bidding", scenario_name, "max-margin", 100, logger);
    
    // Validate expected marketplace behavior
    // Variant A (multiplicative pacing) uses MULTIPLICATIVE_PACING with TOTAL_BUDGET
    // Variant B (optimal bidding) uses OPTIMAL with TOTAL_BUDGET
    // Variant C (cheater bidding) uses CHEATER with TOTAL_BUDGET
    // Variant D (max margin bidding) uses MAX_MARGIN with TOTAL_BUDGET
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant C (cheater) obtained value > Variant B (optimal) obtained value
    let msg = format!(
        "Variant C (Cheater) obtained value is greater than Variant B (Optimal bidding): {:.2} > {:.2}",
        stats_c.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    if stats_c.overall_stat.total_value > stats_b.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B (optimal) obtained value > Variant A (multiplicative pacing) obtained value
    let msg = format!(
        "Variant B (Optimal bidding) obtained value is greater than Variant A (Multiplicative pacing): {:.2} > {:.2}",
        stats_b.overall_stat.total_value,
        stats_a.overall_stat.total_value
    );
    if stats_b.overall_stat.total_value > stats_a.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant C (cheater) obtained value > Variant D (max margin) obtained value
    let msg = format!(
        "Variant C (Cheater) obtained value is greater than Variant D (Max margin): {:.2} > {:.2}",
        stats_c.overall_stat.total_value,
        stats_d.overall_stat.total_value
    );
    if stats_c.overall_stat.total_value > stats_d.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant D (max margin) obtained value > Variant A (multiplicative pacing) obtained value
    let msg = format!(
        "Variant D (Max margin) obtained value is greater than Variant A (Multiplicative pacing): {:.2} > {:.2}",
        stats_d.overall_stat.total_value,
        stats_a.overall_stat.total_value
    );
    if stats_d.overall_stat.total_value > stats_a.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}

