use crate::types::{CampaignType, ChargeType, Campaigns, Marketplace, Sellers};
use crate::simulationrun::{CampaignParams, SellerParams, SimulationStat};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::scenarios::Verbosity;
use crate::utils;

/// Run a variant of the simulation with a specific MRG boost factor
fn run_variant(verbosity: Verbosity, mrg_boost_factor: f64, variant_description: &str) -> SimulationStat {
    // Print variant description if Summary or Full verbosity
    if verbosity >= Verbosity::Summary {
        println!("\n=== {} ===", variant_description);
    }
    
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

    if verbosity == Verbosity::Full {
        marketplace.printout();
    }

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let initial_campaign_params = CampaignParams::new(&marketplace.campaigns);
    // Create seller parameters from sellers (default boost_factor = 1.0)
    let mut initial_seller_params = SellerParams::new(&marketplace.sellers);
    // Set boost_factor for MRG seller (seller_id 0)
    initial_seller_params.params[0].boost_factor = mrg_boost_factor;
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    // Pass verbosity parameter through
    let (_final_simulation_run, stats, final_campaign_params) = SimulationConverge::run(&marketplace, &initial_campaign_params, &initial_seller_params, 100, verbosity);
    
    // Print final stats if Summary or Full verbosity
    if verbosity >= Verbosity::Summary {
        if verbosity == Verbosity::Full {
            // Full verbosity: print campaigns, sellers, and overall stats
            stats.printout(&marketplace.campaigns, &marketplace.sellers, &final_campaign_params);
        } else {
            // Summary mode: only print overall stats
            stats.printout_overall();
        }
    }
    
    stats
}

/// Scenario demonstrating the effect of MRG seller boost factor on marketplace dynamics
/// 
/// This scenario compares the abundant HB variant (1000 HB impressions) with and without
/// a boost factor of 2.0 applied to the MRG seller. The boost factor affects how MRG
/// impressions are valued in the marketplace.
pub fn run(verbosity: Verbosity) -> Result<(), Box<dyn std::error::Error>> {

    // Run variant with boost_factor = 1.0 (default) for MRG seller
    let stats_A = run_variant(verbosity, 1.0, "Running with Abundant HB impressions (MRG boost: 1.0)");    
    // Run variant with boost_factor = 2.0 for MRG seller
    let stats_B = run_variant(verbosity, 2.0, "Running with Abundant HB impressions (MRG boost: 2.0)");
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (boost 1.0) vs Variant B (boost 2.0):
    // - Variant A is unprofitable (overall), while variant B is profitable
    // - Specifically seller 0 (MRG) is unprofitable in variant A and profitable in variant B
    // - Variant A should obtain more total value than variant B
    // - Variant A should have lower total cost than variant B
    
    if verbosity >= Verbosity::Summary {
        println!();
    }
    
    let mut errors: Vec<String> = Vec::new();
    
    // Check: Variant A is unprofitable (overall)
    if stats_A.overall_stat.total_supply_cost <= stats_A.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant A (MRG boost 1.0) to be unprofitable (supply_cost > buyer_charge), but got {} <= {}",
            stats_A.overall_stat.total_supply_cost,
            stats_A.overall_stat.total_buyer_charge
        ));
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.4} > {:.4}",
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
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.4} < {:.4}",
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
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Seller 0 (MRG) in variant A (MRG boost 1.0) is unprofitable (supply_cost > buyer_charge): {:.4} > {:.4}",
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
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Seller 0 (MRG) in variant B (MRG boost 2.0) is profitable (supply_cost < buyer_charge): {:.4} < {:.4}",
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
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (MRG boost 1.0) has more total value: {:.4} > {:.4}",
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
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (MRG boost 1.0) has lower total cost: {:.4} < {:.4}",
            stats_A.overall_stat.total_buyer_charge,
            stats_B.overall_stat.total_buyer_charge
        );
    }
    
    if errors.is_empty() {
        if verbosity >= Verbosity::Summary {
            println!("\nAll validations passed!");
        }
        Ok(())
    } else {
        Err(format!("Scenario 'MRGboost' validation failed:\n{}", errors.join("\n")).into())
    }
}

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "MRGboost",
    description: "Demonstrates the effect of MRG seller boost factor on marketplace dynamics",
    run,
});

