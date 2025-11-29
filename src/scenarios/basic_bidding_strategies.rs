/// This is a simple scenario that uses first price bidding on HB supply.
///
/// It compares different bidding strategies using a fixed budget campaign.
///
/// Its five variants test different bidding approaches:
///
/// - Variant A: Multiplicative pacing (baseline)
///
/// - Variant B: Median Bidding
///
/// - Variant C: Optimal bidding (optimizes marginal utility of spend - equivalent of Max Margin Bidding)
///
/// - Variant D: Max margin bidding (optimizes expected margin)
///
/// - Variant E: Cheater bidding (has perfect information about competition)


#[allow(unused_imports)]
use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::ImpressionsParam;
use crate::competition::CompetitionGeneratorLogNormal;
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "basic_bidding_strategies",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(campaign_type: CampaignType) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign (ID is automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        campaign_type,  // campaign_type - either multiplicative pacing or optimal bidding
        vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 }],  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        10000,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
      floors::FloorGeneratorLogNormal::new(1.0, 3.0),  // floor_generator
    //   floors::FloorGeneratorFixed::new(0.0),
    );

    // Create impressions parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 2.0),   // value_to_campaign_multiplier_dist
    );

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::Standard);

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A with multiplicative pacing
    let simulation_converge_a = prepare_simulationconverge(CampaignType::MULTIPLICATIVE_PACING);
    let stats_a = simulation_converge_a.run_variant("Running with multiplicative pacing", scenario_name, "multiplicative", 100, logger)?;
    
    // Run variant B with Median Bidding
    let simulation_converge_b = prepare_simulationconverge(CampaignType::MEDIAN);
    let stats_b = simulation_converge_b.run_variant("Running with Median Bidding", scenario_name, "median", 100, logger)?;
    
    // Run variant C with optimal bidding
    let simulation_converge_c = prepare_simulationconverge(CampaignType::OPTIMAL);
    let stats_c = simulation_converge_c.run_variant("Running with optimal bidding", scenario_name, "optimal", 100, logger)?;
    
    // Run variant D with max margin bidding
    let simulation_converge_d = prepare_simulationconverge(CampaignType::MAX_MARGIN);
    let stats_d = simulation_converge_d.run_variant("Running with max margin bidding", scenario_name, "max-margin", 100, logger)?;
    
    // Run variant E with cheater bidding
    let simulation_converge_e = prepare_simulationconverge(CampaignType::CHEATER);
    let stats_e = simulation_converge_e.run_variant("Running with cheater bidding", scenario_name, "cheater", 100, logger)?;
    
    // Validate expected marketplace behavior
    // Variant A (multiplicative pacing) uses MULTIPLICATIVE_PACING with TOTAL_BUDGET
    // Variant B (Median Bidding) uses MEDIAN with TOTAL_BUDGET
    // Variant C (optimal bidding) uses OPTIMAL with TOTAL_BUDGET
    // Variant D (max margin bidding) uses MAX_MARGIN with TOTAL_BUDGET
    // Variant E (cheater bidding) uses CHEATER with TOTAL_BUDGET
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant B (Median Bidding) obtained value > Variant A (multiplicative pacing) obtained value
    // Note: This validation is true only when operating in regime of low fill rates
    let msg = format!(
        "Variant B (Median Bidding) obtained value is greater than Variant A (Multiplicative pacing): {:.2} > {:.2} (Note: true only in low fill rate regime)",
        stats_b.overall_stat.total_value,
        stats_a.overall_stat.total_value
    );
    if stats_b.overall_stat.total_value > stats_a.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant C (optimal) and Variant D (max margin) obtained values are roughly equal
    let value_diff = (stats_c.overall_stat.total_value - stats_d.overall_stat.total_value).abs();
    let avg_value = (stats_c.overall_stat.total_value + stats_d.overall_stat.total_value) / 2.0;
    let relative_diff = if avg_value > 0.0 { value_diff / avg_value } else { 0.0 };
    let tolerance = 0.1; // 10% tolerance
    let msg = format!(
        "Variant C (Optimal bidding) and Variant D (Max margin) obtained values are roughly equal: {:.2} vs {:.2} (diff: {:.2}%, tolerance: {:.0}%)",
        stats_c.overall_stat.total_value,
        stats_d.overall_stat.total_value,
        relative_diff * 100.0,
        tolerance * 100.0
    );
    if relative_diff <= tolerance {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant E (cheater) obtained value > Variant D (max margin) obtained value
    let msg = format!(
        "Variant E (Cheater) obtained value is greater than Variant D (Max margin): {:.2} > {:.2}",
        stats_e.overall_stat.total_value,
        stats_d.overall_stat.total_value
    );
    if stats_e.overall_stat.total_value > stats_d.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant E (cheater) obtained value > Variant C (optimal) obtained value
    let msg = format!(
        "Variant E (Cheater) obtained value is greater than Variant C (Optimal bidding): {:.2} > {:.2}",
        stats_e.overall_stat.total_value,
        stats_c.overall_stat.total_value
    );
    if stats_e.overall_stat.total_value > stats_c.overall_stat.total_value {
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

