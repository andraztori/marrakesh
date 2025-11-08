mod types;
mod simulationrun;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};
use types::{AddCampaignParams, AddSellerParams, AuctionResult, CampaignType, ChargeType, Impression, Campaigns, Sellers, Winner, MAX_CAMPAIGNS};
use simulationrun::{generate_statistics, output_statistics, CampaignParams};

/// Container for impressions with methods to create impressions
pub struct Impressions {
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
                        supply_cost: 0.0,
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
        campaign_type: CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 10000,
        },
    }).expect("Failed to add campaign");

    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 1".to_string(),
        campaign_rnd: 67890,
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 200.0,
        },
    }).expect("Failed to add campaign");

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let campaign_params = CampaignParams::new(&campaigns);

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
        impression.run_auction(&campaigns, &campaign_params);
    }

    // Generate statistics
    let stats = generate_statistics(&campaigns, &sellers, &impressions);

    // Output statistics
    output_statistics(&stats, &campaigns, &sellers);
}
