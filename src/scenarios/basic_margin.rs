/// This is a simple scenario that uses first price bidding on HB supply.
///
/// It tests optimal bidding strategy using a fixed budget campaign.


#[allow(unused_imports)]
use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{Campaigns, CampaignGeneral};
use crate::campaign::CampaignTrait;
use crate::campaign_targets::CampaignTargetTotalBudget;
use crate::bid_valuers_single::BidValuerMultiplicative;
use crate::bid_optimizers::{BidOptimizerTrait, BidOptimizerOptimal};
use crate::bid_determination::{BidDeterminationNoMargin, BidDeterminationBidFixedMargin, BidDeterminationFixedMarginUnoptimal};
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
    short_name: "basic_margin",
    run,
});

// Margin constant used for variants B and C (10%)
const MARGIN: f64 = 0.1;

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(net_and_gross: Box<dyn crate::bid_determination::BidDeterminationTrait>, controller: Box<dyn crate::controllers::ControllerTrait>) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign using add_advanced (ID is automatically set to match Vec index)
    let campaign: Box<dyn CampaignTrait> = Box::new(CampaignGeneral {
        campaign_id: 0, // Will be set by add_advanced
        campaign_name: "Campaign 0".to_string(),
        converge_targets: vec![Box::new(CampaignTargetTotalBudget {
            total_budget_target: 10.0,
        })],
        converge_controllers: vec![controller],
        bid_valuer: Box::new(BidValuerMultiplicative),
        bid_optimizer: Box::new(BidOptimizerOptimal) as Box<dyn BidOptimizerTrait>,
        net_and_gross,
    });
    campaigns.add_advanced(campaign);

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
    // Run variant A with no margin
    let simulation_converge_a = prepare_simulationconverge(
        Box::new(BidDeterminationNoMargin::new()),
        Box::new(crate::controllers::ControllerProportionalDerivative::new())
    );
    let stats_a = simulation_converge_a.run_variant("Running with optimal bidding (no margin)", scenario_name, "optimal-bidding-no-margin", 100, logger)?;
    
    // Run variant B with fixed margin (optimal)
    let simulation_converge_b = prepare_simulationconverge(
        Box::new(BidDeterminationBidFixedMargin::new(MARGIN)),
        Box::new(crate::controllers::ControllerProportionalDerivative::new())
    );
    let stats_b = simulation_converge_b.run_variant("Running with optimal bidding (fixed margin optimal)", scenario_name, "optimal-bidding-fixed-margin", 100, logger)?;
    
    // Run variant C with fixed margin unoptimal - use slower convergence parameters
    let simulation_converge_c = prepare_simulationconverge(
        Box::new(BidDeterminationFixedMarginUnoptimal::new(MARGIN)),
        Box::new(crate::controllers::ControllerProportionalDerivative::new_advanced(
            0.005,  // tolerance_fraction
            0.03,   // max_adjustment_factor (slower: 3% vs default 20%)
            0.03,   // proportional_gain (slower: 3% vs default 10%)
            0.015,  // derivative_gain (slower: 1.5% vs default 5%)
            true,   // rescaling (default)
        ))
    );
    let stats_c = simulation_converge_c.run_variant("Running with optimal bidding (fixed margin unoptimal)", scenario_name, "optimal-bidding-fixed-margin-unoptimal", 100, logger)?;
    
    // Validate expected marketplace behavior
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A should have near zero margin (difference between gross_buyer_charge and net_supply_cost)
    let gross_buyer_charge_a = stats_a.overall_stat.total_gross_buyer_charge;
    let net_supply_cost_a = stats_a.overall_stat.total_net_supply_cost;
    let margin_a = if gross_buyer_charge_a > 0.0 {
        (gross_buyer_charge_a - net_supply_cost_a) / gross_buyer_charge_a
    } else {
        0.0
    };
    let margin_tolerance = 0.01; // 1% tolerance for near zero
    let msg = format!(
        "Variant A has near zero margin: gross={:.2}, net={:.2}, margin={:.4} (expected < {:.4})",
        gross_buyer_charge_a, net_supply_cost_a, margin_a, margin_tolerance
    );
    if margin_a.abs() <= margin_tolerance {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B should have exactly MARGIN (10%) margin
    let gross_buyer_charge_b = stats_b.overall_stat.total_gross_buyer_charge;
    let net_supply_cost_b = stats_b.overall_stat.total_net_supply_cost;
    let margin_b = if gross_buyer_charge_b > 0.0 {
        (gross_buyer_charge_b - net_supply_cost_b) / gross_buyer_charge_b
    } else {
        0.0
    };
    let margin_tolerance_pct = 0.01; // 1% tolerance (e.g., 0.09 to 0.11 for 10% margin)
    let msg = format!(
        "Variant B has exactly {:.0}% margin: gross={:.2}, net={:.2}, margin={:.4} (expected {:.4} ± {:.4})",
        MARGIN * 100.0, gross_buyer_charge_b, net_supply_cost_b, margin_b, MARGIN, margin_tolerance_pct
    );
    if (margin_b - MARGIN).abs() <= margin_tolerance_pct {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant C should have exactly MARGIN (10%) margin
    let gross_buyer_charge_c = stats_c.overall_stat.total_gross_buyer_charge;
    let net_supply_cost_c = stats_c.overall_stat.total_net_supply_cost;
    let margin_c = if gross_buyer_charge_c > 0.0 {
        (gross_buyer_charge_c - net_supply_cost_c) / gross_buyer_charge_c
    } else {
        0.0
    };
    let msg = format!(
        "Variant C has exactly {:.0}% margin: gross={:.2}, net={:.2}, margin={:.4} (expected {:.4} ± {:.4})",
        MARGIN * 100.0, gross_buyer_charge_c, net_supply_cost_c, margin_c, MARGIN, margin_tolerance_pct
    );
    if (margin_c - MARGIN).abs() <= margin_tolerance_pct {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B and C should have nearly the same payout to publisher (net_supply_cost)
    // Note: net_supply_cost_b and net_supply_cost_c are already declared above
    let net_cost_diff = (net_supply_cost_b - net_supply_cost_c).abs();
    let net_cost_avg = (net_supply_cost_b + net_supply_cost_c) / 2.0;
    let net_cost_tolerance = net_cost_avg * 0.01; // 1% tolerance
    let msg = format!(
        "Variant B and C have nearly the same payout to publisher (net_supply_cost): B={:.2}, C={:.2}, diff={:.2}, tolerance={:.2}",
        net_supply_cost_b, net_supply_cost_c, net_cost_diff, net_cost_tolerance
    );
    if net_cost_diff <= net_cost_tolerance {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B and C should have the same charge price to advertiser (gross_buyer_charge)
    // Note: gross_buyer_charge_b and gross_buyer_charge_c are already declared above
    let charge_diff = (gross_buyer_charge_b - gross_buyer_charge_c).abs();
    let charge_avg = (gross_buyer_charge_b + gross_buyer_charge_c) / 2.0;
    let charge_tolerance = charge_avg * 0.01; // 1% tolerance
    let msg = format!(
        "Variant B and C have the same charge price to advertiser (gross_buyer_charge): B={:.2}, C={:.2}, diff={:.2}, tolerance={:.2}",
        gross_buyer_charge_b, gross_buyer_charge_c, charge_diff, charge_tolerance
    );
    if charge_diff <= charge_tolerance {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant B (optimal) should have at least 5% more value than Variant C (unoptimal)
    let value_b = stats_b.overall_stat.total_value;
    let value_c = stats_c.overall_stat.total_value;
    let value_diff_pct = if value_c > 0.0 {
        ((value_b - value_c) / value_c) * 100.0
    } else {
        0.0
    };
    let min_value_diff_pct = 5.0; // 5% minimum
    let msg = format!(
        "Variant B (optimal) has at least 5% more value than Variant C (unoptimal): B={:.2}, C={:.2}, diff={:.2}%",
        value_b, value_c, value_diff_pct
    );
    if value_diff_pct >= min_value_diff_pct {
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

