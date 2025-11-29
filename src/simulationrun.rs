/// This file contains the SimulationRun struct, which is used to run one single iteration of simulation (which is then run multiple times by converge.rs)
/// The SimulationRun struct is used to run the simulation and collect the results.
/// 
/// We support two different auction types
/// - Standard: First price auction
/// - Fractional auction: Fractional auction where multiple impressions that bid more than "generated competition" can win the auction fractionally
///     what fraction of auction they win is handled by softmax with its temperature


use crate::impressions::{AuctionResult, FractionalAuctionResult, FractionalWinners, Winner, Impressions, ImpressionsParam};
use crate::sellers::Sellers;
use crate::campaigns::Campaigns;
use crate::converge::{CampaignControllerStates, SellerControllerStates};
use crate::logger::{Logger, LogEvent};
use crate::logln;

/// Simulation type determining the auction mechanism
#[derive(Debug, Clone, PartialEq)]
pub enum SimulationType {
    Standard,
    /// Fractional auction with temperature parameter for softmax
    /// Temperature controls the sharpness of the distribution:
    /// - Lower values (< 1.0) make the distribution sharper (more concentrated on highest bid)
    /// - Higher values (> 1.0) make the distribution smoother (more uniform)
    /// - Default: 1.0 (standard softmax)
    FractionalInternalAuction { softmax_temperature: f64 },
}

/// Marketplace containing campaigns, sellers, and impressions
/// This groups together the three main components of the marketplace simulation
pub struct Marketplace {
    pub campaigns: crate::campaigns::Campaigns,
    pub sellers: crate::sellers::Sellers,
    pub impressions: crate::impressions::Impressions,
    pub simulation_type: SimulationType,
}

impl Marketplace {
    /// Create a new Marketplace
    /// Automatically finalizes campaign groups and creates impressions
    pub fn new(mut campaigns: Campaigns, sellers: Sellers, impressions_params: &ImpressionsParam, simulation_type: SimulationType) -> Self {
        // Finalize campaign groups before creating impressions
        campaigns.finalize_groups();
        // Generally all simulations run perfectly well with fractional auctions...
        //        let simulation_type = SimulationType::FractionalInternalAuction { softmax_temperature: 0.5 };
        let impressions = Impressions::new(&sellers, impressions_params, &campaigns);
        Self {
            campaigns,
            sellers,
            impressions,
            simulation_type,
        }
    }
    
    /// Print initialization information about the marketplace
    pub fn printout(&self, logger: &mut Logger) {
        
        logln!(logger, LogEvent::Simulation, "Initialized {} sellers", self.sellers.sellers.len());
        logln!(logger, LogEvent::Simulation, "Initialized {} campaigns", self.campaigns.campaigns.len());
        logln!(logger, LogEvent::Simulation, "Initialized {} impressions", self.impressions.impressions.len());
    }
}

/// Container for auction results
/// Note: SimulationRun results are matched to Impressions by index in the vectors
pub struct SimulationRun {
    pub results: Vec<AuctionResult>,
    pub results_fractional: Vec<FractionalAuctionResult>,
}

impl SimulationRun {
    /// Create a new SimulationRun container and run auctions for all impressions
    pub fn new(marketplace: &Marketplace, campaign_controller_states: &CampaignControllerStates, seller_controller_states: &SellerControllerStates, logger: &mut Logger) -> Self {
        let mut results = Vec::with_capacity(marketplace.impressions.impressions.len());
        let mut results_fractional = Vec::with_capacity(marketplace.impressions.impressions.len());
        
        // Create campaign_converge slices for all campaigns (once, outside the loop)
        let campaign_converges: Vec<Vec<&dyn crate::controllers::ControllerStateTrait>> = campaign_controller_states.campaign_controller_states.iter()
            .map(|campaign_states_vec| campaign_states_vec.iter().map(|cs| cs.as_ref()).collect())
            .collect();
        
        for impression in &marketplace.impressions.impressions {
            // Get the seller and seller_converge for this impression
            let seller = marketplace.sellers.sellers[impression.seller_id].as_ref();
            // For sellers, we typically use the first controller state
            let seller_converge = seller_controller_states.seller_controller_states[seller.seller_id()][0].as_ref();
            
            // Check simulation type and call appropriate auction method
            match marketplace.simulation_type {
                SimulationType::Standard => {
                    let result = impression.run_auction(&marketplace.campaigns, &campaign_converges, seller, seller_converge, logger);
                    results.push(result);
                }
                SimulationType::FractionalInternalAuction { softmax_temperature } => {
                    let result_fractional = impression.run_fractional_auction(&marketplace.campaigns, &campaign_converges, seller, seller_converge, softmax_temperature, logger);
                    results_fractional.push(result_fractional);
                }
            }
        }
        
        Self { results, results_fractional }
    }
}

/// Statistics for a single campaign
pub struct CampaignStat {
    /// Number of impressions obtained (f64 to support fractional impressions in FractionalInternalAuction)
    pub impressions_obtained: f64,
    pub total_supply_cost: f64,
    pub total_virtual_cost: f64,
    pub total_buyer_charge: f64,
    pub total_value: f64,
}

/// Statistics for a single seller
pub struct SellerStat {
    pub impressions_sold: usize,
    pub total_supply_cost: f64,
    pub total_virtual_cost: f64,
    pub total_buyer_charge: f64,
    pub total_provided_value: f64,
}

/// Overall statistics for the simulation
pub struct OverallStat {
    pub lost_count: usize,
    pub no_bids_count: usize,
    pub total_supply_cost: f64,
    pub total_virtual_cost: f64,
    pub total_buyer_charge: f64,
    pub total_value: f64,
}

/// Complete simulation statistics
pub struct SimulationStat {
    pub campaign_stats: Vec<CampaignStat>,
    pub seller_stats: Vec<SellerStat>,
    pub overall_stat: OverallStat,
    pub convergence_iterations: usize,
}

impl SimulationStat {
    /// Generate statistics from marketplace and simulation run
    /// 
    /// # Arguments
    /// * `marketplace` - The marketplace containing campaigns, sellers, and impressions
    /// * `simulation_run` - The simulation run results
    /// * `convergence_iterations` - Number of iterations it took to converge (1-indexed)
    pub fn new(marketplace: &Marketplace, simulation_run: &SimulationRun, convergence_iterations: usize) -> Self {
        // Initialize campaign statistics
        let num_campaigns = marketplace.campaigns.campaigns.len();
        let mut campaign_stats: Vec<CampaignStat> = (0..num_campaigns)
            .map(|_| CampaignStat {
                impressions_obtained: 0.0,
                total_supply_cost: 0.0,
                total_virtual_cost: 0.0,
                total_buyer_charge: 0.0,
                total_value: 0.0,
            })
            .collect();

        // Initialize seller statistics
        let num_sellers = marketplace.sellers.sellers.len();
        let mut seller_stats: Vec<SellerStat> = (0..num_sellers)
            .map(|_| SellerStat {
                impressions_sold: 0,
                total_supply_cost: 0.0,
                total_virtual_cost: 0.0,
                total_buyer_charge: 0.0,
                total_provided_value: 0.0,
            })
            .collect();

        // Initialize overall statistics
        let mut overall_stat = OverallStat {
            lost_count: 0,
            no_bids_count: 0,
            total_supply_cost: 0.0,
            total_virtual_cost: 0.0,
            total_buyer_charge: 0.0,
            total_value: 0.0,
        };

        // Iterate through impressions once and accumulate all statistics
        for (index, impression) in marketplace.impressions.impressions.iter().enumerate() {
            let seller_id = impression.seller_id;

            // Condition on simulation type to handle different auction result types
            match marketplace.simulation_type {
                SimulationType::Standard => {
                    let result = &simulation_run.results[index];

                    // Update overall statistics based on winner
                    match result.winner {
                        Winner::LOST => {
                            overall_stat.lost_count += 1;
                            // Even when impression is not sold, count supply cost (0.0 for first price, fixed_cost_cpm for fixed price)
                            overall_stat.total_supply_cost += result.supply_cost;
                            // Update seller statistics
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.total_supply_cost += result.supply_cost;
                        }
                        Winner::NO_DEMAND => {
                            overall_stat.no_bids_count += 1;
                            // Even when there's no demand, count supply cost (0.0 for first price, fixed_cost_cpm for fixed price)
                            overall_stat.total_supply_cost += result.supply_cost;
                            // Update seller statistics
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.total_supply_cost += result.supply_cost;
                        }
                        Winner::Campaign { campaign_id, virtual_cost, buyer_charge, .. } => {
                            // Update overall statistics
                            overall_stat.total_supply_cost += result.supply_cost;
                            overall_stat.total_virtual_cost += virtual_cost;
                            overall_stat.total_buyer_charge += buyer_charge;
                            let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                            overall_stat.total_value += impression.value_to_campaign_group[group_id];

                            // Update seller statistics
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.impressions_sold += 1;
                            seller_stat.total_supply_cost += result.supply_cost;
                            seller_stat.total_virtual_cost += virtual_cost;
                            seller_stat.total_buyer_charge += buyer_charge;
                            let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                            seller_stat.total_provided_value += impression.value_to_campaign_group[group_id];

                            // Update campaign statistics
                            let campaign_stat = &mut campaign_stats[campaign_id];
                            campaign_stat.impressions_obtained += 1.0;
                            campaign_stat.total_supply_cost += result.supply_cost;
                            campaign_stat.total_virtual_cost += virtual_cost;
                            campaign_stat.total_buyer_charge += buyer_charge;
                            let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                            campaign_stat.total_value += impression.value_to_campaign_group[group_id];
                        }
                    }
                }
                SimulationType::FractionalInternalAuction { .. } => {
                    let result_fractional = &simulation_run.results_fractional[index];

                    // Update overall statistics based on fractional winners
                    match &result_fractional.winner {
                        FractionalWinners::LOST => {
                            overall_stat.lost_count += 1;
                            // Even when impression is not sold, count supply cost (0.0 for first price, fixed_cost_cpm for fixed price)
                            overall_stat.total_supply_cost += result_fractional.supply_cost;
                            // Update seller statistics
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.total_supply_cost += result_fractional.supply_cost;
                        }
                        FractionalWinners::NO_DEMAND => {
                            overall_stat.no_bids_count += 1;
                            // Even when there's no demand, count supply cost (0.0 for first price, fixed_cost_cpm for fixed price)
                            overall_stat.total_supply_cost += result_fractional.supply_cost;
                            // Update seller statistics
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.total_supply_cost += result_fractional.supply_cost;
                        }
                        FractionalWinners::Campaigns { winners } => {
                            // Calculate total supply cost from fractional winners (weighted by win_fraction)
                            let mut total_supply_cost = 0.0;
                            
                            // Update seller statistics once per impression (impressions_sold is usize, not fractional)
                            let seller_stat = &mut seller_stats[seller_id];
                            seller_stat.impressions_sold += 1;
                            
                            // Process each fractional winner
                            for fractional_winner in winners {
                                let campaign_id = fractional_winner.campaign_id;
                                let win_fraction = fractional_winner.win_fraction;
                                
                                // Accumulate supply cost (weighted by win_fraction)
                                total_supply_cost += fractional_winner.supply_cost * win_fraction;
                                
                                // Update overall statistics (weighted by win_fraction)
                                overall_stat.total_virtual_cost += fractional_winner.virtual_cost * win_fraction;
                                overall_stat.total_buyer_charge += fractional_winner.buyer_charge * win_fraction;
                                let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                                overall_stat.total_value += impression.value_to_campaign_group[group_id] * win_fraction;

                                // Update seller statistics (weighted by win_fraction)
                                seller_stat.total_virtual_cost += fractional_winner.virtual_cost * win_fraction;
                                seller_stat.total_buyer_charge += fractional_winner.buyer_charge * win_fraction;
                                let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                                seller_stat.total_provided_value += impression.value_to_campaign_group[group_id] * win_fraction;

                                // Update campaign statistics (weighted by win_fraction - fractional counting on buy side)
                                let campaign_stat = &mut campaign_stats[campaign_id];
                                campaign_stat.impressions_obtained += win_fraction;
                                campaign_stat.total_supply_cost += fractional_winner.supply_cost * win_fraction;
                                campaign_stat.total_virtual_cost += fractional_winner.virtual_cost * win_fraction;
                                campaign_stat.total_buyer_charge += fractional_winner.buyer_charge * win_fraction;
                                let group_id = marketplace.campaigns.campaign_to_value_group_mapping[campaign_id];
                                campaign_stat.total_value += impression.value_to_campaign_group[group_id] * win_fraction;
                            }
                            
                            // Update overall supply cost (once per impression)
                            overall_stat.total_supply_cost += total_supply_cost;
                            seller_stat.total_supply_cost += total_supply_cost;
                        }
                    }
                }
            }
        }

        Self {
            campaign_stats,
            seller_stats,
            overall_stat,
            convergence_iterations,
        }
    }

    /// Output campaign statistics (without header, for compact iteration output)
    pub fn printout_campaigns(&self, campaigns: &Campaigns, campaign_controller_states: &CampaignControllerStates, logger: &mut Logger, event: LogEvent) {
        
        for (index, campaign_stat) in self.campaign_stats.iter().enumerate() {
            let campaign = &campaigns.campaigns[index];
            let controller_states_vec = &campaign_controller_states.campaign_controller_states[index];
            let controller_states: Vec<&dyn crate::controllers::ControllerStateTrait> = controller_states_vec.iter().map(|cs| cs.as_ref()).collect();
            let type_target_and_controller_string = campaign.type_target_and_controller_state_string(&controller_states);
            
            logln!(logger, event, "\nCampaign {} ({}) - {}", 
                     campaign.campaign_id(), campaign.campaign_name(), type_target_and_controller_string);
            logln!(logger, event, "  Impressions Obtained: {:.2}", campaign_stat.impressions_obtained);
            logln!(logger, event, "  Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     campaign_stat.total_supply_cost, 
                     campaign_stat.total_virtual_cost, 
                     campaign_stat.total_buyer_charge);
            let value_per_spend = if campaign_stat.total_buyer_charge > 0.0 {
                campaign_stat.total_value / campaign_stat.total_buyer_charge
            } else {
                0.0
            };
            logln!(logger, event, "  Obtained Value: {:.2} (per spend: {:.4})", campaign_stat.total_value, value_per_spend);
        }
    }

    /// Output seller statistics (without header, for compact iteration output)
    pub fn printout_sellers(&self, sellers: &Sellers, seller_controller_states: &SellerControllerStates, logger: &mut Logger, event: LogEvent) {
        
        for (index, seller_stat) in self.seller_stats.iter().enumerate() {
            let seller = &sellers.sellers[index];
            let controller_states: Vec<&dyn crate::controllers::ControllerStateTrait> = seller_controller_states.seller_controller_states[index].iter().map(|s| s.as_ref()).collect();
            let type_target_and_controller_string = seller.type_target_and_controller_state_string(&controller_states);
            
            logln!(logger, event, "\nSeller {} ({}) - {}", 
                     seller.seller_id(), seller.seller_name(), type_target_and_controller_string);
            logln!(logger, event, "  Impressions (sold/on offer): {} / {}", seller_stat.impressions_sold, seller.get_impressions_on_offer());
            logln!(logger, event, "  Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     seller_stat.total_supply_cost, 
                     seller_stat.total_virtual_cost, 
                     seller_stat.total_buyer_charge);
            logln!(logger, event, "  Total Provided Value: {:.2}", seller_stat.total_provided_value);
        }
    }

    /// Output complete statistics
    pub fn printout(&self, campaigns: &Campaigns, sellers: &Sellers, campaign_controller_states: &CampaignControllerStates, seller_controller_states: &SellerControllerStates, logger: &mut Logger) {
        
        // Output campaign statistics
        logln!(logger, LogEvent::Variant, "\n=== Campaign Statistics ===");
        self.printout_campaigns(campaigns, campaign_controller_states, logger, LogEvent::Variant);

        // Output seller statistics
        logln!(logger, LogEvent::Variant, "\n=== Seller Statistics ===");
        self.printout_sellers(sellers, seller_controller_states, logger, LogEvent::Variant);

        // Output overall statistics
        self.printout_overall(logger);
    }

    /// Output only overall statistics (no per-campaign or per-seller breakdown)
    pub fn printout_overall(&self, logger: &mut Logger) {
        
        logln!(logger, LogEvent::Variant, "\n=== Overall Statistics ===");
        logln!(logger, LogEvent::Variant, "Convergence: {} iterations", self.convergence_iterations);
        logln!(logger, LogEvent::Variant, "Impressions (lost/no bids): {} / {}", 
                 self.overall_stat.lost_count,
                 self.overall_stat.no_bids_count);
        logln!(logger, LogEvent::Variant, "Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                 self.overall_stat.total_supply_cost, 
                 self.overall_stat.total_virtual_cost, 
                 self.overall_stat.total_buyer_charge);
        
        // Calculate value per spend
        let value_per_spend = if self.overall_stat.total_buyer_charge > 0.0 {
            self.overall_stat.total_value / self.overall_stat.total_buyer_charge
        } else {
            0.0
        };
        logln!(logger, LogEvent::Variant, "Total Obtained Value: {:.2} (per spend: {:.4})", self.overall_stat.total_value, value_per_spend);
    }
}

