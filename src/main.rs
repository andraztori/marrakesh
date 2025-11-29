
#[allow(unused_imports)]
mod simulationrun;
mod converge;
mod utils;
mod impressions;
mod campaign;
mod campaigns;
mod campaign_targets;
mod campaign_bidders_single;
mod campaign_bidders_double;
mod seller;
mod sellers;
mod seller_targets;
mod seller_chargers;
mod scenarios;
mod logger;
mod charts;
mod floors;
mod competition;
mod sigmoid;
mod controller_state;
mod controller_core;
mod controllers;


use sellers::{SellerType, SellerConvergeStrategy, Sellers};
use campaigns::{CampaignType, ConvergeTarget, Campaigns};
use converge::SimulationConverge;
use competition::{CompetitionGeneratorLogNormal, CompetitionGeneratorNone};
use floors::{FloorGeneratorFixed, FloorGeneratorLogNormal};
use logger::{Logger, LogEvent, ConsoleReceiver, FileReceiver, sanitize_filename};
use std::path::PathBuf;

use scenarios::get_scenario_catalog;
use utils::{RAND_SEED, TOTAL_SIMULATION_RUNS};
use std::sync::atomic::Ordering;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    
    // Parse and filter out --verbose and --fastbreak arguments
    let mut args = Vec::new();
    let mut skip_next = false;
    let mut fastbreak = false;
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
        if arg == "--fastbreak" {
            fastbreak = true;
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
        use campaigns::{CampaignGeneral, CampaignTargetNone, CampaignTrait};
        use impressions::Impression;
        use competition::ImpressionCompetition;
        
        // Setup shared resources
        use campaigns::{CampaignBidderOptimal, BidderMaxMargin};
        let bidder_optimal = Box::new(CampaignBidderOptimal) as Box<dyn campaign::CampaignBidderTrait>;
        let campaign_optimal = CampaignGeneral {
            campaign_id: 0,
            campaign_name: "Optimal".to_string(),
            converge_targets: vec![Box::new(CampaignTargetNone)],
            converge_controllers: vec![Box::new(crate::controllers::ControllerConstant::new(0.8298))],
            bidder: bidder_optimal,
        };
        
        let bidder_max_margin = Box::new(BidderMaxMargin) as Box<dyn campaign::CampaignBidderTrait>;
        let campaign_max_margin = CampaignGeneral {
            campaign_id: 0,
            campaign_name: "MaxMargin".to_string(),
            converge_targets: vec![Box::new(CampaignTargetNone)],
            converge_controllers: vec![Box::new(crate::controllers::ControllerConstant::new(0.8298))],
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
            
            let value_to_campaign_group = vec![test_case.value];
            
            let impression = Impression {
                seller_id: 0,
                competition: Some(ImpressionCompetition {
                    bid_cpm: test_case.bid_cpm,
                    win_rate_actual_sigmoid_offset: test_case.sigmoid_offset,
                    win_rate_actual_sigmoid_scale: test_case.sigmoid_scale,
                    win_rate_prediction_sigmoid_offset: test_case.sigmoid_offset,
                    win_rate_prediction_sigmoid_scale: test_case.sigmoid_scale,
                }),
                floor_cpm: test_case.floor_cpm,
                value_to_campaign_group,
                base_impression_value: test_case.value,
            };
            
            println!("{}: {:#?}", test_case.name, impression);
            
            let controller_states: Vec<&dyn campaigns::ControllerStateTrait> = converge_vars.iter().map(|cs| cs.as_ref()).collect();
            let bid_optimal = campaign_optimal.get_bid(&impression, &controller_states, 1.0, test_case.value, &mut logger);
            let bid_max_margin = campaign_max_margin.get_bid(&impression, &controller_states, 1.0, test_case.value, &mut logger);
            
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
        
        // Parse optional starting iteration index if present
        let start_iteration = if args.len() > 3 {
            match args[3].parse::<u64>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Error: Invalid start iteration parameter '{}'. Expected a number.", args[3]);
                    std::process::exit(1);
                }
            }
        } else {
            0
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
        // When running a specific scenario (not "all") with single iteration, also enable Scenario logging to show individual validations
        // When running multiple iterations, suppress Scenario logging to avoid cluttering output
        let mut logger = Logger::new();
        if scenario_arg == "all" {
            logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation]));
        } else {
            // Only show Scenario events on console for single iteration runs
            if iterations == 1 {
                logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation, LogEvent::Scenario]));
            } else {
                logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Validation]));
            }
        }
        
        // Add validation receiver (for validation events)
        let summary_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from("log/summary.log"), vec![LogEvent::Validation]));
        
        // Reset and log initial simulation run count
        TOTAL_SIMULATION_RUNS.store(0, Ordering::Relaxed);
        let initial_count = TOTAL_SIMULATION_RUNS.load(Ordering::Relaxed);
        
        // Log appropriate message
        if scenario_arg == "all" {
            if iterations > 1 {
                logln!(&mut logger, LogEvent::Validation, "Running all scenarios {} times... (Total simulation runs: {})\n", iterations, initial_count);
            } else {
                logln!(&mut logger, LogEvent::Validation, "Running all scenarios... (Total simulation runs: {})\n", initial_count);
            }
        } else {
            if iterations > 1 {
                logln!(&mut logger, LogEvent::Validation, "Running scenario '{}' {} times... (Total simulation runs: {})\n", scenario_arg, iterations, initial_count);
            } else {
                logln!(&mut logger, LogEvent::Validation, "Running scenario '{}'... (Total simulation runs: {})\n", scenario_arg, initial_count);
            }
        }
        
        // Outer loop for scenarios
        'scenarios: for scenario in &scenarios {
            log!(&mut logger, LogEvent::Validation, "{}: ", scenario.short_name);
            
            // Add scenario-level receiver
            let scenario_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/scenario.log", sanitize_filename(scenario.short_name))), vec![LogEvent::Scenario]));
            
            // Inner loop for iterations
            for i in start_iteration..(start_iteration + iterations) {
                if iterations > 1 {
                    let iteration_num = i - start_iteration + 1;
                    log!(&mut logger, LogEvent::Validation, "[{}/{}] ", iteration_num, iterations);
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
                        
                        // If fastbreak is enabled, stop immediately on first failure
                        if fastbreak {
                            // Remove scenario-level receiver before breaking
                            logger.remove_receiver(scenario_receiver_id);
                            logln!(&mut logger, LogEvent::Validation, "\nStopping scenario execution due to failure (--fastbreak enabled)");
                            // Always log the full error message when fastbreak stops execution
                            if iterations > 1 {
                                let iteration_num = i - start_iteration + 1;
                                logln!(&mut logger, LogEvent::Validation, "Error at iteration {}/{} (seed {}): {}", iteration_num, iterations, i, e);
                            } else {
                                logln!(&mut logger, LogEvent::Validation, "Error: {}", e);
                            }
                            break 'scenarios;
                        }
                    }
                }
                
                // Flush to ensure validation is written to summary.log
                let _ = logger.flush();
            }
            
            // Remove scenario-level receiver
            logger.remove_receiver(scenario_receiver_id);
        }
        
        // Log final simulation run count
        let final_count = TOTAL_SIMULATION_RUNS.load(Ordering::Relaxed);
        logln!(&mut logger, LogEvent::Validation, "\nTotal simulation runs completed: {}", final_count);
        
        // Remove validation receiver
        logger.remove_receiver(summary_receiver_id);
    } else {
        // Default behavior: Run the first scenario (or mrg_boost) with summary verbosity
        // For now, default to mrg_boost, but could be made configurable
        let mut logger = Logger::new();
        logger.add_receiver(ConsoleReceiver::new(vec![LogEvent::Simulation, LogEvent::Convergence, LogEvent::Variant]));
        if let Err(e) = scenarios::supply_simple_boost::run("MRGboost", &mut logger) {
            eprintln!("Error running scenario: {}", e);
            std::process::exit(1);
        }
    }
    
}
