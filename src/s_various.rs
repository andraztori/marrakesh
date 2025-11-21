/// This is a simple scenario that uses first price bidding on HB supply.
///
/// It compares different bidding strategies using a fixed budget campaign.
///
/// Its four variants test different bidding approaches:
///
/// - Variant A: Cheater bidding (has perfect information about competition)
///
/// - Variant B: Max margin bidding (optimizes expected margin)
///
/// - Variant C: Optimal bidding (optimizes marginal utility of spend)
///
/// - Variant D: Multiplicative pacing (baseline)
///
/// - Variant E: ALB (Auction Level Bid) bidding


#[allow(unused_imports)]
use crate::simulationrun::Marketplace;
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
    short_name: "various",
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
        ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 },  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
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
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params);

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A with cheater bidding
    let num_impressions = 10000;
    let simulation_converge_a = prepare_simulationconverge(
        num_impressions,
        CampaignType::CHEATER,
    );
    let stats_a = simulation_converge_a.run_variant("Running with cheater bidding", scenario_name, "cheater", 100, logger);
    
    // Run variant B with max margin bidding
    let simulation_converge_b = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN,
    );
    let stats_b = simulation_converge_b.run_variant("Running with max margin bidding", scenario_name, "max-margin", 100, logger);
    
    // Run variant C with optimal bidding
    let simulation_converge_c = prepare_simulationconverge(
        num_impressions,
        CampaignType::OPTIMAL,
    );
    let stats_c = simulation_converge_c.run_variant("Running with optimal bidding", scenario_name, "optimal", 100, logger);
    
    // Run variant D with ALB bidding
    let simulation_converge_d = prepare_simulationconverge(
        num_impressions,
        CampaignType::ALB,
    );
    let stats_d = simulation_converge_d.run_variant("Running with ALB bidding", scenario_name, "alb", 100, logger);
    
    // Run variant E with multiplicative pacing
    let simulation_converge_e = prepare_simulationconverge(
        num_impressions,
        CampaignType::MULTIPLICATIVE_PACING,
    );
    let stats_e = simulation_converge_e.run_variant("Running with multiplicative pacing", scenario_name, "multiplicative", 100, logger);
    
    // Validate expected marketplace behavior
    // Variant A (cheater bidding) uses CHEATER with TOTAL_BUDGET
    // Variant B (max margin bidding) uses MAX_MARGIN with TOTAL_BUDGET
    // Variant C (optimal bidding) uses OPTIMAL with TOTAL_BUDGET
    // Variant D (multiplicative pacing) uses MULTIPLICATIVE_PACING with TOTAL_BUDGET
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A (cheater) obtained value > Variant B (max margin) obtained value
    let msg = format!(
        "Variant A (Cheater) obtained value is greater than Variant B (Max margin): {:.2} > {:.2}",
        stats_a.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    if stats_a.overall_stat.total_value > stats_b.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant A (cheater) obtained value > Variant C (optimal) obtained value
    let msg = format!(
        "Variant A (Cheater) obtained value is greater than Variant C (Optimal bidding): {:.2} > {:.2}",
        stats_a.overall_stat.total_value,
        stats_c.overall_stat.total_value
    );
    if stats_a.overall_stat.total_value > stats_c.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B (max margin) and Variant C (optimal) obtained values are roughly equal
    let value_diff = (stats_b.overall_stat.total_value - stats_c.overall_stat.total_value).abs();
    let avg_value = (stats_b.overall_stat.total_value + stats_c.overall_stat.total_value) / 2.0;
    let relative_diff = if avg_value > 0.0 { value_diff / avg_value } else { 0.0 };
    let tolerance = 0.1; // 10% tolerance
    let msg = format!(
        "Variant B (Max margin) and Variant C (Optimal bidding) obtained values are roughly equal: {:.2} vs {:.2} (diff: {:.2}%, tolerance: {:.0}%)",
        stats_b.overall_stat.total_value,
        stats_c.overall_stat.total_value,
        relative_diff * 100.0,
        tolerance * 100.0
    );
    if relative_diff <= tolerance {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant D (ALB) obtained value > Variant E (multiplicative pacing) obtained value
    // Note: This validation is true only when operating in regime of low fill rates
    let msg = format!(
        "Variant D (ALB bidding) obtained value is greater than Variant E (Multiplicative pacing): {:.2} > {:.2} (Note: true only in low fill rate regime)",
        stats_d.overall_stat.total_value,
        stats_e.overall_stat.total_value
    );
    if stats_d.overall_stat.total_value > stats_e.overall_stat.total_value {
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

