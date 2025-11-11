use crate::types::{AuctionResult, ChargeType, Marketplace, Sellers, Winner};
use crate::campaigns::Campaigns;
use crate::logger::{Logger, LogEvent};
use crate::logln;

/// Container for auction results
/// Note: SimulationRun results are matched to Impressions by index in the vectors
pub struct SimulationRun {
    pub results: Vec<AuctionResult>,
}

impl SimulationRun {
    /// Create a new SimulationRun container and run auctions for all impressions
    pub fn new(marketplace: &Marketplace, campaign_params: &CampaignConvergeParams, seller_params: &SellerConvergeParams) -> Self {
        let mut results = Vec::with_capacity(marketplace.impressions.impressions.len());
        
        for impression in &marketplace.impressions.impressions {
            // Get the seller and seller_param for this impression
            let seller = &marketplace.sellers.sellers[impression.seller_id];
            let seller_param = &seller_params.params[impression.seller_id];
            let result = impression.run_auction(&marketplace.campaigns, campaign_params, seller, seller_param);
            results.push(result);
        }
        
        Self { results }
    }
}

/// Container for campaign convergence parameters
/// Uses dynamic dispatch to support different campaign types
pub struct CampaignConvergeParams {
    pub params: Vec<Box<dyn crate::campaigns::CampaignConverge>>,
}

impl Clone for CampaignConvergeParams {
    fn clone(&self) -> Self {
        Self {
            params: self.params.iter().map(|p| p.clone_box()).collect(),
        }
    }
}

impl CampaignConvergeParams {
    /// Create campaign parameters from campaigns
    pub fn new(campaigns: &Campaigns) -> Self {
        let mut params = Vec::with_capacity(campaigns.campaigns.len());
        for campaign in &campaigns.campaigns {
            params.push(campaign.create_converge_param());
        }
        Self { params }
    }
}

/// Represents seller parameters (boost_factor, etc.)
/// Note: SellerParam is matched to Seller by index in the vectors
#[derive(Debug, Clone)]
pub struct SellerParam {
    pub boost_factor: f64,
}

/// Container for seller parameters
pub struct SellerConvergeParams {
    pub params: Vec<SellerParam>,
}

impl SellerConvergeParams {
    /// Create seller parameters from sellers, defaulting all boost_factors to 1.0
    pub fn new(sellers: &Sellers) -> Self {
        let mut params = Vec::with_capacity(sellers.sellers.len());
        for _seller in &sellers.sellers {
            params.push(SellerParam {
                boost_factor: 1.0,
            });
        }
        Self { params }
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
}

impl SimulationStat {
    /// Generate statistics from marketplace and simulation run
    pub fn new(marketplace: &Marketplace, simulation_run: &SimulationRun) -> Self {
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
        }
    }

    /// Output campaign statistics (without header, for compact iteration output)
    pub fn printout_campaigns(&self, campaigns: &Campaigns, campaign_params: &CampaignConvergeParams, logger: &mut Logger, event: LogEvent) {
        
        for (index, campaign_stat) in self.campaign_stats.iter().enumerate() {
            let campaign = &campaigns.campaigns[index];
            let converge_param = campaign_params.params[index].as_ref();
            
            // Use the trait method to get type and target string
            let type_and_target = campaign.type_and_target_string();
            let formatted_params = campaign.converge_params_string(converge_param);
            
            logln!(logger, event, "\nCampaign {} ({}) - {} - {}", 
                     campaign.campaign_id(), campaign.campaign_name(), type_and_target, formatted_params);
            logln!(logger, event, "  Impressions Obtained: {}", campaign_stat.impressions_obtained);
            logln!(logger, event, "  Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     campaign_stat.total_supply_cost, 
                     campaign_stat.total_virtual_cost, 
                     campaign_stat.total_buyer_charge);
            logln!(logger, event, "  Obtained Value: {:.2}", campaign_stat.total_value);
        }
    }

    /// Output complete statistics
    pub fn printout(&self, campaigns: &Campaigns, sellers: &Sellers, campaign_params: &CampaignConvergeParams, logger: &mut Logger) {
        
        // Output campaign statistics
        logln!(logger, LogEvent::Variant, "\n=== Campaign Statistics ===");
        self.printout_campaigns(campaigns, campaign_params, logger, LogEvent::Variant);

        // Output seller statistics
        logln!(logger, LogEvent::Variant, "\n=== Seller Statistics ===");
        for (index, seller_stat) in self.seller_stats.iter().enumerate() {
            let seller = &sellers.sellers[index];
            let charge_type_str = match seller.charge_type {
                ChargeType::FIXED_COST { fixed_cost_cpm } => format!("FIXED_COST ({} CPM)", fixed_cost_cpm),
                ChargeType::FIRST_PRICE => "FIRST_PRICE".to_string(),
            };

            logln!(logger, LogEvent::Variant, "\nSeller {} ({}) - {}", seller.seller_id, seller.seller_name, charge_type_str);
            logln!(logger, LogEvent::Variant, "  Impressions (sold/on offer): {} / {}", seller_stat.impressions_sold, seller.num_impressions);
            logln!(logger, LogEvent::Variant, "  Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     seller_stat.total_supply_cost, 
                     seller_stat.total_virtual_cost, 
                     seller_stat.total_buyer_charge);
        }

        // Output overall statistics
        self.printout_overall(logger);
    }

    /// Output only overall statistics (no per-campaign or per-seller breakdown)
    pub fn printout_overall(&self, logger: &mut Logger) {
        
        logln!(logger, LogEvent::Variant, "\n=== Overall Statistics ===");
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

