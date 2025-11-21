/// This scenario compares Optimal and Max Margin bidding converging to $20 spend, expecting equality.
/// This numerically proves that the methods are actually equivalent.
///
/// It compares two bidding strategies converging to the same budget:
///
/// - Variant B: Optimal bidding (optimizes marginal utility of spend) converging to $20 spend
///
/// - Variant D: Max margin bidding (optimizes expected margin) converging to $20 spend
/// This scenario proves that Max Margin and Optimal Bidding are equivalent when configured correctly.


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
    short_name: "maxmargin_equality",
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
        campaign_type,  // campaign_type
        ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 },  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
        //floors::FloorGeneratorFixed::new(0.0),
        floors::FloorGeneratorLogNormal::new(1.0, 3.0),
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
    let num_impressions = 10000;
    
    // Run variant B with optimal bidding
    // Converging to $20 spend
    let simulation_converge_b = prepare_simulationconverge(
        num_impressions,
        CampaignType::OPTIMAL,
    );
    let stats_b = simulation_converge_b.run_variant("Running with optimal bidding", scenario_name, "optimal", 100, logger);
    
    // Run variant D with max margin bidding
    // Converging to $20 spend
    let simulation_converge_d = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN,
    );
    let stats_d = simulation_converge_d.run_variant("Running with max margin bidding", scenario_name, "max-margin", 100, logger);
    
    logln!(logger, LogEvent::Scenario, "");
    
    // Validation
    let mut errors = Vec::new();
    
    // Check spend equality
    let spend_diff = (stats_b.overall_stat.total_buyer_charge - stats_d.overall_stat.total_buyer_charge).abs();
    let spend_avg = (stats_b.overall_stat.total_buyer_charge + stats_d.overall_stat.total_buyer_charge) / 2.0;
    let spend_diff_pct = if spend_avg > 0.0 { spend_diff / spend_avg * 100.0 } else { 0.0 };
    
    if spend_diff_pct < 1.0 {
        logln!(logger, LogEvent::Scenario, "✓ Spend is roughly equal ({:.2}% diff)", spend_diff_pct);
    } else {
        let msg = format!("Spend is NOT roughly equal ({:.2}% diff). Optimal: {:.2}, MaxMargin: {:.2}", spend_diff_pct, stats_b.overall_stat.total_buyer_charge, stats_d.overall_stat.total_buyer_charge);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    // Check value equality
    let value_diff = (stats_b.overall_stat.total_value - stats_d.overall_stat.total_value).abs();
    let value_avg = (stats_b.overall_stat.total_value + stats_d.overall_stat.total_value) / 2.0;
    let value_diff_pct = if value_avg > 0.0 { value_diff / value_avg * 100.0 } else { 0.0 };
    
    if value_diff_pct < 1.0 {
        logln!(logger, LogEvent::Scenario, "✓ Value is roughly equal ({:.2}% diff)", value_diff_pct);
    } else {
        let msg = format!("Value is NOT roughly equal ({:.2}% diff). Optimal: {:.2}, MaxMargin: {:.2}", value_diff_pct, stats_b.overall_stat.total_value, stats_d.overall_stat.total_value);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}
