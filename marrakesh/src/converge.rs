use crate::simulationrun::{Marketplace, SimulationRun, SimulationStat};
use crate::campaigns::Campaigns;
use crate::sellers::Sellers;
use crate::logger::{Logger, LogEvent, FileReceiver, sanitize_filename};
use crate::logln;
use crate::warnln;
use std::path::PathBuf;

/// Proportional controller for adjusting campaign pacing based on target vs actual performance
pub struct ControllerProportional {
    tolerance_fraction: f64,      // Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    max_adjustment_factor: f64,   // Maximum adjustment factor (e.g., 0.2 = 20%)
    proportional_gain: f64,       // Proportional gain (e.g., 0.2 = 20% of error)
}

impl ControllerProportional {
    /// Create a new proportional controller with default parameters
    pub fn new() -> Self {
        Self {
            tolerance_fraction: 0.005,  // 0.5% tolerance
            max_adjustment_factor: 0.2,  // Max 20% adjustment
            proportional_gain: 0.1,      // 20% of error
        }
    }

    /// Create initial converging variables
    pub fn create_converging_variables(&self) -> Box<dyn ConvergingVariables> {
        Box::new(ConvergingSingleVariable { converging_variable: 1.0 })
    }

    /// Calculate pacing for next iteration based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `current_converge` - Current convergence parameter
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    pub fn converge_next_iteration(&self, target: f64, actual: f64, current_converge: &dyn ConvergingVariables, next_converge: &mut dyn ConvergingVariables) -> bool {
        let current_pacing = current_converge.as_any().downcast_ref::<ConvergingSingleVariable>().unwrap().converging_variable;
        let next_converge_mut = next_converge.as_any_mut().downcast_mut::<ConvergingSingleVariable>().unwrap();
        
        let tolerance = target * self.tolerance_fraction;
        
        if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 + adjustment_factor);
            next_converge_mut.converging_variable = new_pacing;
            true
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 - adjustment_factor);
            next_converge_mut.converging_variable = new_pacing;
            true
        } else {
            // Within tolerance - keep constant
            next_converge_mut.converging_variable = current_pacing;
            false
        }
    }
    
    /// Get the converging variable from the convergence parameter
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to extract the variable from
    /// 
    /// # Returns
    /// The converging variable value
    pub fn get_converging_variable(&self, converge: &dyn ConvergingVariables) -> f64 {
        converge.as_any().downcast_ref::<ConvergingSingleVariable>().unwrap().converging_variable
    }
}

/// Unified trait for convergence parameters
/// Used for both campaigns and sellers
pub trait ConvergingVariables: std::any::Any {
    /// Clone the convergence parameter
    fn clone_box(&self) -> Box<dyn ConvergingVariables>;
    
    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get a mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Unified convergence parameter for both campaigns and sellers
#[derive(Clone)]
pub struct ConvergingSingleVariable {
    pub converging_variable: f64,
}

impl ConvergingVariables for ConvergingSingleVariable {
    fn clone_box(&self) -> Box<dyn ConvergingVariables> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Trait for campaign convergence strategies
pub trait ConvergeAny<T> {
    /// Perform one iteration of convergence, updating the next convergence parameter
    /// 
    /// # Arguments
    /// * `current_converge` - Current convergence parameter
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn converge_iteration(&self, current_converge: &dyn ConvergingVariables, next_converge: &mut dyn ConvergingVariables, campaign_stat: &T) -> bool;
    
    /// Get the converging parameter (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to extract the pacing value from
    /// 
    /// Default implementation extracts the variable from ConvergingSingleVariable.
    /// Implementations can override this if they need different behavior.
    fn get_converging_variable(&self, converge: &dyn ConvergingVariables) -> f64 {
        converge.as_any().downcast_ref::<ConvergingSingleVariable>().unwrap().converging_variable
    }
    
    /// Create initial converging variables
    fn create_converging_variables(&self) -> Box<dyn ConvergingVariables>;
    
    /// Get a string representation of the convergence target and pacing
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to include pacing information
    fn converge_target_string(&self, converge: &dyn ConvergingVariables) -> String;
}

/// Container for campaign convergence parameters
/// Uses dynamic dispatch to support different campaign types
pub struct CampaignConverges {
    pub campaign_converges: Vec<Box<dyn ConvergingVariables>>,
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
            campaign_converges.push(campaign.create_converging_variables());
        }
        Self { campaign_converges }
    }
}

/// Container for seller convergence parameters
/// Uses dynamic dispatch to support different seller types
pub struct SellerConverges {
    pub seller_converges: Vec<Box<dyn ConvergingVariables>>,
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
            seller_converges.push(seller.create_converging_variables());
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
        let iterations_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/iterations-{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Simulation, LogEvent::Convergence]));
        
        // Add variant receiver (for variant events)
        let variant_receiver_id = logger.add_receiver(FileReceiver::new(&PathBuf::from(format!("log/{}/variant-{}.log", sanitize_filename(scenario_name), sanitize_filename(variant_name))), vec![LogEvent::Variant]));
        
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

