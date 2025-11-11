use crate::types::Marketplace;
use crate::sellers::SellerType;
use crate::sellers::Sellers;
use crate::campaigns::{CampaignType, Campaigns};
use crate::simulationrun::SimulationStat;
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::utils;
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::errln;
use std::path::PathBuf;

/// Run a variant of the simulation with a specific number of HB impressions
fn run_variant(hb_impressions: usize, variant_description: &str, scenario_name: &str, variant_name: &str, logger: &mut Logger) -> SimulationStat {
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
        SellerType::FIXED_COST_FIXED_BOOST {
            fixed_cost_cpm: 10.0,
        },  // charge_type
        1000,  // num_impressions
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        hb_impressions,  // num_impressions
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

    // Create simulation converge instance (initializes campaign and seller params internally)
    let simulation_converge = SimulationConverge::new(marketplace);
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    let (_final_simulation_run, stats, final_campaign_converge_params) = simulation_converge.run(100, variant_name, logger);
    
    // Print final stats (variant-level output)
    // Note: We use initial_seller_converge_params here since converge doesn't return final seller params
    // The seller params should be converged by this point anyway
    stats.printout(&simulation_converge.marketplace.campaigns, &simulation_converge.marketplace.sellers, &final_campaign_converge_params, &simulation_converge.initial_seller_converge_params, logger);
    
    // Remove variant-specific receivers
    logger.remove_receiver(variant_receiver_id);
    logger.remove_receiver(iterations_receiver_id);
    
    stats
}

/// Scenario of how availability of a lot of HB impressions changes pricing to buyer (downwards) 
/// and increases bought value. But increasing HB impressions leads to price advertiser pays 
/// being less than what we need to pay to supply
/// 
/// This scenario demonstrates a key marketplace dynamic: when header bidding (HB) inventory is scarce (100 impressions),
/// buyers pay higher prices due to competition. However, when HB inventory is abundant (1000 impressions),
/// increased supply drives prices down for buyers, but creates a problem where the cost to acquire inventory
/// (supply_cost) exceeds what buyers are charged (buyer_charge), making the marketplace unprofitable.
/// 
/// Expected behavior:
/// - With 100 HB impressions: Higher buyer charges, lower total value, but supply_cost < buyer_charge (profitable)
/// - With 1000 HB impressions: Lower buyer charges, higher total value, but supply_cost > buyer_charge (unprofitable)
pub fn run(logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    let scenario_name = "HBabundance";
    
    // Add scenario-level receiver
    let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario_name))), vec![LogEvent::Scenario]));
    
    // Run variant with 100 HB impressions
    let stats_a = run_variant(1000, "Running with Scarce HB impressions", scenario_name, "scarce", logger);
    
    // Run variant with 1000 HB impressions
    let stats_b = run_variant(10000, "Running with Abundant HB impressions", scenario_name, "abundant", logger);
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (100 HB) should have:
    // - Higher total cost charged to buyers (due to scarcity driving up prices)
    // - Lower total value obtained (fewer impressions available)
    // - Supply cost < buyer charge (marketplace is profitable)
    //
    // Variant B (1000 HB) should have:
    // - Lower total cost charged to buyers (abundance drives prices down)
    // - Higher total value obtained (more impressions available)
    // - Supply cost > buyer charge (marketplace becomes unprofitable)
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors = Vec::new();
    
    // Check: Variant A has higher total cost charged to buyers
    let msg = format!(
        "Variant A (Scarce HB) has higher total buyer charge than variant B (Abundant HB): {:.2} > {:.2}",
        stats_a.overall_stat.total_buyer_charge,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_buyer_charge > stats_b.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant A has lower total value
    let msg = format!(
        "Variant A (Scarce HB) has lower total value than variant B (Abundant HB): {:.2} < {:.2}",
        stats_a.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    if stats_a.overall_stat.total_value < stats_b.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: In variant A, cost of inventory is lower than cost charged to buyers
    let msg = format!(
        "Variant A (Scarce HB) is profitable (supply_cost < buyer_charge): {:.2} < {:.2}",
        stats_a.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_supply_cost < stats_a.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: In variant B, cost of inventory is higher than cost charged to buyers
    let msg = format!(
        "Variant B (Abundant HB) is unprofitable (supply_cost > buyer_charge): {:.2} > {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_b.overall_stat.total_supply_cost > stats_b.overall_stat.total_buyer_charge {
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
    short_name: "HBabundance",
    run,
});
