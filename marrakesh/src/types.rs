/// Charge type for impressions and sellers
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ChargeType {
    FIXED_COST { fixed_cost_cpm: f64 },
    FIRST_PRICE,
}

/// Campaign type determining the constraint model
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    FIXED_IMPRESSIONS { total_impressions_target: i32 },
    FIXED_BUDGET { total_budget_target: f64 },
}

/// Represents an impression on offer
#[derive(Debug, Clone)]
pub struct Impression {
    pub seller_id: usize,
    pub charge_type: ChargeType,
    pub best_other_bid_cpm: f64,
    pub floor_cpm: f64,
    pub win_bid_cpm: f64,
    pub win_campaign_id: usize,
    pub value_to_campaign_id: [f64; 10],
}

/// Represents a campaign participating in auctions
#[derive(Debug, Clone)]
pub struct Campaign {
    pub campaign_id: usize,
    pub campaign_rnd: u64,
    pub total_cost: f64,
    pub pacing: f64,
    pub campaign_type: CampaignType,
}

/// Represents a seller offering impressions
#[derive(Debug, Clone)]
pub struct Seller {
    pub seller_id: usize,
    pub seller_name: String,
    pub charge_type: ChargeType,
    pub num_impressions: usize,
}

/// Parameters for adding a campaign
pub struct AddCampaignParams {
    pub campaign_rnd: u64,
    pub total_cost: f64,
    pub pacing: f64,
    pub campaign_type: CampaignType,
}

/// Container for campaigns with methods to add campaigns
pub struct Campaigns {
    pub campaigns: Vec<Campaign>,
}

impl Campaigns {
    pub fn new() -> Self {
        Self {
            campaigns: Vec::new(),
        }
    }

    pub fn add(&mut self, params: AddCampaignParams) {
        let campaign_id = self.campaigns.len();
        self.campaigns.push(Campaign {
            campaign_id,
            campaign_rnd: params.campaign_rnd,
            total_cost: params.total_cost,
            pacing: params.pacing,
            campaign_type: params.campaign_type,
        });
    }
}

/// Parameters for adding a seller
pub struct AddSellerParams {
    pub seller_name: String,
    pub charge_type: ChargeType,
    pub num_impressions: usize,
}

/// Container for sellers with methods to add sellers
pub struct Sellers {
    pub sellers: Vec<Seller>,
}

impl Sellers {
    pub fn new() -> Self {
        Self {
            sellers: Vec::new(),
        }
    }

    pub fn add(&mut self, params: AddSellerParams) {
        let seller_id = self.sellers.len();
        self.sellers.push(Seller {
            seller_id,
            seller_name: params.seller_name,
            charge_type: params.charge_type,
            num_impressions: params.num_impressions,
        });
    }
}

