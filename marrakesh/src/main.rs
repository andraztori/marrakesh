mod types;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};
use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Impression, Campaigns, Sellers};

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
        let floor_cpm_dist = Normal::new(5.0, 1.0).unwrap();

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

                self.impressions.push(Impression {
                    seller_id: seller.seller_id,
                    charge_type: seller.charge_type.clone(),
                    best_other_bid_cpm,
                    floor_cpm,
                    win_bid_cpm: 0.0,
                    win_campaign_id: 0,
                    value_to_campaign_id: [0.0; 10],
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
        campaign_rnd: 12345,
        total_cost: 0.0,
        pacing: 1.0,
        campaign_type: CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 10000,
        },
    });

    campaigns.add(AddCampaignParams {
        campaign_rnd: 67890,
        total_cost: 0.0,
        pacing: 1.0,
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 200.0,
        },
    });

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
}
