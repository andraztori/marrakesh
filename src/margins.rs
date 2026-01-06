use crate::campaign::{CampaignBid, CampaignTrait};
use crate::impressions::Impression;

/// Trait for applying margins to campaigns on impressions
pub trait MarginTrait {
    /// Apply the margin to an optimized bid for a campaign
    /// 
    /// # Arguments
    /// * `impression` - The impression being evaluated
    /// * `campaign` - The campaign to apply margin for
    /// * `optimized_bid` - The optimized bid value
    /// 
    /// # Returns
    /// A `CampaignBid` containing net_bid and gross_bid values
    fn apply_margin(&self, impression: &Impression, campaign: &dyn CampaignTrait, optimized_bid: f64) -> CampaignBid;
}

/// Margin implementation that sets both net_bid and gross_bid to the optimized bid
pub struct MarginNone;

impl MarginNone {
    /// Create a new MarginNone instance
    pub fn new() -> Self {
        Self
    }
}

impl MarginTrait for MarginNone {
    fn apply_margin(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CampaignBid {
        CampaignBid {
            net_bid: optimized_bid,
            gross_bid: optimized_bid,
        }
    }
}

/// Margin implementation that applies a fixed percentage margin to the optimized bid
/// gross_bid = optimized_bid / (1 - margin)
/// net_bid = optimized_bid
pub struct MarginFixed {
    margin: f64,
}

impl MarginFixed {
    /// Create a new MarginFixed instance with the specified margin
    /// 
    /// # Arguments
    /// * `margin` - The margin percentage (e.g., 0.1 for 10% margin)
    pub fn new(margin: f64) -> Self {
        Self { margin }
    }
}

impl MarginTrait for MarginFixed {
    fn apply_margin(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CampaignBid {
        CampaignBid {
            net_bid: optimized_bid,
            gross_bid: optimized_bid / (1.0 - self.margin),
        }
    }
}

/// Margin implementation that applies a fixed percentage margin to the optimized bid by modifying net_bid
/// This is the unoptimal way to apply margin since we are messing around with net_bid instead of gross_bid
/// net_bid = optimized_bid * (1 - margin)
/// gross_bid = optimized_bid
pub struct MarginFixedUnoptimal {
    margin: f64,
}

impl MarginFixedUnoptimal {
    /// Create a new MarginFixedUnoptimal instance with the specified margin
    /// 
    /// # Arguments
    /// * `margin` - The margin percentage (e.g., 0.1 for 10% margin)
    pub fn new(margin: f64) -> Self {
        Self { margin }
    }
}

impl MarginTrait for MarginFixedUnoptimal {
    fn apply_margin(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CampaignBid {
        CampaignBid {
            net_bid: optimized_bid * (1.0 - self.margin),
            gross_bid: optimized_bid,
        }
    }
}

