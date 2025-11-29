/// In this scenario we compare three variants:
///
/// - One with unprofitable MRG seller due to too much HB supply bringing prices below supply
///   guaranteed prices (using MULTIPLICATIVE_PACING)
///
/// - Second one is where MRG seller dynamically adjusts boost parameter to exactly balance out
///   the market so supply cost equals demand cost (using MULTIPLICATIVE_PACING)
///   It uses simple value function of campaign_value * pacing * supply_boost_factor
/// 
/// - Third one uses MULTIPLICATIVE_ADDITIVE bidding strategy with dynamic boost. 
///   We show that additive is sub-optimal in terms of value-to-cost ratio
///   It uses value function of campaign_value * pacing + supply_boost_factor

use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::Sellers;
use crate::seller::{SellerGeneral, SellerTrait};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::ImpressionsParam;
use crate::competition::{CompetitionGeneratorLogNormal, CompetitionGeneratorNone};
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;
use crate::seller_targets::{SellerTargetNone, SellerTargetTotalCost};
use crate::seller_chargers::{SellerChargerFirstPrice, SellerChargerFixedPrice};

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "supply_controlled_boost",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_variant(dynamic_boost: bool, campaign_type: CampaignType) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Check if this is MULTIPLICATIVE_ADDITIVE variant before campaign_type is moved
    let is_multiplicative_additive = campaign_type == CampaignType::MULTIPLICATIVE_ADDITIVE;

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        campaign_type.clone(),
        vec![ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: 1000 }],
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        campaign_type,
        vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 }],
    );

    // Add two sellers (IDs are automatically set to match Vec index via add_advanced)
    // First seller (MRG) type depends on dynamic_boost parameter
    let fixed_cost_cpm = 10.0;
    let impressions_on_offer_mrg = 1000;
    
    // Create converge_target and converge_controller for MRG seller
    let (converge_target_mrg, converge_controller_mrg): (Box<dyn crate::seller_targets::SellerTargetTrait>, Box<dyn crate::controllers::ControllerTrait>) = if dynamic_boost {
        // Converge when cost of impressions matches virtual price
        // fixed_cost_cpm is in CPM (cost per 1000 impressions), so divide by 1000 to get cost per impression
        let target_total_cost = (impressions_on_offer_mrg as f64) * fixed_cost_cpm / 1000.0;
        let controller = if is_multiplicative_additive {
            // Use advanced controller setup for MULTIPLICATIVE_ADDITIVE variant
            // This is needed due to additive bidding strategy for supply requiring larger adjustments to converge
            crate::controllers::ControllerProportionalDerivative::new_advanced(
                0.005, // tolerance_fraction
                0.5,   // max_adjustment_factor
                1.0,   // proportional_gain
                0.5,   // derivative_gain (half of proportional_gain)
            )
        } else {
            crate::controllers::ControllerProportionalDerivative::new()
        };
        (
            Box::new(SellerTargetTotalCost {
                target_cost: target_total_cost,
            }),
            Box::new(controller),
        )
    } else {
        (
            Box::new(SellerTargetNone),
            Box::new(crate::controllers::ControllerConstant::new(1.0)),
        )
    };
    
    // Create MRG seller
    let seller_mrg: Box<dyn SellerTrait> = Box::new(SellerGeneral {
        seller_id: 0,  // Will be set by add_advanced
        seller_name: "MRG".to_string(),
        impressions_on_offer: impressions_on_offer_mrg,
        converge_targets: vec![converge_target_mrg],
        converge_controllers: vec![converge_controller_mrg],
        competition_generator: CompetitionGeneratorNone::new(),
        floor_generator: floors::FloorGeneratorFixed::new(0.0),
        seller_charger: Box::new(SellerChargerFixedPrice {
            fixed_cost_cpm,
        }),
    });
    sellers.add_advanced(seller_mrg);

    // Create HB seller
    let seller_hb: Box<dyn SellerTrait> = Box::new(SellerGeneral {
        seller_id: 0,  // Will be set by add_advanced
        seller_name: "HB".to_string(),
        impressions_on_offer: 10000,
        converge_targets: vec![Box::new(SellerTargetNone)],
        converge_controllers: vec![Box::new(crate::controllers::ControllerConstant::new(1.0))],
        competition_generator: CompetitionGeneratorLogNormal::new(10.0),
        floor_generator: floors::FloorGeneratorLogNormal::new(0.2, 3.0),
        seller_charger: Box::new(SellerChargerFirstPrice),
    });
    sellers.add_advanced(seller_hb);

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
    // Run variant A with fixed boost (no convergence) for MRG seller, using MULTIPLICATIVE_PACING
    let simulation_converge_a = prepare_variant(false, CampaignType::MULTIPLICATIVE_PACING);
    let stats_a = simulation_converge_a.run_variant("Running with Abundant HB impressions (Multiplicative)", scenario_name, "no_boost", 100, logger);
    
    // Run variant B with dynamic boost (convergence) for MRG seller, using MULTIPLICATIVE_PACING
    let simulation_converge_b = prepare_variant(true, CampaignType::MULTIPLICATIVE_PACING);
    let stats_b = simulation_converge_b.run_variant("Running with Abundant HB impressions (MRG Dynamic boost, Multiplicative)", scenario_name, "dynamic_boost", 100, logger);
    
    // Run variant C with dynamic boost (convergence) for MRG seller, using MULTIPLICATIVE_ADDITIVE
    let simulation_converge_c = prepare_variant(true, CampaignType::MULTIPLICATIVE_ADDITIVE);
    let stats_c = simulation_converge_c.run_variant("Running with Abundant HB impressions (MRG Dynamic boost, Multiplicative Additive)", scenario_name, "dynamic_boost_additive", 100, logger);
    
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
    
    // Check: Variant B (dynamic boost, Multiplicative) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost = stats_b.overall_stat.total_supply_cost;
    let virtual_cost = stats_b.overall_stat.total_virtual_cost;
    let diff = (supply_cost - virtual_cost).abs();
    let max_diff = supply_cost.max(virtual_cost) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant B (dynamic boost, Multiplicative) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost, virtual_cost, diff, max_diff
    );
    if diff <= max_diff {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    

    // Check: Variant C (dynamic boost with MULTIPLICATIVE_ADDITIVE) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost_c = stats_c.overall_stat.total_supply_cost;
    let virtual_cost_c = stats_c.overall_stat.total_virtual_cost;
    let diff_c = (supply_cost_c - virtual_cost_c).abs();
    let max_diff_c = supply_cost_c.max(virtual_cost_c) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant C (dynamic boost, Multiplicative Additive) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost_c, virtual_cost_c, diff_c, max_diff_c
    );
    if diff_c <= max_diff_c {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }

    // Check: Variant B (dynamic boost, Multiplicative) total supply cost should be lower than variant A (no boost)
    let msg = format!(
        "Variant B (dynamic boost, Multiplicative) total supply cost is lower than variant A (no boost): {:.2} < {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_supply_cost
    );
    if stats_b.overall_stat.total_supply_cost < stats_a.overall_stat.total_supply_cost {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    // Check: Variant B (dynamic boost, Multiplicative) should have higher value-to-cost ratio than Variant C (additive)
    let supply_cost_b = stats_b.overall_stat.total_supply_cost;
    let value_b = stats_b.overall_stat.total_value;
    let supply_cost_c_check = stats_c.overall_stat.total_supply_cost;
    let value_c = stats_c.overall_stat.total_value;
    
    if supply_cost_b > 0.0 && supply_cost_c_check > 0.0 {
        let ratio_b = value_b / supply_cost_b;
        let ratio_c = value_c / supply_cost_c_check;
        let msg = format!(
            "Variant B (dynamic boost, Multiplicative) has higher value-to-cost ratio than Variant C (additive): {:.4} > {:.4}",
            ratio_b, ratio_c
        );
        if ratio_b > ratio_c {
            logln!(logger, LogEvent::Scenario, "✓ {}", msg);
        } else {
            errors.push(msg.clone());
            errln!(logger, LogEvent::Scenario, "{}", msg);
        }
    } else {
        let msg = format!(
            "Cannot compare value-to-cost ratios: Variant B supply_cost={:.2}, Variant C supply_cost={:.2}",
            supply_cost_b, supply_cost_c_check
        );
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}
