mod types;

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
        for seller in &sellers.sellers {
            for _ in 0..seller.num_impressions {
                self.impressions.push(Impression {
                    seller_id: seller.seller_id,
                    charge_type: seller.charge_type.clone(),
                    best_other_bid_cpm: 0.0,
                    floor_cpm: 0.0,
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
            total_impressions_target: 1000,
        },
    });

    campaigns.add(AddCampaignParams {
        campaign_rnd: 67890,
        total_cost: 0.0,
        pacing: 1.0,
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 5000.0,
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
