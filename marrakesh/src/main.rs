mod types;
mod simulationrun;
mod converge;
mod utils;
mod impressions;
mod scenarios;

// Include scenario files so their constructors run
mod s_one;

use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Campaigns, Sellers};
use simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use converge::SimulationConverge;
use impressions::Impressions;

use scenarios::Verbosity;

fn main() {
    // Run the s_one scenario
    if let Err(e) = s_one::run(Verbosity::Summary) {
        eprintln!("Error running scenario: {}", e);
        std::process::exit(1);
    }
    
    // Old main code (unreachable)
    if false {
        // Initialize containers for campaigns and sellers
        let mut campaigns = Campaigns::new();
        let mut sellers = Sellers::new();

        // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
        campaigns.add(AddCampaignParams {
            campaign_name: "Campaign 0".to_string(),
            campaign_rnd: 12345,
            campaign_type: CampaignType::FIXED_IMPRESSIONS {
                total_impressions_target: 1000,
            },
        }).expect("Failed to add campaign");

        campaigns.add(AddCampaignParams {
            campaign_name: "Campaign 1".to_string(),
            campaign_rnd: 67890,
            campaign_type: CampaignType::FIXED_BUDGET {
                total_budget_target: 20.0,
            },
        }).expect("Failed to add campaign");


        // Add two sellers (IDs are automatically set to match Vec index)
        sellers.add(AddSellerParams {
            seller_name: "MRG".to_string(),
            charge_type: ChargeType::FIXED_COST {
                fixed_cost_cpm: 10.0,
            },
            num_impressions: 1000,
        });

        sellers.add(AddSellerParams {
            seller_name: "HB".to_string(),
            charge_type: ChargeType::FIRST_PRICE,
            num_impressions: 1000,
        });

        // Create impressions for all sellers using default parameters
        let impressions_params = impressions::ImpressionsParam::new(
            utils::lognormal_dist(10.0, 3.0),  // best_other_bid_dist
            utils::lognormal_dist(10.0, 3.0),  // floor_cpm_dist
            utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
            utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
            0.0,   // fixed_cost_floor_cpm
        );
        let impressions = Impressions::new(&sellers, &impressions_params);

        println!("Initialized {} sellers", sellers.sellers.len());
        println!("Initialized {} campaigns", campaigns.campaigns.len());
        println!("Initialized {} impressions", impressions.impressions.len());

        // Create campaign parameters from campaigns (default pacing = 1.0)
        let mut campaign_params = CampaignParams::new(&campaigns);
        
        // Run simulation loop with pacing adjustments (maximum 100 iterations)
        // Verbosity::None means only print convergence message and final solution
        SimulationConverge::run(&impressions, &campaigns, &sellers, &mut campaign_params, 100, Verbosity::None);
        
        // Run final simulation and output complete statistics
        let final_simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params);
        let final_stats = SimulationStat::new(&campaigns, &sellers, &impressions, &final_simulation_run);
        println!("\n=== Final Results ===");
        final_stats.printout(&campaigns, &sellers, &campaign_params);
    }
}
