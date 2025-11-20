use crate::simulationrun::{Marketplace, SimulationRun, SimulationStat};
use crate::campaigns::Campaigns;
use crate::sellers::Sellers;
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::warnln;
use std::path::PathBuf;
use crate::utils::VERBOSE_AUCTION;
use std::sync::atomic::Ordering;
pub use crate::controllers::ControllerState;

/// Trait for campaign convergence strategies
pub trait ConvergeTargetAny<T> {
    /// Get the actual and target values for convergence
    /// 
    /// # Arguments
    /// * `stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// A tuple `(actual, target)` representing the actual value achieved and the target value
    fn get_actual_and_target(&self, stat: &T) -> (f64, f64);
    
    /// Get a string representation of the convergence target
    fn converge_target_string(&self) -> String;
}

/// Container for campaign convergence parameters
/// Uses dynamic dispatch to support different campaign types
pub struct CampaignConverges {
    pub campaign_converges: Vec<Box<dyn ControllerState>>,
}

impl Clone for CampaignConverges {
    fn clone(&self) -> Self {
        Self {
            campaign_converges: self.campaign_converges.iter().map(|p| p.clone_box()).collect(),
        }
    }
}

impl CampaignConverges {
    /// Create campaign converges from campaigns
    pub fn new(campaigns: &Campaigns) -> Self {
        let mut campaign_converges = Vec::with_capacity(campaigns.campaigns.len());
        for campaign in &campaigns.campaigns {
            campaign_converges.push(campaign.create_controller_state());
        }
        Self { campaign_converges }
    }
}

/// Container for seller convergence parameters
/// Uses dynamic dispatch to support different seller types
pub struct SellerConverges {
    pub seller_converges: Vec<Box<dyn ControllerState>>,
}

impl Clone for SellerConverges {
    fn clone(&self) -> Self {
        Self {
            seller_converges: self.seller_converges.iter().map(|p| p.clone_box()).collect(),
        }
    }
}

impl SellerConverges {
    /// Create seller converges from sellers
    pub fn new(sellers: &Sellers) -> Self {
        let mut seller_converges = Vec::with_capacity(sellers.sellers.len());
        for seller in &sellers.sellers {
            seller_converges.push(seller.create_controller_state());
        }
        Self { seller_converges }
    }
}

/// Object for running simulation convergence with pacing adjustments
pub struct SimulationConverge {
    pub marketplace: Marketplace,
    pub initial_campaign_converges: CampaignConverges,
    pub initial_seller_converges: SellerConverges,
}

impl SimulationConverge {
    /// Create a new SimulationConverge instance
    /// 
    /// # Arguments
    /// * `marketplace` - The marketplace containing campaigns, sellers, and impressions
    /// 
    /// Initializes campaign and seller convergence parameters internally
    pub fn new(marketplace: Marketplace) -> Self {
        let initial_campaign_converges = CampaignConverges::new(&marketplace.campaigns);
        let initial_seller_converges = SellerConverges::new(&marketplace.sellers);
        
        Self {
            marketplace,
            initial_campaign_converges,
            initial_seller_converges,
        }
    }
    
    /// Run simulation loop with pacing adjustments (maximum max_iterations iterations)
    /// 
    /// # Arguments
    /// * `max_iterations` - Maximum number of iterations to run
    /// * `scenario_name` - Name of the scenario (for log file paths)
    /// * `variant_name` - Name of the variant being run
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns a tuple of (final SimulationRun, final SimulationStat, final CampaignConverges, final SellerConverges)
    pub fn run(
        &self,
        max_iterations: usize,
        scenario_name: &str,
        variant_name: &str,
        logger: &mut Logger,
    ) -> (SimulationRun, SimulationStat, CampaignConverges, SellerConverges) {
        
        let mut final_simulation_run = None;
        let mut final_stats = None;
        let mut final_campaign_converges = None;
        let mut final_seller_converges = None;
        let mut converged = false;
        
        // Initialize current campaign converges from input for the first iteration
        let mut current_campaign_converges = self.initial_campaign_converges.clone();
        // Initialize current seller converges from input for the first iteration
        let mut current_seller_converges = self.initial_seller_converges.clone();
        
        for iteration in 0..max_iterations {
            logln!(logger, LogEvent::Simulation, "\n=== {} - Iteration {} ===", variant_name, iteration + 1);
            
            // Create auction receiver for this iteration
            let auctions_receiver_id = if VERBOSE_AUCTION.load(Ordering::Relaxed) {
                let receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/auctions-{}-iter{}.csv", sanitize_filename(scenario_name), sanitize_filename(variant_name), iteration + 1)), vec![LogEvent::Auction]));
                
                // Write CSV header
                let mut header_fields = vec![
                    "seller_id".to_string(),
                    "campaign_id".to_string(),
                    "winning_bid".to_string(),
                    "floor_cpm".to_string(),
                    "impression_base_value".to_string(),
                    "competing_bid".to_string(),
                    "win_rate_actual_sigmoid_offset".to_string(),
                    "win_rate_actual_sigmoid_scale".to_string(),
                ];
                
                // Add campaign columns
                for campaign_id in 0..self.marketplace.campaigns.campaigns.len() {
                    header_fields.push(format!("campaign_{}_value", campaign_id));
                    header_fields.push(format!("campaign_{}_bid", campaign_id));
                }
                
                logln!(logger, LogEvent::Auction, "{}", header_fields.join(","));
                
                Some(receiver_id)
            } else {
                None
            };
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(&self.marketplace, &current_campaign_converges, &current_seller_converges, logger);
            
            // Remove auction receiver after this iteration
            if let Some(id) = auctions_receiver_id {
                logger.remove_receiver(id);
            }

            // Generate statistics (use iteration + 1 for 1-indexed iteration count)
            let stats = SimulationStat::new(&self.marketplace, &simulation_run, iteration + 1);
            
            // Calculate next iteration's campaign converges based on current results
            let mut next_campaign_converges = current_campaign_converges.clone();
            let mut pacing_changed = false;
            for (index, campaign) in self.marketplace.campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let previous_state = current_campaign_converges.campaign_converges[index].as_ref();
                let next_state = next_campaign_converges.campaign_converges[index].as_mut();
                
                // Use the campaign's next_controller_state method (now part of CampaignTrait)
                pacing_changed |= campaign.next_controller_state(previous_state, next_state, campaign_stat);
            }
            
            // Calculate next iteration's seller converges based on current results
            let mut next_seller_converges = current_seller_converges.clone();
            let mut boost_changed = false;
            for (index, seller) in self.marketplace.sellers.sellers.iter().enumerate() {
                let seller_stat = &stats.seller_stats[index];
                let previous_state = current_seller_converges.seller_converges[index].as_ref();
                let next_state = next_seller_converges.seller_converges[index].as_mut();
                
                // Use the seller's next_controller_state method
                boost_changed |= seller.next_controller_state(previous_state, next_state, seller_stat);
            }
            
            // Output campaign statistics for each iteration (using the converges that were actually used)
            stats.printout_campaigns(&self.marketplace.campaigns, &current_campaign_converges, logger, LogEvent::Simulation);
            
            // Output seller statistics for each iteration (using the converges that were actually used)
            stats.printout_sellers(&self.marketplace.sellers, &current_seller_converges, logger, LogEvent::Simulation);
            
            // Keep track of final simulation run and stats
                final_simulation_run = Some(simulation_run);
                final_stats = Some(stats);
            final_campaign_converges = Some(current_campaign_converges.clone());
            final_seller_converges = Some(current_seller_converges.clone());
            
            // Break early if no pacing or boost changes were made (converged)
            if !pacing_changed && !boost_changed {
                converged = true;
                logln!(logger, LogEvent::Convergence, "{}: Converged after {} iterations", variant_name, iteration + 1);
                break;
            }
            
            // Prepare for next iteration
            current_campaign_converges = next_campaign_converges;
            current_seller_converges = next_seller_converges;
        }
        
        // Log if we reached max iterations
        if !converged {
            warnln!(logger, LogEvent::Convergence, "{}: Reached maximum iterations ({})", variant_name, max_iterations);
        }
        
        
        // Return the final simulation run, stats, and converges
        (
            final_simulation_run.expect("Should have at least one iteration"),
            final_stats.expect("Should have at least one iteration"),
            final_campaign_converges.expect("Should have at least one iteration"),
            final_seller_converges.expect("Should have at least one iteration"),
        )
    }
    
    /// Run simulation variant with logging setup and cleanup
    /// 
    /// # Arguments
    /// * `variant_description` - Description of the variant being run
    /// * `scenario_name` - Name of the scenario (for log file paths)
    /// * `variant_name` - Name of the variant (for log file paths)
    /// * `max_iterations` - Maximum number of iterations to run
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns the final SimulationStat
    pub fn run_variant(
        &self,
        variant_description: &str,
        scenario_name: &str,
        variant_name: &str,
        max_iterations: usize,
        logger: &mut Logger,
    ) -> SimulationStat {
        // Add variant iterations receiver (for simulation and convergence events)
        let iterations_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/iterations-{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Simulation, LogEvent::Convergence]));
        
        // Add variant receiver (for variant events)
        let variant_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/variant-{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Variant]));
        
        // Add impressions receiver (for logging impression data)
    //    let impressions_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/imps-{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Impression]));
        
        logln!(logger, LogEvent::Variant, "\n=== {} ===", variant_description);
        
        self.marketplace.printout(logger);
        
        // Log impression data
        /*
        for impression in &self.marketplace.impressions.impressions {
            if let Some(comp) = &impression.competition {
                logln!(logger, LogEvent::Impression,
                    "base_value={:.4}, campaign_0_value={:.4}, floor={:.4}, comp_bid={:.4}, pred_offset={:.4}, pred_scale={:.4}, actual_offset={:.4}, actual_scale={:.4}",
                    impression.base_impression_value,
                    impression.value_to_campaign_id[0],
                    impression.floor_cpm,
                    comp.bid_cpm,
                    comp.win_rate_prediction_sigmoid_offset,
                    comp.win_rate_prediction_sigmoid_scale,
                    comp.win_rate_actual_sigmoid_offset,
                    comp.win_rate_actual_sigmoid_scale
                );
            } else {
                logln!(logger, LogEvent::Impression,
                    "base_value={:.4}, campaign_0_value={:.4}, floor={:.4}, no_competition",
                    impression.base_impression_value,
                    impression.value_to_campaign_id[0],
                    impression.floor_cpm
                );
            }
        }
        */
        // Run simulation loop with pacing adjustments
        let (_final_simulation_run, stats, final_campaign_converges, final_seller_converges) = self.run(max_iterations, scenario_name, variant_name, logger);
        
        // Print final stats (variant-level output)
        stats.printout(&self.marketplace.campaigns, &self.marketplace.sellers, &final_campaign_converges, &final_seller_converges, logger);
        
        // Remove variant-specific receivers
//        logger.remove_receiver(impressions_receiver_id);
        logger.remove_receiver(variant_receiver_id);
        logger.remove_receiver(iterations_receiver_id);
        
        stats
    }
}

