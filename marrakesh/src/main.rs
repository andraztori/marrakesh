mod types;
mod simulationrun;
mod converge;
mod utils;
mod impressions;
mod scenarios;

// Include scenario files so their constructors run
mod s_one;
mod s_mrg_boost;
mod s_mrg_dynamic_boost;

use types::{CampaignType, ChargeType, Campaigns, Sellers};
use simulationrun::{CampaignParams, SellerParams};
use converge::SimulationConverge;
use impressions::Impressions;

use scenarios::{Verbosity, get_scenario_catalog};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // Check if "all" argument is provided
    if args.len() > 1 && args[1] == "all" {
        // Run all scenarios from the catalog in non-verbose mode
        let scenarios = get_scenario_catalog();
        println!("Running all scenarios...\n");
        
        for scenario in scenarios {
            print!("{}: ", scenario.short_name);
            match (scenario.run)(Verbosity::None) {
                Ok(()) => println!("✓ PASSED"),
                Err(e) => {
                    println!("✗ FAILED");
                    eprintln!("  Error: {}", e);
                }
            }
        }
    } else {
        // Default behavior: Run the first scenario (or s_mrg_boost) with summary verbosity
        // For now, default to s_mrg_boost, but could be made configurable
        if let Err(e) = s_mrg_boost::run(Verbosity::Summary) {
            eprintln!("Error running scenario: {}", e);
            std::process::exit(1);
        }
    }
    
    // Old main code (unreachable)
    if false {
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
            1000,  // num_impressions
        );

        // Create impressions for all sellers using default parameters
        let impressions_params = impressions::ImpressionsParam::new(
            utils::lognormal_dist(10.0, 3.0),  // best_other_bid_dist
            utils::lognormal_dist(10.0, 3.0),  // floor_cpm_dist
            utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
            utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
            0.0,   // fixed_cost_floor_cpm
        );
        let impressions = Impressions::new(&sellers, &impressions_params);

        // Create marketplace containing campaigns, sellers, and impressions
        let marketplace = types::Marketplace {
            campaigns,
            sellers,
            impressions,
        };
        
        marketplace.printout();

        // Create campaign parameters from campaigns (default pacing = 1.0)
        let initial_campaign_params = CampaignParams::new(&marketplace.campaigns);
        // Create seller parameters from sellers (default boost_factor = 1.0)
        let initial_seller_params = SellerParams::new(&marketplace.sellers);
        
        // Run simulation loop with pacing adjustments (maximum 100 iterations)
        // Verbosity::None means only print convergence message and final solution
        let (_final_simulation_run, final_stats, final_campaign_params) = SimulationConverge::run(&marketplace, &initial_campaign_params, &initial_seller_params, 100, Verbosity::None);
        println!("\n=== Final Results ===");
        final_stats.printout(&marketplace.campaigns, &marketplace.sellers, &final_campaign_params);
    }
}
