/// This scenario compares Max Margin bidding with single and double targets.
///
/// It compares two bidding strategies:
///
/// - Variant A: Max margin bidding converging to 1000 impressions
///
/// - Variant B: Max margin double target bidding converging to 1000 impressions and avg value of 0.8
/// This is very much the example of needing to buy certain amount of impressions
/// while hitting 80% viewability rate
/// 
/// In order to model viewability as "value", we use beta distribution for base impression value

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
    short_name: "viewability",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(hb_impressions: usize, campaign_type: CampaignType, converge_targets: Vec<ConvergeTarget>) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign (ID is automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        campaign_type,  // campaign_type
        converge_targets,  // converge_targets
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
        utils::beta_dist(30.0, 3.0),  // base_impression_value_dist (beta distribution, values 0-1), this resembles viewability
        utils::lognormal_dist(1.0, 2.0),   // value_to_campaign_multiplier_dist
    );

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::Standard);

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    let num_impressions = 10000;
    const TARGET_IMPRESSIONS: i32 = 1000;
    const TARGET_AVG_VALUE: f64 = 0.92;
    
    // Run variant A with max margin bidding
    // Converging to TARGET_IMPRESSIONS impressions
    let simulation_converge_a = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN,
        vec![ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: TARGET_IMPRESSIONS }],
    );
    let stats_a = simulation_converge_a.run_variant(&format!("Running with max margin bidding ({} impressions)", TARGET_IMPRESSIONS), scenario_name, "max-margin-impressions", 100, logger);
    
    // Run variant B with max margin double target bidding
    // Converging to TARGET_IMPRESSIONS impressions and avg value of TARGET_AVG_VALUE
    let simulation_converge_b = prepare_simulationconverge(
        num_impressions,
        CampaignType::MAX_MARGIN_DOUBLE_TARGET,
        vec![
            ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: TARGET_IMPRESSIONS },
            ConvergeTarget::AVG_VALUE { avg_impression_value_to_campaign: TARGET_AVG_VALUE },
        ],
    );
    let stats_b = simulation_converge_b.run_variant(&format!("Running with max margin double target ({} impressions, avg value {})", TARGET_IMPRESSIONS, TARGET_AVG_VALUE), scenario_name, "max-margin-double", 1000, logger);
    
    logln!(logger, LogEvent::Scenario, "");
    
    // Validation
    let mut errors = Vec::new();
    
    // Check that both campaigns achieved roughly TARGET_IMPRESSIONS impressions
    let impressions_a = stats_a.campaign_stats[0].impressions_obtained;
    let impressions_b = stats_b.campaign_stats[0].impressions_obtained;
    let impressions_target = TARGET_IMPRESSIONS as f64;
    
    let impressions_a_diff = (impressions_a - impressions_target).abs();
    let impressions_b_diff = (impressions_b - impressions_target).abs();
    
    if impressions_a_diff < 50.0 {
        logln!(logger, LogEvent::Scenario, "✓ Variant A achieved roughly {} impressions: {:.0}", TARGET_IMPRESSIONS, impressions_a);
    } else {
        let msg = format!("Variant A did NOT achieve roughly {} impressions: {:.0}", TARGET_IMPRESSIONS, impressions_a);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    if impressions_b_diff < 50.0 {
        logln!(logger, LogEvent::Scenario, "✓ Variant B achieved roughly {} impressions: {:.0}", TARGET_IMPRESSIONS, impressions_b);
    } else {
        let msg = format!("Variant B did NOT achieve roughly {} impressions: {:.0}", TARGET_IMPRESSIONS, impressions_b);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    // Check that variant B achieved roughly TARGET_AVG_VALUE avg value
    let avg_value_b = if impressions_b > 0.0 {
        stats_b.campaign_stats[0].total_value / impressions_b
    } else {
        0.0
    };
    let avg_value_target = TARGET_AVG_VALUE;
    let avg_value_diff = (avg_value_b - avg_value_target).abs();
    
    if avg_value_diff < 0.05 {
        logln!(logger, LogEvent::Scenario, "✓ Variant B achieved roughly {} avg value: {:.4}", TARGET_AVG_VALUE, avg_value_b);
    } else {
        let msg = format!("Variant B did NOT achieve roughly {} avg value: {:.4}", TARGET_AVG_VALUE, avg_value_b);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}

