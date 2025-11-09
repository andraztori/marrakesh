use crate::types::{AuctionResult, Campaigns, ChargeType, Sellers, Winner};
use crate::impressions::Impressions;

/// Container for auction results
/// Note: SimulationRun results are matched to Impressions by index in the vectors
pub struct SimulationRun {
    pub results: Vec<AuctionResult>,
}

impl SimulationRun {
    /// Create a new SimulationRun container and run auctions for all impressions
    pub fn new(impressions: &Impressions, campaigns: &Campaigns, campaign_params: &CampaignParams) -> Self {
        let mut results = Vec::with_capacity(impressions.impressions.len());
        
        for impression in &impressions.impressions {
            let result = impression.run_auction(campaigns, campaign_params);
            results.push(result);
        }
        
        Self { results }
    }
}

/// Represents campaign parameters (pacing, etc.)
/// Note: CampaignParam is matched to Campaign by index in the vectors
#[derive(Debug, Clone)]
pub struct CampaignParam {
    pub pacing: f64,
}

/// Container for campaign parameters
pub struct CampaignParams {
    pub params: Vec<CampaignParam>,
}

impl CampaignParams {
    /// Create campaign parameters from campaigns, defaulting all pacings to 1.0
    pub fn new(campaigns: &Campaigns) -> Self {
        let mut params = Vec::with_capacity(campaigns.campaigns.len());
        for _campaign in &campaigns.campaigns {
            params.push(CampaignParam {
                pacing: 1.0,
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
    /// Generate statistics from campaigns, sellers, impressions, and simulation run
    pub fn new(campaigns: &Campaigns, sellers: &Sellers, impressions: &Impressions, simulation_run: &SimulationRun) -> Self {
        // Calculate campaign statistics
        let mut campaign_stats = Vec::new();
        for campaign in &campaigns.campaigns {
            let mut impressions_obtained = 0;
            let mut total_supply_cost = 0.0;
            let mut total_virtual_cost = 0.0;
            let mut total_buyer_charge = 0.0;
            let mut total_value = 0.0;

            for (index, impression) in impressions.impressions.iter().enumerate() {
                if let Winner::Campaign { campaign_id, virtual_cost, buyer_charge } = simulation_run.results[index].winner {
                    if campaign_id == campaign.campaign_id {
                        impressions_obtained += 1;
                        total_supply_cost += simulation_run.results[index].supply_cost;
                        total_virtual_cost += virtual_cost;
                        total_buyer_charge += buyer_charge;
                        total_value += impression.value_to_campaign_id[campaign.campaign_id] / 1000.0;
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
        for seller in &sellers.sellers {
            let mut impressions_sold = 0;
            let mut total_supply_cost = 0.0;
            let mut total_virtual_cost = 0.0;
            let mut total_buyer_charge = 0.0;

            for (index, impression) in impressions.impressions.iter().enumerate() {
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
                    let impression = &impressions.impressions[index];
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

    /// Output campaign statistics to terminal (without header, for compact iteration output)
    pub fn printout_campaigns(&self, campaigns: &Campaigns, campaign_params: &CampaignParams) {
        use crate::types::CampaignType;
        
        for (index, campaign_stat) in self.campaign_stats.iter().enumerate() {
            let campaign = &campaigns.campaigns[index];
            let pacing = campaign_params.params[index].pacing;
            
            let type_and_target = match &campaign.campaign_type {
                CampaignType::FIXED_IMPRESSIONS { total_impressions_target } => {
                    format!("FIXED_IMPRESSIONS (target: {})", total_impressions_target)
                }
                CampaignType::FIXED_BUDGET { total_budget_target } => {
                    format!("FIXED_BUDGET (target: {:.2})", total_budget_target)
                }
            };
            
            println!("\nCampaign {} ({}) - {} - Pacing: {:.4}", 
                     campaign.campaign_id, campaign.campaign_name, type_and_target, pacing);
            println!("  Impressions Obtained: {}", campaign_stat.impressions_obtained);
            println!("  Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     campaign_stat.total_supply_cost, 
                     campaign_stat.total_virtual_cost, 
                     campaign_stat.total_buyer_charge);
            println!("  Obtained Value: {:.2}", campaign_stat.total_value);
        }
    }

    /// Output complete statistics to terminal
    pub fn printout(&self, campaigns: &Campaigns, sellers: &Sellers, campaign_params: &CampaignParams) {
        // Output campaign statistics
        println!("\n=== Campaign Statistics ===");
        self.printout_campaigns(campaigns, campaign_params);

        // Output seller statistics
        println!("\n=== Seller Statistics ===");
        for (index, seller_stat) in self.seller_stats.iter().enumerate() {
            let seller = &sellers.sellers[index];
            let charge_type_str = match seller.charge_type {
                ChargeType::FIXED_COST { fixed_cost_cpm } => format!("FIXED_COST ({} CPM)", fixed_cost_cpm),
                ChargeType::FIRST_PRICE => "FIRST_PRICE".to_string(),
            };

            println!("\nSeller {} ({}) - {}", seller.seller_id, seller.seller_name, charge_type_str);
            println!("  Impressions (sold/on offer): {} / {}", seller_stat.impressions_sold, seller.num_impressions);
            println!("  Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                     seller_stat.total_supply_cost, 
                     seller_stat.total_virtual_cost, 
                     seller_stat.total_buyer_charge);
        }

        // Output overall statistics
        self.printout_overall();
    }

    /// Output only overall statistics (no per-campaign or per-seller breakdown)
    pub fn printout_overall(&self) {
        println!("=== Overall Statistics ===");
        println!("Impressions (below floor/other demand/no bids): {} / {} / {}", 
                 self.overall_stat.below_floor_count,
                 self.overall_stat.other_demand_count,
                 self.overall_stat.no_bids_count);
        println!("Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                 self.overall_stat.total_supply_cost, 
                 self.overall_stat.total_virtual_cost, 
                 self.overall_stat.total_buyer_charge);
        println!("Total Obtained Value: {:.2}", self.overall_stat.total_value);
    }
}

