mod types;

use types::{Campaign, CampaignType, Impression};

// Global vectors for impressions and campaigns
#[allow(static_mut_refs)]
static mut IMPRESSIONS: Vec<Impression> = Vec::new();
#[allow(static_mut_refs)]
static mut CAMPAIGNS: Vec<Campaign> = Vec::new();

fn main() {
    // Initialize with two hardcoded campaigns
    unsafe {
        CAMPAIGNS.push(Campaign {
            campaign_id: 1,
            campaign_rnd: 12345,
            total_cost: 0.0,
            pacing: 1.0,
            campaign_type: CampaignType::FIXED_IMPRESSIONS {
                total_impressions_target: 1000,
            },
        });

        CAMPAIGNS.push(Campaign {
            campaign_id: 2,
            campaign_rnd: 67890,
            total_cost: 0.0,
            pacing: 1.0,
            campaign_type: CampaignType::FIXED_BUDGET {
                total_budget_target: 5000.0,
            },
        });

        println!("Initialized {} campaigns", CAMPAIGNS.len());
        println!("Initialized {} impressions", IMPRESSIONS.len());
    }
}
