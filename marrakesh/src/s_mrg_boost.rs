use crate::types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Campaigns, Sellers};
use crate::simulationrun::{CampaignParams, SellerParams, SimulationRun, SimulationStat};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::scenarios::Verbosity;
use crate::utils;

/// Run a variant of the simulation with a specific MRG boost factor
fn run_variant(verbosity: Verbosity, mrg_boost_factor: f64) -> SimulationStat {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 0".to_string(),
        campaign_type: CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 100,
        },
    });

    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 1".to_string(),
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 2.0,
        },
    });


    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(AddSellerParams {
        seller_name: "MRG".to_string(),
        charge_type: ChargeType::FIXED_COST {
            fixed_cost_cpm: 10.0,
        },
        num_impressions: 100,
    });

    sellers.add(AddSellerParams {
        seller_name: "HB".to_string(),
        charge_type: ChargeType::FIRST_PRICE,
        num_impressions: 1000,
    });

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // best_other_bid_dist
        utils::lognormal_dist(10.0, 3.0),  // floor_cpm_dist
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
        0.0,   // fixed_cost_floor_cpm
    );
    let impressions = Impressions::new(&sellers, &impressions_params);

    if verbosity == Verbosity::Full {
        println!("Initialized {} sellers", sellers.sellers.len());
        println!("Initialized {} campaigns", campaigns.campaigns.len());
        println!("Initialized {} impressions", impressions.impressions.len());
    }

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let mut campaign_params = CampaignParams::new(&campaigns);
    // Create seller parameters from sellers (default boost_factor = 1.0)
    let mut seller_params = SellerParams::new(&sellers);
    // Set boost_factor for MRG seller (seller_id 0)
    seller_params.params[0].boost_factor = mrg_boost_factor;
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    // Pass verbosity parameter through
    SimulationConverge::run(&impressions, &campaigns, &sellers, &mut campaign_params, &seller_params, 100, verbosity);
    
    // Run final simulation and return statistics
    let final_simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params, &sellers, &seller_params);
    let stats = SimulationStat::new(&campaigns, &sellers, &impressions, &final_simulation_run);
    
    // Print final stats if Summary or Full verbosity
    if verbosity >= Verbosity::Summary {
        if verbosity == Verbosity::Full {
            // Full verbosity: print campaigns, sellers, and overall stats
            stats.printout(&campaigns, &sellers, &campaign_params);
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
    if verbosity >= Verbosity::Summary {
        println!("\n=== Running with Abundant HB impressions (MRG boost: 1.0) ===");
    }
    let stats_A = run_variant(verbosity, 1.0);
    
    // Run variant with boost_factor = 2.0 for MRG seller
    if verbosity >= Verbosity::Summary {
        println!("\n=== Running with Abundant HB impressions (MRG boost: 2.0) ===");
    }
    let stats_B = run_variant(verbosity, 2.0);
    
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

