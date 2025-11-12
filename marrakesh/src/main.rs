mod simulationrun;
mod converge;
mod utils;
mod impressions;
mod campaigns;
mod sellers;
mod scenarios;
mod logger;

// Include scenario files so their constructors run
mod s_one;
mod s_mrg_boost;
mod s_mrg_dynamic_boost;

use sellers::SellerType;
use sellers::Sellers;
use campaigns::{CampaignType, Campaigns};
use converge::SimulationConverge;
use impressions::Impressions;
use logger::{Logger, LogEvent, ConsoleReceiver, FileReceiver, sanitize_filename};
use std::path::PathBuf;

use scenarios::get_scenario_catalog;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // Check if "all" argument is provided
    if args.len() > 1 && args[1] == "all" {
        // Set up logger with console and validation file receivers
        let mut logger = Logger::new();
        logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation]));
        
        // Add validation receiver (for validation events)
        let summary_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from("log/summary.log"), vec![LogEvent::Validation]));
        
        // Run all scenarios from the catalog in non-verbose mode
        let scenarios = get_scenario_catalog();
        logln!(&mut logger, LogEvent::Validation, "Running all scenarios...\n");
        
        for scenario in scenarios {
            log!(&mut logger, LogEvent::Validation, "{}: ", scenario.short_name);
            
            // Add scenario-level receiver
            let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario.short_name))), vec![LogEvent::Scenario]));
            
            match (scenario.run)(scenario.short_name, &mut logger) {
                Ok(()) => logln!(&mut logger, LogEvent::Validation, "✓ PASSED"),
                Err(_e) => {
                    logln!(&mut logger, LogEvent::Validation, "✗ FAILED");
                }
            }
            
            // Remove scenario-level receiver
            logger.remove_receiver(scenario_receiver_id);
            
            // Flush to ensure validation is written to summary.log
            let _ = logger.flush();
        }
        
        // Remove validation receiver
        logger.remove_receiver(summary_receiver_id);
    } else {
        // Default behavior: Run the first scenario (or s_mrg_boost) with summary verbosity
        // For now, default to s_mrg_boost, but could be made configurable
        let mut logger = Logger::new();
        logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Simulation, LogEvent::Convergence, LogEvent::Variant]));
        if let Err(e) = s_mrg_boost::run("MRGboost", &mut logger) {
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
            SellerType::FIXED_COST_FIXED_BOOST {
                fixed_cost_cpm: 10.0,
            },  // charge_type
            1000,  // num_impressions
        );

        sellers.add(
            "HB".to_string(),  // seller_name
            SellerType::FIRST_PRICE,  // seller_type
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
        let marketplace = simulationrun::Marketplace {
            campaigns,
            sellers,
            impressions,
        };
        
        let mut logger = Logger::new();
        logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Simulation, LogEvent::Convergence, LogEvent::Variant]));
        
        marketplace.printout(&mut logger);

        // Create simulation converge instance (initializes campaign and seller params internally)
        let simulation_converge = SimulationConverge::new(marketplace);
        
        // Run simulation loop with pacing adjustments (maximum 100 iterations)
        let (_final_simulation_run, final_stats, final_campaign_converge_params, final_seller_converge_params) = simulation_converge.run(100, "test", &mut logger);
        logln!(&mut logger, LogEvent::Variant, "\n=== Final Results ===");
        final_stats.printout(&simulation_converge.marketplace.campaigns, &simulation_converge.marketplace.sellers, &final_campaign_converge_params, &final_seller_converge_params, &mut logger);
    }
}
