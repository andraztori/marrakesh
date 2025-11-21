
#[allow(unused_imports)]
mod simulationrun;
mod converge;
mod utils;
mod impressions;
mod campaigns;
mod campaign_targets;
mod campaign_bidders;
mod sellers;
mod scenarios;
mod logger;
mod charts;
mod floors;
mod competition;
mod sigmoid;
mod controllers;

// Include scenario files so their constructors run
mod s_one;
mod s_optimal;
mod s_mrg_boost;
mod s_mrg_dynamic_boost;
mod s_maxmargin_equality;

use sellers::{SellerType, SellerConvergeStrategy, Sellers};
use campaigns::{CampaignType, ConvergeTarget, Campaigns};
use converge::SimulationConverge;
use impressions::Impressions;
use competition::{CompetitionGeneratorLogNormal, CompetitionGeneratorNone};
use floors::{FloorGeneratorFixed, FloorGeneratorLogNormal};
use logger::{Logger, LogEvent, ConsoleReceiver, FileReceiver, sanitize_filename};
use std::path::PathBuf;

use scenarios::get_scenario_catalog;
use utils::RAND_SEED;
use std::sync::atomic::Ordering;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    
    // Parse and filter out --verbose argument
    let mut args = Vec::new();
    let mut skip_next = false;
    for (i, arg) in raw_args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--verbose" {
            if i + 1 < raw_args.len() && raw_args[i+1] == "auction" {
                utils::VERBOSE_AUCTION.store(true, Ordering::Relaxed);
                skip_next = true;
            }
            continue;
        }
        args.push(arg.clone());
    }
    
    // Check if "charts" argument is provided
    if args.len() > 1 && args[1] == "charts" {
        match charts::generate_all_histograms() {
            Ok(()) => {
                println!("All histogram generation completed successfully.");
            }
            Err(e) => {
                eprintln!("Error generating histograms: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }
    
    // Check if "sigmoid" argument is provided
    if args.len() > 1 && args[1] == "sigmoid" {
        match charts::generate_sigmoid_charts() {
            Ok(()) => {
                println!("Sigmoid charts generation completed successfully.");
            }
            Err(e) => {
                eprintln!("Error generating sigmoid charts: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }
    
    // Check if "test" argument is provided
    if args.len() > 1 && args[1] == "test" {
        use campaigns::{CampaignGeneral, ConvergeNone, CampaignTrait, MAX_CAMPAIGNS};
        use impressions::Impression;
        use competition::ImpressionCompetition;
        
        // Setup shared resources
        use campaigns::{CampaignBidderOptimal, BidderMaxMargin};
        let bidder_optimal = Box::new(CampaignBidderOptimal) as Box<dyn campaigns::CampaignBidder>;
        let campaign_optimal = CampaignGeneral {
            campaign_id: 0,
            campaign_name: "Optimal".to_string(),
            converge_target: Box::new(ConvergeNone),
            converge_controller: Box::new(crate::controllers::ConvergeControllerConstant::new(0.8298)),
            bidder: bidder_optimal,
        };
        
        let bidder_max_margin = Box::new(BidderMaxMargin) as Box<dyn campaigns::CampaignBidder>;
        let campaign_max_margin = CampaignGeneral {
            campaign_id: 0,
            campaign_name: "MaxMargin".to_string(),
            converge_target: Box::new(ConvergeNone),
            converge_controller: Box::new(crate::controllers::ConvergeControllerConstant::new(0.8298)),
            bidder: bidder_max_margin,
        };
        
        let converge_vars = campaign_optimal.create_controller_state();
        let mut logger = Logger::new();

        struct TestCase {
            name: &'static str,
            value: f64,
            bid_cpm: f64,
            floor_cpm: f64,
            sigmoid_offset: f64,
            sigmoid_scale: f64,
        }

        let test_cases = vec![
            TestCase {
                name: "Impression 1",
                value: 17.8285,
                bid_cpm: 0.0,
                floor_cpm: 8.8836,
                sigmoid_offset: 5.5722,
                sigmoid_scale: 1.5482,
            },
            TestCase {
                name: "Impression 2",
                value: 9.4124,
                bid_cpm: 8.758,
                floor_cpm: 9.0558,
                sigmoid_offset: 9.4124,
                sigmoid_scale: 1.9304,
            },
        ];

        for test_case in test_cases {
            println!("\n--- {} ---", test_case.name);
            
            let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
            value_to_campaign_id[0] = test_case.value;
            
            let impression = Impression {
                seller_id: 0,
                competition: Some(ImpressionCompetition {
                    bid_cpm: test_case.bid_cpm,
                    win_rate_prediction_sigmoid_offset: test_case.sigmoid_offset,
                    win_rate_prediction_sigmoid_scale: test_case.sigmoid_scale,
                    win_rate_actual_sigmoid_offset: test_case.sigmoid_offset,
                    win_rate_actual_sigmoid_scale: test_case.sigmoid_scale,
                }),
                floor_cpm: test_case.floor_cpm,
                value_to_campaign_id,
                base_impression_value: test_case.value,
            };
            
            println!("{}: {:#?}", test_case.name, impression);
            
            let bid_optimal = campaign_optimal.get_bid(&impression, converge_vars.as_ref(), 1.0, &mut logger);
            let bid_max_margin = campaign_max_margin.get_bid(&impression, converge_vars.as_ref(), 1.0, &mut logger);
            
            println!("Optimal Bid (pacing=0.8298): {:?}", bid_optimal);
            println!("Max Margin Bid (pacing=0.8298): {:?}", bid_max_margin);
        }
        
        return;
    }
    if args.len() > 1 {
        let scenario_arg = &args[1];
        
        // Parse iterations parameter if present
        let iterations = if args.len() > 2 {
            match args[2].parse::<u64>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Error: Invalid iterations parameter '{}'. Expected a number.", args[2]);
                    std::process::exit(1);
                }
            }
        } else {
            1
        };
        
        // Get all scenarios from the catalog
        let all_scenarios = get_scenario_catalog();
        
        // Filter scenarios: if "all", use all scenarios; otherwise filter to the named scenario
        let scenarios: Vec<_> = if scenario_arg == "all" {
            all_scenarios.clone()
        } else {
            // Find the requested scenario
            let found = all_scenarios.iter().find(|s| s.short_name == scenario_arg);
            match found {
                Some(scenario) => vec![scenario.clone()],
                None => {
                    eprintln!("Error: Scenario '{}' not found.", scenario_arg);
                    eprintln!("Available scenarios:");
                    for s in &all_scenarios {
                        eprintln!("  - {}", s.short_name);
                    }
                    std::process::exit(1);
                }
            }
        };
        
        // Set up logger with console and validation file receivers
        // When running a specific scenario (not "all"), also enable Scenario logging to show individual validations
        let mut logger = Logger::new();
        if scenario_arg == "all" {
            logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation]));
        } else {
            logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation, LogEvent::Scenario]));
        }
        
        // Add validation receiver (for validation events)
        let summary_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from("log/summary.log"), vec![LogEvent::Validation]));
        
        // Log appropriate message
        if scenario_arg == "all" {
            if iterations > 1 {
                logln!(&mut logger, LogEvent::Validation, "Running all scenarios {} times...\n", iterations);
            } else {
                logln!(&mut logger, LogEvent::Validation, "Running all scenarios...\n");
            }
        } else {
            if iterations > 1 {
                logln!(&mut logger, LogEvent::Validation, "Running scenario '{}' {} times...\n", scenario_arg, iterations);
            } else {
                logln!(&mut logger, LogEvent::Validation, "Running scenario '{}'...\n", scenario_arg);
            }
        }
        
        // Outer loop for scenarios
        for scenario in &scenarios {
            log!(&mut logger, LogEvent::Validation, "{}: ", scenario.short_name);
            
            // Add scenario-level receiver
            let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario.short_name))), vec![LogEvent::Scenario]));
            
            // Inner loop for iterations
            for i in 0..iterations {
                if iterations > 1 {
                    log!(&mut logger, LogEvent::Validation, "[{}/{}] ", i + 1, iterations);
                }
                
                // Set RAND_SEED to iteration number
                RAND_SEED.store(i, Ordering::Relaxed);
                
                match (scenario.run)(scenario.short_name, &mut logger) {
                    Ok(()) => {
                        if iterations > 1 {
                            logln!(&mut logger, LogEvent::Validation, "✓");
                        } else {
                            logln!(&mut logger, LogEvent::Validation, "✓ PASSED");
                        }
                    },
                    Err(e) => {
                        if iterations > 1 {
                            logln!(&mut logger, LogEvent::Validation, "✗");
                        } else {
                            logln!(&mut logger, LogEvent::Validation, "✗ FAILED: {}", e);
                        }
                    }
                }
                
                // Flush to ensure validation is written to summary.log
                let _ = logger.flush();
            }
            
            // Remove scenario-level receiver
            logger.remove_receiver(scenario_receiver_id);
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
            CampaignType::MULTIPLICATIVE_PACING,
            ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: 1000 },
        );

        campaigns.add(
            "Campaign 1".to_string(),  // campaign_name
            CampaignType::MULTIPLICATIVE_PACING,
            ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 },
        );

        // Add two sellers (IDs are automatically set to match Vec index)
        sellers.add(
            "MRG".to_string(),  // seller_name
            SellerType::FIXED_PRICE {
                fixed_cost_cpm: 10.0,
            },  // seller_type
            SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
            1000,  // impressions_on_offer
            CompetitionGeneratorNone::new(),  // competition_generator
            FloorGeneratorFixed::new(0.0),  // floor_generator
        );

        sellers.add(
            "HB".to_string(),  // seller_name
            SellerType::FIRST_PRICE,  // seller_type
            SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
            1000,  // impressions_on_offer
            CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
            FloorGeneratorLogNormal::new(0.2, 3.0),  // floor_generator
        );

        // Create impressions for all sellers using default parameters
        let impressions_params = impressions::ImpressionsParam::new(
            utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
            utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
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

        // Create simulation converge instance (initializes campaign and seller converges internally)
        let simulation_converge = SimulationConverge::new(marketplace);
        
        // Run simulation loop with pacing adjustments (maximum 100 iterations)
        let (_final_simulation_run, final_stats, final_campaign_controller_states, final_seller_controller_states) = simulation_converge.run(100, "main", "test", &mut logger);
        logln!(&mut logger, LogEvent::Variant, "\n=== Final Results ===");
        final_stats.printout(&simulation_converge.marketplace.campaigns, &simulation_converge.marketplace.sellers, &final_campaign_controller_states, &final_seller_controller_states, &mut logger);
    }
}
