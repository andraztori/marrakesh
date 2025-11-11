use crate::types::Marketplace;
use crate::sellers::SellerType;
use crate::sellers::Sellers;
use crate::campaigns::{CampaignType, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::utils;
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::errln;
use std::path::PathBuf;

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(dynamic_boost: bool) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 1000,
        },  // campaign_type
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        CampaignType::FIXED_BUDGET {
            total_budget_target: 20.0,
        },  // campaign_type
    );

    // Add two sellers (IDs are automatically set to match Vec index)
    // First seller (MRG) type depends on dynamic_boost parameter
    sellers.add(
        "MRG".to_string(),  // seller_name
        if dynamic_boost {
            SellerType::FIXED_COST_DYNAMIC_BOOST {
                fixed_cost_cpm: 10.0,
            }
        } else {
            SellerType::FIXED_COST_FIXED_BOOST {
                fixed_cost_cpm: 10.0,
            }
        },
        1000,  // num_impressions
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        10000,  // num_impressions
    );

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // best_other_bid_dist
        utils::lognormal_dist(10.0, 3.0),  // floor_cpm_dist
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
        0.0,   // fixed_cost_floor_cpm
    );
    let impressions = Impressions::new(&sellers, &impressions_params);

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace {
        campaigns,
        sellers,
        impressions,
    };

    // Create simulation converge instance (initializes campaign and seller params internally)
    SimulationConverge::new(marketplace)
}


/// Scenario demonstrating the effect of MRG seller boost factor on marketplace dynamics
/// 
/// This scenario compares the abundant HB variant (1000 HB impressions) with and without
/// a boost factor of 2.0 applied to the MRG seller. The boost factor affects how MRG
/// impressions are valued in the marketplace.
pub fn run(logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    let scenario_name = "MRGdynamicboost";
    
    // Add scenario-level receiver
    let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario_name))), vec![LogEvent::Scenario]));
    
    // Run variant with fixed boost (no convergence) for MRG seller
    let simulation_converge_a = prepare_simulationconverge(false);
    let stats_a = simulation_converge_a.run_variant("Running with Abundant HB impressions", scenario_name, "no_boost", 100, logger);
    
    // Run variant with dynamic boost (convergence) for MRG seller
    let simulation_converge_b = prepare_simulationconverge(true);
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
    
    // Remove scenario-level receiver
    logger.remove_receiver(scenario_receiver_id);
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "MRGdynamicboost",
    run,
});
