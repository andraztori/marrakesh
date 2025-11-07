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
    pub seller_id: i32,
    pub charge_type: ChargeType,
    pub best_other_bid_cpm: f64,
    pub floor_cpm: f64,
    pub win_bid_cpm: f64,
    pub win_campaign_id: i32,
    pub value_to_campaign_id: [f64; 10],
}

/// Represents a campaign participating in auctions
#[derive(Debug, Clone)]
pub struct Campaign {
    pub campaign_id: i32,
    pub campaign_rnd: u64,
    pub total_cost: f64,
    pub pacing: f64,
    pub campaign_type: CampaignType,
}

/// Represents a seller offering impressions
#[derive(Debug, Clone)]
pub struct Seller {
    pub seller_id: i32,
    pub seller_name: String,
    pub charge_type: ChargeType,
}

