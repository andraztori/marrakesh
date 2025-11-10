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

/// Represents campaign parameters (pacing, etc.)
/// Note: This struct is kept for backward compatibility but is no longer used.
/// Campaign convergence parameters now use the CampaignConverge trait.
#[derive(Debug, Clone)]
pub struct CampaignParam {
    pub pacing: f64,
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
    /// Create campaign parameters from campaigns, defaulting all pacings to 1.0
    pub fn new(campaigns: &Campaigns) -> Self {
        let mut params = Vec::with_capacity(campaigns.campaigns.len());
        for campaign in &campaigns.campaigns {
            params.push(campaign.create_converge_param(1.0));
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
        // Calculate campaign statistics
        use crate::campaigns::CampaignTrait;
        let mut campaign_stats = Vec::new();
        for campaign in &marketplace.campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let mut impressions_obtained = 0;
            let mut total_supply_cost = 0.0;
            let mut total_virtual_cost = 0.0;
            let mut total_buyer_charge = 0.0;
            let mut total_value = 0.0;

            for (index, impression) in marketplace.impressions.impressions.iter().enumerate() {
                if let Winner::Campaign { campaign_id: winner_campaign_id, virtual_cost, buyer_charge } = simulation_run.results[index].winner {
                    if winner_campaign_id == campaign_id {
                        impressions_obtained += 1;
                        total_supply_cost += simulation_run.results[index].supply_cost;
                        total_virtual_cost += virtual_cost;
                        total_buyer_charge += buyer_charge;
                        total_value += impression.value_to_campaign_id[campaign_id] / 1000.0;
                    }
                }
            }

            campaign_stats.push(CampaignStat {
                impressions_obtained,
                total_supply_cost,
                total_virtual_cost,
                total_buyer_charge,
                total_value,
            });
        }

        // Calculate seller statistics
        let mut seller_stats = Vec::new();
        for seller in &marketplace.sellers.sellers {
            let mut impressions_sold = 0;
            let mut total_supply_cost = 0.0;
            let mut total_virtual_cost = 0.0;
            let mut total_buyer_charge = 0.0;

            for (index, impression) in marketplace.impressions.impressions.iter().enumerate() {
                if impression.seller_id == seller.seller_id {
                    match simulation_run.results[index].winner {
                        Winner::Campaign { virtual_cost, buyer_charge, .. } => {
                            impressions_sold += 1;
                            total_supply_cost += simulation_run.results[index].supply_cost;
                            total_virtual_cost += virtual_cost;
                            total_buyer_charge += buyer_charge;
                        }
                        _ => {}
                    }
                }
            }

            seller_stats.push(SellerStat {
                impressions_sold,
                total_supply_cost,
                total_virtual_cost,
                total_buyer_charge,
            });
        }

        // Calculate overall statistics
        let mut below_floor_count = 0;
        let mut other_demand_count = 0;
        let mut no_bids_count = 0;
        let mut total_supply_cost_all = 0.0;
        let mut total_virtual_cost_all = 0.0;
        let mut total_buyer_charge_all = 0.0;
        let mut total_value_all = 0.0;

        for (index, result) in simulation_run.results.iter().enumerate() {
            match result.winner {
                Winner::BELOW_FLOOR => below_floor_count += 1,
                Winner::OTHER_DEMAND => other_demand_count += 1,
                Winner::NO_DEMAND => no_bids_count += 1,
                Winner::Campaign { campaign_id, virtual_cost, buyer_charge, .. } => {
                    // All costs are already converted from CPM
                    total_supply_cost_all += result.supply_cost;
                    total_virtual_cost_all += virtual_cost;
                    total_buyer_charge_all += buyer_charge;
                    // Add value for the winning campaign
                    let impression = &marketplace.impressions.impressions[index];
                    total_value_all += impression.value_to_campaign_id[campaign_id] / 1000.0;
                }
            }
        }

        Self {
            campaign_stats,
            seller_stats,
            overall_stat: OverallStat {
                below_floor_count,
                other_demand_count,
                no_bids_count,
                total_supply_cost: total_supply_cost_all,
                total_virtual_cost: total_virtual_cost_all,
                total_buyer_charge: total_buyer_charge_all,
                total_value: total_value_all,
            },
        }
    }

    /// Output campaign statistics (without header, for compact iteration output)
    pub fn printout_campaigns(&self, campaigns: &Campaigns, campaign_params: &CampaignConvergeParams, logger: &mut Logger, event: LogEvent) {
        use crate::campaigns::CampaignTrait;
        
        for (index, campaign_stat) in self.campaign_stats.iter().enumerate() {
            let campaign = &campaigns.campaigns[index];
            let pacing = campaign_params.params[index].pacing();
            
            // Use the trait method to get type and target string
            let type_and_target = campaign.type_and_target_string();
            
            logln!(logger, event, "\nCampaign {} ({}) - {} - Pacing: {:.4}", 
                     campaign.campaign_id(), campaign.campaign_name(), type_and_target, pacing);
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

