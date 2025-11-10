use crate::types::{CampaignType, Marketplace};
use crate::simulationrun::{CampaignParams, SellerParams, SimulationRun, SimulationStat};
use crate::logger::{Logger, LogEvent};
use crate::logln;

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
            proportional_gain: 0.2,      // 20% of error
        }
    }

    /// Adjust pacing based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `current_pacing` - Current pacing value
    /// 
    /// # Returns
    /// Tuple of (new_pacing, changed) where changed indicates if pacing was modified
    pub fn adjust_pacing(&self, target: f64, actual: f64, current_pacing: f64) -> (f64, bool) {
        let tolerance = target * self.tolerance_fraction;
        
        if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 + adjustment_factor);
            (new_pacing, true)
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 - adjustment_factor);
            (new_pacing, true)
        } else {
            // Within tolerance - keep constant
            (current_pacing, false)
        }
    }
}

/// Object for running simulation convergence with pacing adjustments
pub struct SimulationConverge;

impl SimulationConverge {
    /// Run simulation loop with pacing adjustments (maximum max_iterations iterations)
    /// 
    /// # Arguments
    /// * `logger` - Logger for event-based logging
    /// 
    /// # Returns
    /// Returns a tuple of (final SimulationRun, final SimulationStat, final CampaignParams)
    pub fn run(
        marketplace: &Marketplace,
        campaign_params: &CampaignParams,
        seller_params: &SellerParams,
        max_iterations: usize,
        logger: &mut Logger,
    ) -> (SimulationRun, SimulationStat, CampaignParams) {
        
        let mut final_simulation_run = None;
        let mut final_stats = None;
        let mut final_campaign_params = None;
        let mut converged = false;
        
        // Start with a clone of the input campaign_params
        let mut current_campaign_params = campaign_params.clone();
        
        for iteration in 0..max_iterations {
            logln!(logger, LogEvent::Simulation, "\n=== Iteration {} ===", iteration + 1);
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(marketplace, &current_campaign_params, seller_params);

            // Generate statistics
            let stats = SimulationStat::new(marketplace, &simulation_run);
            
            // Adjust pacing for each campaign based on targets using proportional controller
            let controller = ControllerProportional::new();
            let mut pacing_changed = false;
            for (index, campaign) in marketplace.campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let current_pacing = current_campaign_params.params[index].pacing;
                
                // Adjust pacing based on campaign type using controller
                let (new_pacing, changed) = match &campaign.campaign_type {
                    CampaignType::FIXED_IMPRESSIONS { total_impressions_target } => {
                        let target = *total_impressions_target as f64;
                        let actual = campaign_stat.impressions_obtained as f64;
                        controller.adjust_pacing(target, actual, current_pacing)
                    }
                    CampaignType::FIXED_BUDGET { total_budget_target } => {
                        let target = *total_budget_target;
                        let actual = campaign_stat.total_buyer_charge;
                        controller.adjust_pacing(target, actual, current_pacing)
                    }
                };
                
                current_campaign_params.params[index].pacing = new_pacing;
                if changed {
                    pacing_changed = true;
                }
            }
            
            // Output campaign statistics for each iteration
            stats.printout_campaigns(&marketplace.campaigns, &current_campaign_params, logger, LogEvent::Simulation);
            
            // Break early if no pacing changes were made (converged)
            if !pacing_changed {
                final_simulation_run = Some(simulation_run);
                final_stats = Some(stats);
                final_campaign_params = Some(current_campaign_params);
                converged = true;
                logln!(logger, LogEvent::Convergence, "Converged after {} iterations", iteration + 1);
                break;
            }
            
            // Keep track of final simulation run and stats in case we reach max_iterations
            final_simulation_run = Some(simulation_run);
            final_stats = Some(stats);
            final_campaign_params = Some(current_campaign_params.clone());
        }
        
        // Log if we reached max iterations
        if !converged {
            logln!(logger, LogEvent::Convergence, "Reached maximum iterations ({})", max_iterations);
        }
        
        // Return the final simulation run, stats, and campaign params
        (
            final_simulation_run.expect("Should have at least one iteration"),
            final_stats.expect("Should have at least one iteration"),
            final_campaign_params.expect("Should have at least one iteration"),
        )
    }
}

