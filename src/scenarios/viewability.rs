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
use crate::campaigns::{Campaigns, CampaignGeneral};
use crate::campaign::CampaignTrait;
use crate::campaign_bidders_single::BidderMaxMargin;
use crate::campaign_bidders_double::CampaignBidderDouble;
use crate::campaign_targets::{CampaignTargetTotalImpressions, CampaignTargetAvgValue};
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
fn prepare_simulationconverge(hb_impressions: usize, campaign: Box<dyn CampaignTrait>) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign using add_advanced (ID is automatically set to match Vec index)
    campaigns.add_advanced(campaign);

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
        utils::lognormal_dist(1.0, 0.01),   // value_to_campaign_multiplier_dist
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
    let campaign_a: Box<dyn CampaignTrait> = Box::new(CampaignGeneral {
        campaign_id: 0, // Will be set by add_advanced
        campaign_name: "C0".to_string(),
        converge_targets: vec![Box::new(CampaignTargetTotalImpressions { total_impressions_target: TARGET_IMPRESSIONS })],
        converge_controllers: vec![Box::new(crate::controllers::ControllerProportionalDerivative::new())],
        bidder: Box::new(BidderMaxMargin),
    });
    let simulation_converge_a = prepare_simulationconverge(num_impressions, campaign_a);
    let stats_a = simulation_converge_a.run_variant(&format!("Running with max margin bidding ({} impressions)", TARGET_IMPRESSIONS), scenario_name, "max-margin-impressions", 100, logger)?;
    
    // Run variant B with max margin double target bidding
    // Converging to TARGET_IMPRESSIONS impressions and avg value of TARGET_AVG_VALUE
    let campaign_b: Box<dyn CampaignTrait> = Box::new(CampaignGeneral {
        campaign_id: 0, // Will be set by add_advanced
        campaign_name: "C0".to_string(),
        converge_targets: vec![
            Box::new(CampaignTargetTotalImpressions { total_impressions_target: TARGET_IMPRESSIONS }), 
            Box::new(CampaignTargetAvgValue { avg_impression_value_to_campaign: TARGET_AVG_VALUE })
        ],
         converge_controllers: vec![
             Box::new(crate::controllers::ControllerProportionalDerivative::new()), 
             Box::new(crate::controllers::ControllerProportionalDerivative::new_advanced(
                 0.005,  // tolerance_fraction
                 0.5,  // max_adjustment_factor
                 0.35, // proportional_gain
                 0.35, // derivative_gain
                 true,  // rescaling
             ))],
        bidder: Box::new(CampaignBidderDouble),
    });
    let simulation_converge_b = prepare_simulationconverge(num_impressions, campaign_b);
    let stats_b = simulation_converge_b.run_variant(&format!("Running with max margin double target ({} impressions, avg value {})", TARGET_IMPRESSIONS, TARGET_AVG_VALUE), scenario_name, "max-margin-double", 1000, logger)?;
    
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
    
    // Check that variant A had lower avg value than variant B
    let avg_value_a = if impressions_a > 0.0 {
        stats_a.campaign_stats[0].total_value / impressions_a
    } else {
        0.0
    };
    
    if avg_value_a < avg_value_b {
        logln!(logger, LogEvent::Scenario, "✓ Variant A had lower avg value ({:.4}) than variant B ({:.4})", avg_value_a, avg_value_b);
    } else {
        let msg = format!("Variant A did NOT have lower avg value than variant B: A={:.4}, B={:.4}", avg_value_a, avg_value_b);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    // Check that variant A spend was lower than variant B spend
    let spend_a = stats_a.campaign_stats[0].total_buyer_charge;
    let spend_b = stats_b.campaign_stats[0].total_buyer_charge;
    
    if spend_a < spend_b {
        logln!(logger, LogEvent::Scenario, "✓ Variant A had lower spend ({:.2}) than variant B ({:.2})", spend_a, spend_b);
    } else {
        let msg = format!("Variant A did NOT have lower spend than variant B: A={:.2}, B={:.2}", spend_a, spend_b);
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
        errors.push(msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}

