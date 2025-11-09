use crate::types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Campaigns, Sellers};
use crate::simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::scenarios::Verbosity;
use crate::utils;

/// Run a variant of the simulation with a specific number of HB impressions
fn run_variant(verbosity: Verbosity, hb_impressions: usize) -> SimulationStat {
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
        num_impressions: hb_impressions,
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
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    // Pass verbosity parameter through
    SimulationConverge::run(&impressions, &campaigns, &sellers, &mut campaign_params, 100, verbosity);
    
    // Run final simulation and return statistics
    let final_simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params);
    let stats = SimulationStat::new(&campaigns, &sellers, &impressions, &final_simulation_run);
    
    // Print final stats if Summary or Full verbosity
    if verbosity >= Verbosity::Summary {
        if verbosity == Verbosity::Full {
            stats.printout(&campaigns, &sellers, &campaign_params);
        } else {
            // Summary mode: only print overall stats
            stats.printout_overall();
        }
    }
    
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
pub fn run(verbosity: Verbosity) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant with 100 HB impressions
    if verbosity >= Verbosity::Summary {
        println!("\n=== Running with Scarce HB impressions ===");
    }
    let stats_A = run_variant(verbosity, 100);
    
    // Run variant with 1000 HB impressions
    if verbosity >= Verbosity::Summary {
        println!("\n=== Running with Abundant HB impressions ===");
    }
    let stats_B = run_variant(verbosity, 1000);
    
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
    
    if verbosity >= Verbosity::Summary {
        println!();
    }
    
    let mut errors = Vec::new();
    
    // Check: Variant A has higher total cost charged to buyers
    if stats_A.overall_stat.total_buyer_charge <= stats_B.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant A (Scarce HB) to have higher total buyer charge than variant B (Abundant HB), but got {} <= {}",
            stats_A.overall_stat.total_buyer_charge,
            stats_B.overall_stat.total_buyer_charge
        ));
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (Scarce HB) has higher total buyer charge: {:.4} > {:.4}",
            stats_A.overall_stat.total_buyer_charge,
            stats_B.overall_stat.total_buyer_charge
        );
    }
    
    // Check: Variant A has lower total value
    if stats_A.overall_stat.total_value >= stats_B.overall_stat.total_value {
        errors.push(format!(
            "Expected variant A (Scarce HB) to have lower total value than variant B (Abundant HB), but got {} >= {}",
            stats_A.overall_stat.total_value,
            stats_B.overall_stat.total_value
        ));
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (Scarce HB) has lower total value: {:.4} < {:.4}",
            stats_A.overall_stat.total_value,
            stats_B.overall_stat.total_value
        );
    }
    
    // Check: In variant A, cost of inventory is lower than cost charged to buyers
    if stats_A.overall_stat.total_supply_cost >= stats_A.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant A (Scarce HB) to have supply cost < buyer charge (profitable), but got {} >= {}",
            stats_A.overall_stat.total_supply_cost,
            stats_A.overall_stat.total_buyer_charge
        ));
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant A (Scarce HB) is profitable (supply_cost < buyer_charge): {:.4} < {:.4}",
            stats_A.overall_stat.total_supply_cost,
            stats_A.overall_stat.total_buyer_charge
        );
    }
    
    // Check: In variant B, cost of inventory is higher than cost charged to buyers
    if stats_B.overall_stat.total_supply_cost <= stats_B.overall_stat.total_buyer_charge {
        errors.push(format!(
            "Expected variant B (Abundant HB) to have supply cost > buyer charge (unprofitable), but got {} <= {}",
            stats_B.overall_stat.total_supply_cost,
            stats_B.overall_stat.total_buyer_charge
        ));
    } else if verbosity >= Verbosity::Summary {
        println!("✓ Variant B (Abundant HB) is unprofitable (supply_cost > buyer_charge): {:.4} > {:.4}",
            stats_B.overall_stat.total_supply_cost,
            stats_B.overall_stat.total_buyer_charge
        );
    }
    
    if errors.is_empty() {
        if verbosity >= Verbosity::Summary {
            println!("\nAll validations passed!");
        }
        Ok(())
    } else {
        Err(format!("Scenario 'HBabundance' validation failed:\n{}", errors.join("\n")).into())
    }
}

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "HBabundance",
    description: "Example of how availability of a lot of HB impressions changes pricing to buyer (downwards) and increases bought value. But increasing HB impressions leads to price advertiser pays being less than what we need to pay to supply",
    run,
});

