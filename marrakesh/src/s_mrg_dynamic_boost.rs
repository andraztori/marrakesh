use crate::types::{ChargeType, Marketplace};
use crate::sellers::Sellers;
use crate::campaigns::{CampaignType, Campaigns};
use crate::simulationrun::{CampaignConvergeParams, SellerConvergeParams, SimulationStat};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::utils;
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::errln;
use std::path::PathBuf;

/// Run a variant of the simulation with a specific MRG boost factor
fn run_variant(mrg_boost_factor: f64, variant_description: &str, scenario_name: &str, variant_name: &str, logger: &mut Logger) -> SimulationStat {
    // Add variant iterations receiver (for simulation and convergence events)
    let iterations_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}_iterations.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Simulation, LogEvent::Convergence]));
    
    // Add variant receiver (for variant events)
    let variant_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Variant]));
    
    logln!(logger, LogEvent::Variant, "\n=== {} ===", variant_description);
    
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 100,
        },  // campaign_type
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        CampaignType::FIXED_BUDGET {
            total_budget_target: 2.0,
        },  // campaign_type
    );

    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(
        "MRG".to_string(),  // seller_name
        ChargeType::FIXED_COST {
            fixed_cost_cpm: 10.0,
        },  // charge_type
        100,  // num_impressions
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        ChargeType::FIRST_PRICE,  // charge_type
        1000,  // num_impressions
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

    marketplace.printout(logger);

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let initial_campaign_converge_params = CampaignConvergeParams::new(&marketplace.campaigns);
    // Create seller parameters from sellers (default boost_factor = 1.0)
    let mut initial_seller_converge_params = SellerConvergeParams::new(&marketplace.sellers);
    // Set boost_factor for MRG seller (seller_id 0)
    initial_seller_converge_params.params[0].boost_factor = mrg_boost_factor;
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    let (_final_simulation_run, stats, final_campaign_converge_params) = SimulationConverge::run(&marketplace, &initial_campaign_converge_params, &initial_seller_converge_params, 100, variant_name, logger);
    
    // Print final stats (variant-level output)
    stats.printout(&marketplace.campaigns, &marketplace.sellers, &final_campaign_converge_params, logger);
    
    // Remove variant-specific receivers
    logger.remove_receiver(variant_receiver_id);
    logger.remove_receiver(iterations_receiver_id);
    
    stats
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
    
    // Run variant with boost_factor = 1.0 (default) for MRG seller
    let stats_a = run_variant(1.0, "Running with Abundant HB impressions (MRG boost: 1.0)", scenario_name, "boost_1.0", logger);
    
    // Run variant with boost_factor = 2.0 for MRG seller
    let stats_b = run_variant(2.0, "Running with Abundant HB impressions (MRG boost: 2.0)", scenario_name, "boost_2.0", logger);
    
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
