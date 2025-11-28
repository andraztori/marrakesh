/// In this scenario we compare two variants:
///
/// - One with unprofitable MRG seller due to too much HB supply bringing prices below supply
///   guaranteed prices
///
/// - The second one where MRG seller forces higher valuation of its supply by demand and thus
///   gets higher prices and market balances/becomes profitable

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
    short_name: "supply_simple_boost",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(mrg_boost_factor: f64) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        vec![ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: 1000 }],
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 }],
    );

    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(
        "MRG".to_string(),  // seller_name
        SellerType::FIXED_PRICE {
            fixed_cost_cpm: 10.0,
        },  // seller_type
        SellerConvergeStrategy::NONE { default_value: mrg_boost_factor },  // seller_converge
        1000,  // impressions_on_offer
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
    let simulation_converge = SimulationConverge::new(marketplace);
    
    simulation_converge
}


/// Scenario demonstrating the effect of MRG seller boost factor on marketplace dynamics
/// 
/// This scenario compares the abundant HB variant (1000 HB impressions) with and without
/// a boost factor of 2.0 applied to the MRG seller. The boost factor affects how MRG
/// impressions are valued in the marketplace.
pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant with boost_factor = 1.0 (default) for MRG seller
    let simulation_converge_a = prepare_simulationconverge(1.0);
    let stats_a = simulation_converge_a.run_variant("Running with Abundant HB impressions (MRG boost: 1.0)", scenario_name, "boost_1.0", 100, logger);
    
    // Run variant with boost_factor = 2.0 for MRG seller
    let simulation_converge_b = prepare_simulationconverge(2.0);
    let stats_b = simulation_converge_b.run_variant("Running with Abundant HB impressions (MRG boost: 2.0)", scenario_name, "boost_2.0", 100, logger);
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (boost 1.0) vs Variant B (boost 2.0):
    // - Variant A is unprofitable (overall), while variant B is profitable
    // - Specifically seller 0 (MRG) is unprofitable in variant A and profitable in variant B
    // - Variant A should obtain more total value than variant B
    // - Variant A should have lower total cost than variant B
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A is unprofitable (overall)
    let msg = format!(
        "Variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.2} > {:.2}",
        stats_a.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_supply_cost > stats_a.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant B is profitable (overall)
    let msg = format!(
        "Variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.2} < {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_b.overall_stat.total_supply_cost < stats_b.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Seller 0 (MRG) is unprofitable in variant A
    let msg = format!(
        "Seller 0 (MRG) in variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.2} > {:.2}",
        stats_a.seller_stats[0].total_supply_cost,
        stats_a.seller_stats[0].total_buyer_charge
    );
    if stats_a.seller_stats[0].total_supply_cost > stats_a.seller_stats[0].total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Seller 0 (MRG) is profitable in variant B
    let msg = format!(
        "Seller 0 (MRG) in variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.2} < {:.2}",
        stats_b.seller_stats[0].total_supply_cost,
        stats_b.seller_stats[0].total_buyer_charge
    );
    if stats_b.seller_stats[0].total_supply_cost < stats_b.seller_stats[0].total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant A has more total value than variant B
    let msg = format!(
        "Variant A (MRG boost 1.0) has more total value than variant B (MRG boost 2.0): {:.2} > {:.2}",
        stats_a.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    if stats_a.overall_stat.total_value > stats_b.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant A has lower total cost than variant B
    let msg = format!(
        "Variant A (MRG boost 1.0) has lower total cost than variant B (MRG boost 2.0): {:.2} < {:.2}",
        stats_a.overall_stat.total_buyer_charge,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_buyer_charge < stats_b.overall_stat.total_buyer_charge {
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
