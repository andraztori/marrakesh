/// In this scenario we compare three variants:
///
/// - MAX_MARGIN: Uses max margin bidding with multiplicative supply boost
///   full_price = campaign_control_factor * seller_control_factor * value_to_campaign
///
/// - MAX_MARGIN_ADDITIVE_SUPPLY: Uses max margin bidding with additive supply boost
///   full_price = campaign_control_factor * value_to_campaign + seller_control_factor
///
/// - MAX_MARGIN_EXPONENTIAL_SUPPLY: Uses max margin bidding with exponential supply boost
///   full_price = (campaign_control_factor * value_to_campaign) ^ seller_control_factor
/// 
/// All variants use dynamic boost for MRG seller and competition data for both sellers.

use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::Sellers;
use crate::seller::{SellerGeneral, SellerTrait};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::ImpressionsParam;
use crate::competition::CompetitionGeneratorLogNormal;
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;
use crate::seller_targets::{SellerTargetNone, SellerTargetTotalCost};
use crate::seller_chargers::{SellerChargerFirstPrice, SellerChargerFixedPrice};

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "supply_controlled_boost_2",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_variant(campaign_type: CampaignType) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Check if this is MAX_MARGIN_ADDITIVE_SUPPLY variant before campaign_type is moved
    let is_additive_supply = campaign_type == CampaignType::MAX_MARGIN_ADDITIVE_SUPPLY;

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
    
    // Create converge_target and converge_controller for MRG seller (always dynamic boost)
    // Converge when cost of impressions matches virtual price
    // fixed_cost_cpm is in CPM (cost per 1000 impressions), so divide by 1000 to get cost per impression
    let target_total_cost = (impressions_on_offer_mrg as f64) * fixed_cost_cpm / 1000.0;
    // Use aggressive controller setup for both variants to ensure faster convergence
    let controller = crate::controllers::ControllerProportionalDerivative::new_advanced(
        0.002, // tolerance_fraction
        0.3,   // max_adjustment_factor
        0.2,   // proportional_gain (aggressive: 100% of error)
        0.05,  // derivative_gain (half of proportional_gain)
        true,  // rescaling (default)
    );
    let (converge_target_mrg, converge_controller_mrg): (Box<dyn crate::seller_targets::SellerTargetTrait>, Box<dyn crate::controllers::ControllerTrait>) = (
        Box::new(SellerTargetTotalCost {
            target_cost: target_total_cost,
        }),
        Box::new(controller),
    );
    
    // Create MRG seller with competition data
    let seller_mrg: Box<dyn SellerTrait> = Box::new(SellerGeneral {
        seller_id: 0,  // Will be set by add_advanced
        seller_name: "MRG".to_string(),
        impressions_on_offer: impressions_on_offer_mrg,
        converge_targets: vec![converge_target_mrg],
        converge_controllers: vec![converge_controller_mrg],
        competition_generator: CompetitionGeneratorLogNormal::new(10.0),  // Add competition to MRG seller
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
        impressions_on_offer: 5000,
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


/// Scenario comparing MAX_MARGIN with multiplicative vs additive vs exponential supply boost
/// 
/// This scenario compares max margin bidding with:
/// - Multiplicative supply boost: full_price = campaign_control_factor * seller_control_factor * value_to_campaign
/// - Additive supply boost: full_price = campaign_control_factor * value_to_campaign + seller_control_factor
/// - Exponential supply boost: full_price = (campaign_control_factor * value_to_campaign) ^ seller_control_factor
/// 
/// All variants use dynamic boost for MRG seller and competition data for both sellers.
pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A with MAX_MARGIN (multiplicative supply boost)
    let simulation_converge_a = prepare_variant(CampaignType::MAX_MARGIN);
    let stats_a = simulation_converge_a.run_variant("Running MAX_MARGIN with multiplicative supply boost", scenario_name, "max_margin_multiplicative_supply", 100, logger)?;
    
    // Run variant B with MAX_MARGIN_ADDITIVE_SUPPLY (additive supply boost)
    let simulation_converge_b = prepare_variant(CampaignType::MAX_MARGIN_ADDITIVE_SUPPLY);
    let stats_b = simulation_converge_b.run_variant("Running MAX_MARGIN_ADDITIVE_SUPPLY with additive supply boost", scenario_name, "max_margin_additive_supply", 100, logger)?;
    
    // Run variant C with MAX_MARGIN_EXPONENTIAL_SUPPLY (exponential supply boost)
    let simulation_converge_c = prepare_variant(CampaignType::MAX_MARGIN_EXPONENTIAL_SUPPLY);
    let stats_c = simulation_converge_c.run_variant("Running MAX_MARGIN_EXPONENTIAL_SUPPLY with exponential supply boost", scenario_name, "max_margin_exponential_supply", 100, logger)?;
    
    // Validate expected marketplace behavior
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A (MAX_MARGIN) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost_a = stats_a.overall_stat.total_supply_cost;
    let virtual_cost_a = stats_a.overall_stat.total_virtual_cost;
    let diff_a = (supply_cost_a - virtual_cost_a).abs();
    let max_diff_a = supply_cost_a.max(virtual_cost_a) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant A (MAX_MARGIN) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost_a, virtual_cost_a, diff_a, max_diff_a
    );
    if diff_a <= max_diff_a {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant B (MAX_MARGIN_ADDITIVE_SUPPLY) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost_b = stats_b.overall_stat.total_supply_cost;
    let virtual_cost_b = stats_b.overall_stat.total_virtual_cost;
    let diff_b = (supply_cost_b - virtual_cost_b).abs();
    let max_diff_b = supply_cost_b.max(virtual_cost_b) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant B (MAX_MARGIN_ADDITIVE_SUPPLY) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost_b, virtual_cost_b, diff_b, max_diff_b
    );
    if diff_b <= max_diff_b {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant C (MAX_MARGIN_EXPONENTIAL_SUPPLY) - total overall supply and virtual cost should be nearly equal (max 1% off)
    let supply_cost_c = stats_c.overall_stat.total_supply_cost;
    let virtual_cost_c = stats_c.overall_stat.total_virtual_cost;
    let diff_c = (supply_cost_c - virtual_cost_c).abs();
    let max_diff_c = supply_cost_c.max(virtual_cost_c) * 0.01; // 1% of the larger value
    let msg = format!(
        "Variant C (MAX_MARGIN_EXPONENTIAL_SUPPLY) - Total overall supply and virtual cost are nearly equal (within 1%): supply={:.2}, virtual={:.2}, diff={:.2}, max_diff={:.2}",
        supply_cost_c, virtual_cost_c, diff_c, max_diff_c
    );
    if diff_c <= max_diff_c {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }

    // Check: Variant A (MAX_MARGIN) should have better value-to-cost ratio than Variant B (MAX_MARGIN_ADDITIVE_SUPPLY)
    if supply_cost_a > 0.0 && supply_cost_b > 0.0 {
        let value_a = stats_a.overall_stat.total_value;
        let value_b = stats_b.overall_stat.total_value;
        let ratio_a = value_a / supply_cost_a;
        let ratio_b = value_b / supply_cost_b;
        let msg = format!(
            "Variant A (MAX_MARGIN) has better value-to-cost ratio than Variant B (MAX_MARGIN_ADDITIVE_SUPPLY): {:.4} > {:.4}",
            ratio_a, ratio_b
        );
        if ratio_a > ratio_b {
            logln!(logger, LogEvent::Scenario, "✓ {}", msg);
        } else {
            errors.push(msg.clone());
            errln!(logger, LogEvent::Scenario, "{}", msg);
        }
    } else {
        let msg = format!(
            "Cannot compare value-to-cost ratios: Variant A supply_cost={:.2}, Variant B supply_cost={:.2}",
            supply_cost_a, supply_cost_b
        );
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }

    // Check: Variant A (MAX_MARGIN) should have higher total value than Variant B (MAX_MARGIN_ADDITIVE_SUPPLY)
    let value_a = stats_a.overall_stat.total_value;
    let value_b = stats_b.overall_stat.total_value;
    let msg = format!(
        "Variant A (MAX_MARGIN) has higher total value than Variant B (MAX_MARGIN_ADDITIVE_SUPPLY): {:.2} > {:.2}",
        value_a, value_b
    );
    if value_a > value_b {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant C (MAX_MARGIN_EXPONENTIAL_SUPPLY) - compare value-to-cost ratio with other variants
    if supply_cost_c > 0.0 {
        let value_c = stats_c.overall_stat.total_value;
        let ratio_c = value_c / supply_cost_c;
        let msg = format!(
            "Variant C (MAX_MARGIN_EXPONENTIAL_SUPPLY) value-to-cost ratio: {:.4}",
            ratio_c
        );
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}
