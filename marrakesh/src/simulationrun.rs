use crate::types::{Campaigns, ChargeType, Sellers, Winner};
use crate::Impressions;

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
}

/// Complete simulation statistics
pub struct SimulationStat {
    pub campaign_stats: Vec<CampaignStat>,
    pub seller_stats: Vec<SellerStat>,
    pub overall_stat: OverallStat,
}

/// Generate statistics from campaigns, sellers, and impressions
pub fn generate_statistics(campaigns: &Campaigns, sellers: &Sellers, impressions: &Impressions) -> SimulationStat {
    // Calculate campaign statistics
    let mut campaign_stats = Vec::new();
    for campaign in &campaigns.campaigns {
        let mut impressions_obtained = 0;
        let mut total_supply_cost = 0.0;
        let mut total_virtual_cost = 0.0;
        let mut total_buyer_charge = 0.0;
        let mut total_value = 0.0;

        for impression in &impressions.impressions {
            if let Winner::Campaign { campaign_id, virtual_cost, buyer_charge } = impression.result.winner {
                if campaign_id == campaign.campaign_id {
                    impressions_obtained += 1;
                    total_supply_cost += impression.result.supply_cost;
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

        for impression in &impressions.impressions {
            if impression.seller_id == seller.seller_id {
                match impression.result.winner {
                    Winner::Campaign { virtual_cost, buyer_charge, .. } => {
                        impressions_sold += 1;
                        total_supply_cost += impression.result.supply_cost;
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

    for impression in &impressions.impressions {
        match impression.result.winner {
            Winner::BELOW_FLOOR => below_floor_count += 1,
            Winner::OTHER_DEMAND => other_demand_count += 1,
            Winner::NO_DEMAND => no_bids_count += 1,
            Winner::Campaign { virtual_cost, buyer_charge, .. } => {
                // All costs are already converted from CPM
                total_supply_cost_all += impression.result.supply_cost;
                total_virtual_cost_all += virtual_cost;
                total_buyer_charge_all += buyer_charge;
            }
        }
    }

    SimulationStat {
        campaign_stats,
        seller_stats,
        overall_stat: OverallStat {
            below_floor_count,
            other_demand_count,
            no_bids_count,
            total_supply_cost: total_supply_cost_all,
            total_virtual_cost: total_virtual_cost_all,
            total_buyer_charge: total_buyer_charge_all,
        },
    }
}

/// Output statistics to terminal
pub fn output_statistics(stats: &SimulationStat, campaigns: &Campaigns, sellers: &Sellers) {
    // Output campaign statistics
    println!("\n=== Campaign Statistics ===");
    for (index, campaign_stat) in stats.campaign_stats.iter().enumerate() {
        let campaign = &campaigns.campaigns[index];
        println!("\nCampaign {} ({})", campaign.campaign_id, campaign.campaign_name);
        println!("  Impressions Obtained: {}", campaign_stat.impressions_obtained);
        println!("  Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
                 campaign_stat.total_supply_cost, 
                 campaign_stat.total_virtual_cost, 
                 campaign_stat.total_buyer_charge);
        println!("  Total Value: {:.2}", campaign_stat.total_value);
    }

    // Output seller statistics
    println!("\n=== Seller Statistics ===");
    for (index, seller_stat) in stats.seller_stats.iter().enumerate() {
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
    println!("\n=== Overall Statistics ===");
    println!("Below Floor: {}", stats.overall_stat.below_floor_count);
    println!("Sold to Other Demand: {}", stats.overall_stat.other_demand_count);
    println!("Without Any Bids: {}", stats.overall_stat.no_bids_count);
    println!("Total Costs (supply/virtual/buyer): {:.2} / {:.2} / {:.2}", 
             stats.overall_stat.total_supply_cost, 
             stats.overall_stat.total_virtual_cost, 
             stats.overall_stat.total_buyer_charge);
}

