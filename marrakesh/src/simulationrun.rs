use crate::impressions::{AuctionResult, Winner};
use crate::sellers::Sellers;
use crate::campaigns::Campaigns;
use crate::converge::{CampaignControllerStates, SellerControllerStates};
use crate::logger::{Logger, LogEvent};
use crate::logln;

/// Marketplace containing campaigns, sellers, and impressions
/// This groups together the three main components of the marketplace simulation
pub struct Marketplace {
    pub campaigns: crate::campaigns::Campaigns,
    pub sellers: crate::sellers::Sellers,
    pub impressions: crate::impressions::Impressions,
}

impl Marketplace {
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
}

impl SimulationRun {
    /// Create a new SimulationRun container and run auctions for all impressions
    pub fn new(marketplace: &Marketplace, campaign_controller_states: &CampaignControllerStates, seller_controller_states: &SellerControllerStates, logger: &mut Logger) -> Self {
        let mut results = Vec::with_capacity(marketplace.impressions.impressions.len());
        
        for impression in &marketplace.impressions.impressions {
            // Get the seller and seller_converge for this impression
            let seller = marketplace.sellers.sellers[impression.seller_id].as_ref();
            let seller_converge = seller_controller_states.seller_controller_states[seller.seller_id()].as_ref();
            let result = impression.run_auction(&marketplace.campaigns, campaign_controller_states, seller, seller_converge, logger);
            results.push(result);
        }
        
        Self { results }
    }
}

/// Statistics for a single campaign
pub struct CampaignStat {
    pub impressions_obtained: usize,
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
    pub below_floor_count: usize,
    pub other_demand_count: usize,
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
                impressions_obtained: 0,
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
            below_floor_count: 0,
            other_demand_count: 0,
            no_bids_count: 0,
            total_supply_cost: 0.0,
            total_virtual_cost: 0.0,
            total_buyer_charge: 0.0,
            total_value: 0.0,
        };

        // Iterate through impressions once and accumulate all statistics
            for (index, impression) in marketplace.impressions.impressions.iter().enumerate() {
            let result = &simulation_run.results[index];
            let seller_id = impression.seller_id;

            // Update overall statistics based on winner
            match result.winner {
                Winner::BELOW_FLOOR => overall_stat.below_floor_count += 1,
                Winner::OTHER_DEMAND => overall_stat.other_demand_count += 1,
                Winner::NO_DEMAND => overall_stat.no_bids_count += 1,
                Winner::Campaign { campaign_id, virtual_cost, buyer_charge, .. } => {
                    // Update overall statistics
                    overall_stat.total_supply_cost += result.supply_cost;
                    overall_stat.total_virtual_cost += virtual_cost;
                    overall_stat.total_buyer_charge += buyer_charge;
                    overall_stat.total_value += impression.value_to_campaign_id[campaign_id] / 1000.0;

                    // Update seller statistics
                    let seller_stat = &mut seller_stats[seller_id];
                    seller_stat.impressions_sold += 1;
                    seller_stat.total_supply_cost += result.supply_cost;
                    seller_stat.total_virtual_cost += virtual_cost;
                    seller_stat.total_buyer_charge += buyer_charge;
                    seller_stat.total_provided_value += impression.value_to_campaign_id[campaign_id] / 1000.0;

                    // Update campaign statistics
                    let campaign_stat = &mut campaign_stats[campaign_id];
                    campaign_stat.impressions_obtained += 1;
                    campaign_stat.total_supply_cost += result.supply_cost;
                    campaign_stat.total_virtual_cost += virtual_cost;
                    campaign_stat.total_buyer_charge += buyer_charge;
                    campaign_stat.total_value += impression.value_to_campaign_id[campaign_id] / 1000.0;
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
            let controller_state = campaign_controller_states.campaign_controller_states[index].as_ref();
            let type_target_and_controller_string = campaign.type_target_and_controller_state_string(controller_state);
            
            logln!(logger, event, "\nCampaign {} ({}) - {}", 
                     campaign.campaign_id(), campaign.campaign_name(), type_target_and_controller_string);
            logln!(logger, event, "  Impressions Obtained: {}", campaign_stat.impressions_obtained);
            logln!(logger, event, "  Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     campaign_stat.total_supply_cost, 
                     campaign_stat.total_virtual_cost, 
                     campaign_stat.total_buyer_charge);
            logln!(logger, event, "  Obtained Value: {:.2}", campaign_stat.total_value);
        }
    }

    /// Output seller statistics (without header, for compact iteration output)
    pub fn printout_sellers(&self, sellers: &Sellers, seller_controller_states: &SellerControllerStates, logger: &mut Logger, event: LogEvent) {
        
        for (index, seller_stat) in self.seller_stats.iter().enumerate() {
            let seller = &sellers.sellers[index];
            let controller_state = seller_controller_states.seller_controller_states[index].as_ref();
            let type_target_and_controller_string = seller.type_target_and_controller_state_string(controller_state);
            
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
        logln!(logger, LogEvent::Variant, "Impressions (below floor/other demand/no bids): {} / {} / {}", 
                 self.overall_stat.below_floor_count,
                 self.overall_stat.other_demand_count,
                 self.overall_stat.no_bids_count);
        logln!(logger, LogEvent::Variant, "Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                 self.overall_stat.total_supply_cost, 
                 self.overall_stat.total_virtual_cost, 
                 self.overall_stat.total_buyer_charge);
        logln!(logger, LogEvent::Variant, "Total Obtained Value: {:.2}", self.overall_stat.total_value);
    }
}

