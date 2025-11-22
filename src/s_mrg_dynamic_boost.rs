/// In this scenario we compare two variants:
///
/// - One with unprofitable MRG seller due to too much HB supply bringing prices below supply
///   guaranteed prices
///
/// - Second one is where MRG seller dynamically adjusts boost parameter to exactly balance out
///   the market so supply cost equals demand cost

use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::ImpressionsParam;
use crate::competition::{CompetitionGeneratorLogNormal, CompetitionGeneratorNone};
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "MRGdynamicboost",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_variant(dynamic_boost: bool) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: 1000 },
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 },
    );

    // Add two sellers (IDs are automatically set to match Vec index)
    // First seller (MRG) type depends on dynamic_boost parameter
    let fixed_cost_cpm = 10.0;
    let impressions_on_offer = 1000;
    sellers.add(
        "MRG".to_string(),  // seller_name
        SellerType::FIXED_PRICE {
            fixed_cost_cpm,
        },  // seller_type
        if dynamic_boost {
            // Converge when cost of impressions matches virtual price
            // fixed_cost_cpm is in CPM (cost per 1000 impressions), so divide by 1000 to get cost per impression
            let target_total_cost = (impressions_on_offer as f64) * fixed_cost_cpm / 1000.0;
            SellerConvergeStrategy::TOTAL_COST { target_total_cost }
        } else {
            SellerConvergeStrategy::NONE { default_value: 1.0 }
        },  // seller_converge
        impressions_on_offer,  // impressions_on_offer
        CompetitionGeneratorNone::new(),  // competition_generator
        floors::FloorGeneratorFixed::new(0.0),  // floor_generator
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        10000,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
        floors::FloorGeneratorLogNormal::new(0.2, 3.0),  // floor_generator
    );

    // Create impressions parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
    );

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::Standard);

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}


/// Scenario demonstrating the effect of MRG seller boost factor on marketplace dynamics
/// 
/// This scenario compares the abundant HB variant (1000 HB impressions) with and without
/// a boost factor of 2.0 applied to the MRG seller. The boost factor affects how MRG
/// impressions are valued in the marketplace.
pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant with fixed boost (no convergence) for MRG seller
    let simulation_converge_a = prepare_variant(false);
    let stats_a = simulation_converge_a.run_variant("Running with Abundant HB impressions", scenario_name, "no_boost", 100, logger);
    
    // Run variant with dynamic boost (convergence) for MRG seller
    let simulation_converge_b = prepare_variant(true);
    let stats_b = simulation_converge_b.run_variant("Running with Abundant HB impressions (MRG Dynamic boost)", scenario_name, "dynamic_boost", 100, logger);
    
    // Validate expected marketplace behavior
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A (no boost) - seller 0 should not be profitable (supply_cost > virtual_cost)
    let msg = format!(
        "Variant A (no boost) - Seller 0 (MRG) is not profitable (supply_cost > virtual_cost): {:.2} > {:.2}",
        stats_a.seller_stats[0].total_supply_cost,
        stats_a.seller_stats[0].total_virtual_cost
    );
    if stats_a.seller_stats[0].total_supply_cost > stats_a.seller_stats[0].total_virtual_cost {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant B (dynamic boost) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost = stats_b.overall_stat.total_supply_cost;
    let virtual_cost = stats_b.overall_stat.total_virtual_cost;
    let diff = (supply_cost - virtual_cost).abs();
    let max_diff = supply_cost.max(virtual_cost) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant B (dynamic boost) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost, virtual_cost, diff, max_diff
    );
    if diff <= max_diff {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant B (dynamic boost) total supply cost should be lower than variant A (no boost)
    let msg = format!(
        "Variant B (dynamic boost) total supply cost is lower than variant A (no boost): {:.2} < {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_supply_cost
    );
    if stats_b.overall_stat.total_supply_cost < stats_a.overall_stat.total_supply_cost {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}
