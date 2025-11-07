mod types;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};
use types::{AddCampaignParams, AddSellerParams, AuctionResult, CampaignType, ChargeType, Impression, Campaigns, Sellers, Winner, MAX_CAMPAIGNS};

/// Container for impressions with methods to create impressions
struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    pub fn new() -> Self {
        Self {
            impressions: Vec::new(),
        }
    }

    pub fn create_from_sellers(&mut self, sellers: &Sellers) {
        // Use deterministic seed for reproducible results
        let mut rng = StdRng::seed_from_u64(999);
        let best_other_bid_dist = Normal::new(10.0, 1.0).unwrap();
        let floor_cpm_dist = Normal::new(8.0, 3.0).unwrap();
        let value_to_campaign_dist = Normal::new(10.0, 3.0).unwrap();

        for seller in &sellers.sellers {
            for _ in 0..seller.num_impressions {
                let (best_other_bid_cpm, floor_cpm) = match seller.charge_type {
                    ChargeType::FIRST_PRICE => {
                        (
                            best_other_bid_dist.sample(&mut rng),
                            floor_cpm_dist.sample(&mut rng),
                        )
                    }
                    ChargeType::FIXED_COST { .. } => (0.0, 0.0),
                };

                // Generate random values for value_to_campaign_id array
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                for i in 0..MAX_CAMPAIGNS {
                    value_to_campaign_id[i] = value_to_campaign_dist.sample(&mut rng);
                }

                self.impressions.push(Impression {
                    seller_id: seller.seller_id,
                    charge_type: seller.charge_type.clone(),
                    best_other_bid_cpm,
                    floor_cpm,
                    result: AuctionResult {
                        winner: Winner::BELOW_FLOOR,
                        true_cost_cpm: 0.0,
                    },
                    value_to_campaign_id,
                });
            }
        }
    }
}

fn main() {
    // Initialize containers for impressions, campaigns, and sellers
    let mut impressions = Impressions::new();
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 0".to_string(),
        campaign_rnd: 12345,
        total_cost: 0.0,
        pacing: 1.0,
        campaign_type: CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 10000,
        },
    }).expect("Failed to add campaign");

    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 1".to_string(),
        campaign_rnd: 67890,
        total_cost: 0.0,
        pacing: 1.0,
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 200.0,
        },
    }).expect("Failed to add campaign");

    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(AddSellerParams {
        seller_name: "MRG".to_string(),
        charge_type: ChargeType::FIXED_COST {
            fixed_cost_cpm: 10.0,
        },
        num_impressions: 10000,
    });

    sellers.add(AddSellerParams {
        seller_name: "HB".to_string(),
        charge_type: ChargeType::FIRST_PRICE,
        num_impressions: 10000,
    });

    // Create impressions for all sellers
    impressions.create_from_sellers(&sellers);

    println!("Initialized {} campaigns", campaigns.campaigns.len());
    println!("Initialized {} impressions", impressions.impressions.len());
    println!("Initialized {} sellers", sellers.sellers.len());

    // Run auctions for all impressions
    for impression in &mut impressions.impressions {
        impression.run_auction(&campaigns);
    }

    // Calculate and print stats for each campaign
    println!("\n=== Campaign Statistics ===");
    for campaign in &campaigns.campaigns {
        let mut impressions_obtained = 0;
        let mut total_cost = 0.0;
        let mut total_value = 0.0;

        for impression in &impressions.impressions {
            if let Winner::Campaign { campaign_id, virtual_cost_cpm, .. } = impression.result.winner {
                if campaign_id == campaign.campaign_id {
                    impressions_obtained += 1;
                    // Divide by 1000 since everything is expressed in CPM
                    total_cost += virtual_cost_cpm / 1000.0;
                    total_value += impression.value_to_campaign_id[campaign.campaign_id] / 1000.0;
                }
            }
        }

        println!("\nCampaign {} ({})", campaign.campaign_id, campaign.campaign_name);
        println!("  Impressions Obtained: {}", impressions_obtained);
        println!("  Total Cost: {:.2}", total_cost);
        println!("  Total Value: {:.2}", total_value);
    }

    // Count overall statistics
    let mut below_floor_count = 0;
    let mut other_demand_count = 0;
    let mut no_bids_count = 0;
    let mut total_cost_all = 0.0;

    for impression in &impressions.impressions {
        match impression.result.winner {
            Winner::BELOW_FLOOR => below_floor_count += 1,
            Winner::OTHER_DEMAND => other_demand_count += 1,
            Winner::NO_DEMAND => no_bids_count += 1,
            Winner::Campaign { virtual_cost_cpm, .. } => {
                // Check if this is a fixed cost impression
                if let ChargeType::FIXED_COST { fixed_cost_cpm } = impression.charge_type {
                    // For fixed cost impressions, use the fixed_cost_cpm
                    total_cost_all += fixed_cost_cpm / 1000.0;
                } else {
                    // For first price impressions, use virtual_cost_cpm
                    total_cost_all += virtual_cost_cpm / 1000.0;
                }
            }
        }
    }

    // Calculate statistics for each seller
    println!("\n=== Seller Statistics ===");
    for seller in &sellers.sellers {
        let mut impressions_sold = 0;
        let mut total_revenue = 0.0;

        for impression in &impressions.impressions {
            if impression.seller_id == seller.seller_id {
                match impression.result.winner {
                    Winner::Campaign { virtual_cost_cpm, .. } => {
                        impressions_sold += 1;
                        // Calculate revenue based on charge type
                        match impression.charge_type {
                            ChargeType::FIXED_COST { fixed_cost_cpm } => {
                                total_revenue += fixed_cost_cpm / 1000.0;
                            }
                            ChargeType::FIRST_PRICE => {
                                total_revenue += virtual_cost_cpm / 1000.0;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        println!("\nSeller {} ({})", seller.seller_id, seller.seller_name);
        println!("  Impressions Sold: {}", impressions_sold);
        println!("  Total Revenue: {:.2}", total_revenue);
    }

    println!("\n=== Overall Statistics ===");
    println!("Below Floor: {}", below_floor_count);
    println!("Sold to Other Demand: {}", other_demand_count);
    println!("Without Any Bids: {}", no_bids_count);
    println!("Total Cost (all impressions): {:.2}", total_cost_all);
}
