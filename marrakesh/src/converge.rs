use crate::types::Marketplace;
use crate::simulationrun::{CampaignConvergeParams, SellerConvergeParams, SimulationRun, SimulationStat};
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::warnln;
use std::path::PathBuf;

/// Object for running simulation convergence with pacing adjustments
pub struct SimulationConverge {
    pub marketplace: Marketplace,
    pub initial_campaign_converge_params: CampaignConvergeParams,
    pub initial_seller_converge_params: SellerConvergeParams,
}

impl SimulationConverge {
    /// Create a new SimulationConverge instance
    /// 
    /// # Arguments
    /// * `marketplace` - The marketplace containing campaigns, sellers, and impressions
    /// 
    /// Initializes campaign and seller convergence parameters internally
    pub fn new(marketplace: Marketplace) -> Self {
        let initial_campaign_converge_params = CampaignConvergeParams::new(&marketplace.campaigns);
        let initial_seller_converge_params = SellerConvergeParams::new(&marketplace.sellers);
        
        Self {
            marketplace,
            initial_campaign_converge_params,
            initial_seller_converge_params,
        }
    }
    
    /// Run simulation loop with pacing adjustments (maximum max_iterations iterations)
    /// 
    /// # Arguments
    /// * `max_iterations` - Maximum number of iterations to run
    /// * `variant_name` - Name of the variant being run
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns a tuple of (final SimulationRun, final SimulationStat, final CampaignConvergeParams)
    pub fn run(
        &self,
        max_iterations: usize,
        variant_name: &str,
        logger: &mut Logger,
    ) -> (SimulationRun, SimulationStat, CampaignConvergeParams) {
        
        let mut final_simulation_run = None;
        let mut final_stats = None;
        let mut final_campaign_converge_params = None;
        let mut converged = false;
        
        // Initialize current campaign converge params from input for the first iteration
        let mut current_campaign_converge_params = self.initial_campaign_converge_params.clone();
        // Initialize current seller converge params from input for the first iteration
        let mut current_seller_converge_params = self.initial_seller_converge_params.clone();
        
        for iteration in 0..max_iterations {
            logln!(logger, LogEvent::Simulation, "\n=== {} - Iteration {} ===", variant_name, iteration + 1);
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(&self.marketplace, &current_campaign_converge_params, &current_seller_converge_params);

            // Generate statistics
            let stats = SimulationStat::new(&self.marketplace, &simulation_run);
            
            // Calculate next iteration's campaign converge params based on current results
            let mut next_campaign_converge_params = current_campaign_converge_params.clone();
            let mut pacing_changed = false;
            for (index, campaign) in self.marketplace.campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let current_converge = current_campaign_converge_params.params[index].as_ref();
                let next_converge = next_campaign_converge_params.params[index].as_mut();
                
                // Use the campaign's converge_iteration method (now part of CampaignTrait)
                pacing_changed |= campaign.converge_iteration(current_converge, next_converge, campaign_stat);
            }
            
            // Calculate next iteration's seller converge params based on current results
            let mut next_seller_converge_params = current_seller_converge_params.clone();
            let mut boost_changed = false;
            for (index, seller) in self.marketplace.sellers.sellers.iter().enumerate() {
                let seller_stat = &stats.seller_stats[index];
                let current_converge = current_seller_converge_params.params[index].as_ref();
                let next_converge = next_seller_converge_params.params[index].as_mut();
                
                // Use the seller's converge_iteration method
                boost_changed |= seller.converge_iteration(current_converge, next_converge, seller_stat);
            }
            
            // Output campaign statistics for each iteration (using the params that were actually used)
            stats.printout_campaigns(&self.marketplace.campaigns, &current_campaign_converge_params, logger, LogEvent::Simulation);
            
            // Output seller statistics for each iteration (using the params that were actually used)
            stats.printout_sellers(&self.marketplace.sellers, &current_seller_converge_params, logger, LogEvent::Simulation);
            
            // Keep track of final simulation run and stats
            final_simulation_run = Some(simulation_run);
            final_stats = Some(stats);
            final_campaign_converge_params = Some(current_campaign_converge_params);
            
            // Break early if no pacing or boost changes were made (converged)
            if !pacing_changed && !boost_changed {
                converged = true;
                logln!(logger, LogEvent::Convergence, "{}: Converged after {} iterations", variant_name, iteration + 1);
                break;
            }
            
            // Prepare for next iteration
            current_campaign_converge_params = next_campaign_converge_params;
            current_seller_converge_params = next_seller_converge_params;
        }
        
        // Log if we reached max iterations
        if !converged {
            warnln!(logger, LogEvent::Convergence, "{}: Reached maximum iterations ({})", variant_name, max_iterations);
        }
        
        // Return the final simulation run, stats, and campaign converge params
        (
            final_simulation_run.expect("Should have at least one iteration"),
            final_stats.expect("Should have at least one iteration"),
            final_campaign_converge_params.expect("Should have at least one iteration"),
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
        let iterations_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}_iterations.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Simulation, LogEvent::Convergence]));
        
        // Add variant receiver (for variant events)
        let variant_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Variant]));
        
        logln!(logger, LogEvent::Variant, "\n=== {} ===", variant_description);
        
        self.marketplace.printout(logger);
        
        // Run simulation loop with pacing adjustments
        let (_final_simulation_run, stats, final_campaign_converge_params) = self.run(max_iterations, variant_name, logger);
        
        // Print final stats (variant-level output)
        // Note: We use initial_seller_converge_params here since converge doesn't return final seller params
        // The seller params should be converged by this point anyway
        stats.printout(&self.marketplace.campaigns, &self.marketplace.sellers, &final_campaign_converge_params, &self.initial_seller_converge_params, logger);
        
        // Remove variant-specific receivers
        logger.remove_receiver(variant_receiver_id);
        logger.remove_receiver(iterations_receiver_id);
        
        stats
    }
}

