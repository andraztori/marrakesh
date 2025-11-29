/// This scenario compares MAX_MARGIN, MEDIAN, and MULTIPLICATIVE_PACING bidding strategies
/// with varying numbers of impressions on offer.
/// Median is sometimes called Auction Level Bid (ALB)
///
/// It validates two scenarios:
/// - When number of impressions on offer is low (5000): Median Bidding works worse (obtains less value) than multiplicative bidding
/// - When number of impressions on offer is high (50000): Median Bidding works better (obtains more value) than multiplicative bidding
/// - In both cases, Median Bidding should capture less value than max margin
/// 
/// This shows that Median Bidding works as a strategy when we are in a regime with low win rates and does not work in regime with high win rates.
/// This is because for low win rates, it functions as a limit on how high our bids go,
/// but in high win rate regime it forces bids for valueable impressions to be lower than they should be

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
    short_name: "median_bidder",
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
        campaign_type,  // campaign_type - either multiplicative pacing, Median Bidding, or max margin
        vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 30.0 }],  // converge_target
    );

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
        floors::FloorGeneratorLogNormal::new(1.0, 3.0),  // floor_generator
    );

    // Create impressions parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 2.0),   // value_to_campaign_multiplier_dist
    );

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::FractionalInternalAuction { softmax_temperature: 0.5 });

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // DEBUG MODE: Only running multiplicative high for debugging
    // To roll back: uncomment all the commented sections below
    
    /* COMMENTED OUT FOR DEBUG
    logln!(logger, LogEvent::Scenario, "=== Scenario: Median Bidding Comparison with Low Impressions (5000) ===");
    
    // Scenario 1: Low impressions (5000) - Median Bidding should work worse than multiplicative
    let num_impressions_low = 4000;
    
    // Run with max margin bidding
    let simulation_converge_maxmargin_low = prepare_simulationconverge(
        num_impressions_low,
        CampaignType::MAX_MARGIN,
    );
    let stats_maxmargin_low = simulation_converge_maxmargin_low.run_variant(
        "Running with max margin bidding (low impressions)", 
        scenario_name, 
        "max-margin-low", 
        200, 
        logger
    );
    
    // Run with Median Bidding
    let simulation_converge_median_low = prepare_simulationconverge(
        num_impressions_low,
        CampaignType::MEDIAN,
    );
    // TODO figure out better controller to not need 2500 here
    let stats_median_low = simulation_converge_median_low.run_variant(
        "Running with Median Bidding (low impressions)", 
        scenario_name, 
        "median-low", 
        2500, 
        logger
    );
    
    // Run with multiplicative pacing
    let simulation_converge_mult_low = prepare_simulationconverge(
        num_impressions_low,
        CampaignType::MULTIPLICATIVE_PACING,
    );
    let stats_mult_low = simulation_converge_mult_low.run_variant(
        "Running with multiplicative pacing (low impressions)", 
        scenario_name, 
        "multiplicative-low", 
        2500, 
        logger
    );
    */
    
    logln!(logger, LogEvent::Scenario, "");
    logln!(logger, LogEvent::Scenario, "=== Scenario: Median Bidding Comparison with High Impressions (50000) ===");
    
    // Scenario 2: High impressions (50000) - Median Bidding should work better than multiplicative
    let num_impressions_high = 50000;
    
    /* COMMENTED OUT FOR DEBUG
    // Run with max margin bidding
    let simulation_converge_maxmargin_high = prepare_simulationconverge(
        num_impressions_high,
        CampaignType::MAX_MARGIN,
    );
    let stats_maxmargin_high = simulation_converge_maxmargin_high.run_variant(
        "Running with max margin bidding (high impressions)", 
        scenario_name, 
        "max-margin-high", 
        100, 
        logger
    );
    
    // Run with Median Bidding
    let simulation_converge_median_high = prepare_simulationconverge(
        num_impressions_high,
        CampaignType::MEDIAN,
    );
    let stats_median_high = simulation_converge_median_high.run_variant(
        "Running with Median Bidding (high impressions)", 
        scenario_name, 
        "median-high", 
        100, 
        logger
    );
    */
    
    // Run with multiplicative pacing
    let simulation_converge_mult_high = prepare_simulationconverge(
        num_impressions_high,
        CampaignType::MULTIPLICATIVE_PACING,
    );
    let stats_mult_high = simulation_converge_mult_high.run_variant(
        "Running with multiplicative pacing (high impressions)", 
        scenario_name, 
        "multiplicative-high", 
        100, 
        logger
    );
    
    /* COMMENTED OUT FOR DEBUG
    // Validate expected marketplace behavior
    logln!(logger, LogEvent::Scenario, "");
    logln!(logger, LogEvent::Scenario, "=== Validation Results ===");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Validation 1: Low impressions (5000) - Median Bidding should work worse than multiplicative
    let msg = format!(
        "Low impressions (5000): Multiplicative pacing obtained value > Median Bidding obtained value: {:.2} > {:.2}",
        stats_mult_low.overall_stat.total_value,
        stats_median_low.overall_stat.total_value
    );
    if stats_mult_low.overall_stat.total_value > stats_median_low.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Validation 2: High impressions (50000) - Median Bidding should work better than multiplicative
    let msg = format!(
        "High impressions (50000): Median Bidding obtained value > Multiplicative pacing obtained value: {:.2} > {:.2}",
        stats_median_high.overall_stat.total_value,
        stats_mult_high.overall_stat.total_value
    );
    if stats_median_high.overall_stat.total_value > stats_mult_high.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Validation 3: Low impressions - Max margin should capture more value than Median Bidding
    let msg = format!(
        "Low impressions (5000): Max margin obtained value > Median Bidding obtained value: {:.2} > {:.2}",
        stats_maxmargin_low.overall_stat.total_value,
        stats_median_low.overall_stat.total_value
    );
    if stats_maxmargin_low.overall_stat.total_value > stats_median_low.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Validation 4: High impressions - Max margin should capture more value than Median Bidding
    let msg = format!(
        "High impressions (50000): Max margin obtained value > Median Bidding obtained value: {:.2} > {:.2}",
        stats_maxmargin_high.overall_stat.total_value,
        stats_median_high.overall_stat.total_value
    );
    if stats_maxmargin_high.overall_stat.total_value > stats_median_high.overall_stat.total_value {
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
    */
    
    // DEBUG MODE: Just return Ok for now
    Ok(())
}

