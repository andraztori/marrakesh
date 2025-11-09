use crate::types::{CampaignType, Campaigns, Sellers};
use crate::simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use crate::impressions::Impressions;
use crate::scenarios::Verbosity;

/// Object for running simulation convergence with pacing adjustments
pub struct SimulationConverge;

impl SimulationConverge {
    /// Run simulation loop with pacing adjustments (maximum max_iterations iterations)
    /// 
    /// # Arguments
    /// * `verbosity` - Controls output level: None (no output), Summary (final stats only), Full (all iterations)
    pub fn run(
        impressions: &Impressions,
        campaigns: &Campaigns,
        sellers: &Sellers,
        campaign_params: &mut CampaignParams,
        max_iterations: usize,
        verbosity: Verbosity,
    ) {
        let mut final_stats = None;
        let mut converged = false;
        
        for iteration in 0..max_iterations {
            if verbosity == Verbosity::Full {
                println!("\n=== Iteration {} ===", iteration + 1);
            }
            
            // Run auctions for all impressions
            let simulation_run = SimulationRun::new(impressions, campaigns, campaign_params);

            // Generate statistics
            let stats = SimulationStat::new(campaigns, sellers, impressions, &simulation_run);
            
            // Adjust pacing for each campaign based on targets
            // Use adaptive adjustment that reduces as we get closer to target
            let mut pacing_changed = false;
            for (index, campaign) in campaigns.campaigns.iter().enumerate() {
                let campaign_stat = &stats.campaign_stats[index];
                let pacing = &mut campaign_params.params[index].pacing;
                
                // Get target and actual values based on campaign type
                let (target, actual) = match &campaign.campaign_type {
                    CampaignType::FIXED_IMPRESSIONS { total_impressions_target } => {
                        (*total_impressions_target as f64, campaign_stat.impressions_obtained as f64)
                    }
                    CampaignType::FIXED_BUDGET { total_budget_target } => {
                        (*total_budget_target, campaign_stat.total_buyer_charge)
                    }
                };
                
                let tolerance = target * 0.005; // 0.5% tolerance
                
                if actual < target - tolerance {
                    // Below target - increase pacing
                    // Calculate error percentage and use proportional adjustment
                    let error_ratio = (target - actual) / target;
                    // Use smaller adjustment when closer to target (max 10%)
                    let adjustment_factor = (error_ratio * 0.1).min(0.1);
                    *pacing *= 1.0 + adjustment_factor;
                    pacing_changed = true;
                } else if actual > target + tolerance {
                    // Above target - decrease pacing
                    // Calculate error percentage and use proportional adjustment
                    let error_ratio = (actual - target) / target;
                    // Use smaller adjustment when closer to target (max 10%)
                    let adjustment_factor = (error_ratio * 0.1).min(0.1);
                    *pacing *= 1.0 - adjustment_factor;
                    pacing_changed = true;
                }
                // If practically on goal (within 0.5%), keep constant
            }
            
            // Output campaign statistics only during iterations if full verbosity
            if verbosity == Verbosity::Full {
                stats.printout_campaigns(campaigns, campaign_params);
            }
            
            // Break early if no pacing changes were made (converged)
            if !pacing_changed {
                final_stats = Some(stats);
                converged = true;
                if verbosity == Verbosity::Full {
                    println!("Converged after {} iterations", iteration + 1);
                }
                break;
            }
            
            // Keep track of final stats in case we reach max_iterations
            final_stats = Some(stats);
        }
        
        // Print final solution only for Full verbosity
        // Summary verbosity will be handled by the caller (e.g., run_variant)
        if verbosity == Verbosity::Full {
            if let Some(stats) = final_stats {
                if !converged {
                    println!("Reached maximum iterations ({})", max_iterations);
                }
                stats.printout_campaigns(campaigns, campaign_params);
            }
        } else if !converged && verbosity == Verbosity::None {
            // Only print if we didn't converge (and verbosity is None)
            println!("Reached maximum iterations ({})", max_iterations);
        }
    }
}

