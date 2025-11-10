use crate::types::{CampaignType, ChargeType, Campaigns, Marketplace, Sellers};
use crate::simulationrun::{CampaignParams, SellerParams, SimulationStat};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::scenarios::Verbosity;
use crate::utils;
use crate::logger::{Logger, LogEvent, ConsoleReceiver, FileReceiver, sanitize_filename};
use crate::logln;
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
    sellers.add(
        "MRG".to_string(),  // seller_name
        ChargeType::FIXED_COST {
            fixed_cost_cpm: 10.0,
        },  // charge_type
        1000,  // num_impressions
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        ChargeType::FIRST_PRICE,  // charge_type
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

    marketplace.printout(logger);

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let initial_campaign_params = CampaignParams::new(&marketplace.campaigns);
    // Create seller parameters from sellers (default boost_factor = 1.0)
    let mut initial_seller_params = SellerParams::new(&marketplace.sellers);
    // Set boost_factor for MRG seller (seller_id 0)
    initial_seller_params.params[0].boost_factor = mrg_boost_factor;
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    let (_final_simulation_run, stats, final_campaign_params) = SimulationConverge::run(&marketplace, &initial_campaign_params, &initial_seller_params, 100, logger);
    
    // Print final stats (variant-level output)
    stats.printout(&marketplace.campaigns, &marketplace.sellers, &final_campaign_params, logger);
    
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
pub fn run(_verbosity: Verbosity) -> Result<(), Box<dyn std::error::Error>> {
    let scenario_name = "MRGboost";
    
    // Set up main logger with console receiver
    let mut logger = Logger::new();
    let console_receiver = ConsoleReceiver::new(vec![
        LogEvent::Simulation,
        LogEvent::Convergence,
        LogEvent::Variant,
        LogEvent::Scenario,
        LogEvent::Validation,
    ]);
    logger.add_receiver(Box::new(console_receiver));
    
    // Add scenario-level receiver
    let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario_name))), vec![LogEvent::Scenario]));

    // Run variant with boost_factor = 1.0 (default) for MRG seller
    let stats_A = run_variant(1.0, "Running with Abundant HB impressions (MRG boost: 1.0)", scenario_name, "boost_1.0", &mut logger);    
    // Run variant with boost_factor = 2.0 for MRG seller
    let stats_B = run_variant(2.0, "Running with Abundant HB impressions (MRG boost: 2.0)", scenario_name, "boost_2.0", &mut logger);
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (boost 1.0) vs Variant B (boost 2.0):
    // - Variant A is unprofitable (overall), while variant B is profitable
    // - Specifically seller 0 (MRG) is unprofitable in variant A and profitable in variant B
    // - Variant A should obtain more total value than variant B
    // - Variant A should have lower total cost than variant B
    
    logln!(&mut logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A is unprofitable (overall)
    if stats_A.overall_stat.total_supply_cost <= stats_A.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant A (MRG boost 1.0) to be unprofitable (supply_cost > buyer_charge), but got {} <= {}",
            stats_A.overall_stat.total_supply_cost,
            stats_A.overall_stat.total_buyer_charge
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.4} > {:.4}",
            stats_A.overall_stat.total_supply_cost,
            stats_A.overall_stat.total_buyer_charge
        );
    }
    
    // Check: Variant B is profitable (overall)
    if stats_B.overall_stat.total_supply_cost >= stats_B.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant B (MRG boost 2.0) to be profitable (supply_cost < buyer_charge), but got {} >= {}",
            stats_B.overall_stat.total_supply_cost,
            stats_B.overall_stat.total_buyer_charge
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.4} < {:.4}",
            stats_B.overall_stat.total_supply_cost,
            stats_B.overall_stat.total_buyer_charge
        );
    }
    
    // Check: Seller 0 (MRG) is unprofitable in variant A
    if stats_A.seller_stats[0].total_supply_cost <= stats_A.seller_stats[0].total_buyer_charge {
        errors.push(format!(
            "Expected seller 0 (MRG) in variant A (MRG boost 1.0) to be unprofitable (supply_cost > buyer_charge), but got {} <= {}",
            stats_A.seller_stats[0].total_supply_cost,
            stats_A.seller_stats[0].total_buyer_charge
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Seller 0 (MRG) in variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.4} > {:.4}",
            stats_A.seller_stats[0].total_supply_cost,
            stats_A.seller_stats[0].total_buyer_charge
        );
    }
    
    // Check: Seller 0 (MRG) is profitable in variant B
    if stats_B.seller_stats[0].total_supply_cost >= stats_B.seller_stats[0].total_buyer_charge {
        errors.push(format!(
            "Expected seller 0 (MRG) in variant B (MRG boost 2.0) to be profitable (supply_cost < buyer_charge), but got {} >= {}",
            stats_B.seller_stats[0].total_supply_cost,
            stats_B.seller_stats[0].total_buyer_charge
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Seller 0 (MRG) in variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.4} < {:.4}",
            stats_B.seller_stats[0].total_supply_cost,
            stats_B.seller_stats[0].total_buyer_charge
        );
    }
    
    // Check: Variant A has more total value than variant B
    if stats_A.overall_stat.total_value <= stats_B.overall_stat.total_value {
        errors.push(format!(
            "Expected variant A (MRG boost 1.0) to have more total value than variant B (MRG boost 2.0), but got {} <= {}",
            stats_A.overall_stat.total_value,
            stats_B.overall_stat.total_value
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Variant A (MRG boost 1.0) has more total value: {:.4} > {:.4}",
            stats_A.overall_stat.total_value,
            stats_B.overall_stat.total_value
        );
    }
    
    // Check: Variant A has lower total cost than variant B
    if stats_A.overall_stat.total_buyer_charge >= stats_B.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant A (MRG boost 1.0) to have lower total cost than variant B (MRG boost 2.0), but got {} >= {}",
            stats_A.overall_stat.total_buyer_charge,
            stats_B.overall_stat.total_buyer_charge
        ));
    } else {
        logln!(&mut logger, LogEvent::Scenario, "✓ Variant A (MRG boost 1.0) has lower total cost: {:.4} < {:.4}",
            stats_A.overall_stat.total_buyer_charge,
            stats_B.overall_stat.total_buyer_charge
        );
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
    short_name: "MRGboost",
    description: "Demonstrates the effect of MRG seller boost factor on marketplace dynamics",
    run,
});
