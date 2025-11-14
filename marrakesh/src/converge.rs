use crate::simulationrun::{Marketplace, CampaignConverges, SellerConverges, SimulationRun, SimulationStat};
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::warnln;
use std::path::PathBuf;

/// Unified trait for convergence parameters
/// Used for both campaigns and sellers
pub trait Converge: std::any::Any {
    /// Clone the convergence parameter
    fn clone_box(&self) -> Box<dyn Converge>;
    
    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get a mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Unified convergence parameter for both campaigns and sellers
#[derive(Clone)]
pub struct ConvergingParam {
    pub converging_param: f64,
}

impl Converge for ConvergingParam {
    fn clone_box(&self) -> Box<dyn Converge> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
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
    /// * `variant_name` - Name of the variant being run
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns a tuple of (final SimulationRun, final SimulationStat, final CampaignConverges, final SellerConverges)
    pub fn run(
        &self,
        max_iterations: usize,
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
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(&self.marketplace, &current_campaign_converges, &current_seller_converges, logger);

            // Generate statistics
            let stats = SimulationStat::new(&self.marketplace, &simulation_run);
            
            // Calculate next iteration's campaign converges based on current results
            let mut next_campaign_converges = current_campaign_converges.clone();
            let mut pacing_changed = false;
            for (index, campaign) in self.marketplace.campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let current_converge = current_campaign_converges.campaign_converges[index].as_ref();
                let next_converge = next_campaign_converges.campaign_converges[index].as_mut();
                
                // Use the campaign's converge_iteration method (now part of CampaignTrait)
                pacing_changed |= campaign.converge_iteration(current_converge, next_converge, campaign_stat);
            }
            
            // Calculate next iteration's seller converges based on current results
            let mut next_seller_converges = current_seller_converges.clone();
            let mut boost_changed = false;
            for (index, seller) in self.marketplace.sellers.sellers.iter().enumerate() {
                let seller_stat = &stats.seller_stats[index];
                let current_converge = current_seller_converges.seller_converges[index].as_ref();
                let next_converge = next_seller_converges.seller_converges[index].as_mut();
                
                // Use the seller's converge_iteration method
                boost_changed |= seller.converge_iteration(current_converge, next_converge, seller_stat);
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
        let iterations_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}_iterations.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Simulation, LogEvent::Convergence]));
        
        // Add variant receiver (for variant events)
        let variant_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Variant]));
        
        logln!(logger, LogEvent::Variant, "\n=== {} ===", variant_description);
        
        self.marketplace.printout(logger);
        
        // Run simulation loop with pacing adjustments
        let (_final_simulation_run, stats, final_campaign_converges, final_seller_converges) = self.run(max_iterations, variant_name, logger);
        
        // Print final stats (variant-level output)
        stats.printout(&self.marketplace.campaigns, &self.marketplace.sellers, &final_campaign_converges, &final_seller_converges, logger);
        
        // Remove variant-specific receivers
        logger.remove_receiver(variant_receiver_id);
        logger.remove_receiver(iterations_receiver_id);
        
        stats
    }
}

