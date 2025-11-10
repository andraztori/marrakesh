use crate::types::Marketplace;
use crate::campaigns::CampaignTrait;
use crate::simulationrun::{CampaignConvergeParams, SellerConvergeParams, SimulationRun, SimulationStat};
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::warnln;
use crate::utils::ControllerProportional;

/// Object for running simulation convergence with pacing adjustments
pub struct SimulationConverge;

impl SimulationConverge {
    /// Run simulation loop with pacing adjustments (maximum max_iterations iterations)
    /// 
    /// # Arguments
    /// * `variant_name` - Name of the variant being run
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns a tuple of (final SimulationRun, final SimulationStat, final CampaignConvergeParams)
    pub fn run(
        marketplace: &Marketplace,
        initial_campaign_converge_params: &CampaignConvergeParams,
        seller_converge_params: &SellerConvergeParams,
        max_iterations: usize,
        variant_name: &str,
        logger: &mut Logger,
    ) -> (SimulationRun, SimulationStat, CampaignConvergeParams) {
        
        let mut final_simulation_run = None;
        let mut final_stats = None;
        let mut final_campaign_converge_params = None;
        let mut converged = false;
        
        // Initialize current campaign converge params from input for the first iteration
        let mut current_campaign_converge_params = initial_campaign_converge_params.clone();
        
        for iteration in 0..max_iterations {
            logln!(logger, LogEvent::Simulation, "\n=== {} - Iteration {} ===", variant_name, iteration + 1);
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(marketplace, &current_campaign_converge_params, seller_converge_params);

            // Generate statistics
            let stats = SimulationStat::new(marketplace, &simulation_run);
            
            // Calculate next iteration's campaign converge params based on current results
            let controller = ControllerProportional::new();
            let mut next_campaign_converge_params = current_campaign_converge_params.clone();
            let mut pacing_changed = false;
            for (index, campaign) in marketplace.campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let current_converge = current_campaign_converge_params.params[index].as_ref();
                let next_converge = next_campaign_converge_params.params[index].as_mut();
                
                // Use the campaign's converge_iteration method (now part of CampaignTrait)
                pacing_changed |= campaign.converge_iteration(current_converge, next_converge, campaign_stat, &controller);
            }
            
            // Output campaign statistics for each iteration (using the params that were actually used)
            stats.printout_campaigns(&marketplace.campaigns, &current_campaign_converge_params, logger, LogEvent::Simulation);
            
            // Keep track of final simulation run and stats
            final_simulation_run = Some(simulation_run);
            final_stats = Some(stats);
            final_campaign_converge_params = Some(current_campaign_converge_params);
            
            // Break early if no pacing changes were made (converged)
            if !pacing_changed {
                converged = true;
                logln!(logger, LogEvent::Convergence, "{}: Converged after {} iterations", variant_name, iteration + 1);
                break;
            }
            
            // Prepare for next iteration
            current_campaign_converge_params = next_campaign_converge_params;
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
}

