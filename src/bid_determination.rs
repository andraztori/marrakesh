use crate::campaign::{CompleteBid, CampaignTrait};
use crate::impressions::Impression;

/// Trait for applying margins to campaigns on impressions
pub trait BidDeterminationTrait {
    /// Get the complete bid with net and gross bid values for a campaign
    /// 
    /// # Arguments
    /// * `impression` - The impression being evaluated
    /// * `campaign` - The campaign to get complete bid for
    /// * `optimized_bid` - The optimized bid value
    /// 
    /// # Returns
    /// A `CompleteBid` containing net_bid and gross_bid values
    fn get_complete_bid(&self, impression: &Impression, campaign: &dyn CampaignTrait, optimized_bid: f64) -> CompleteBid;
}

/// Margin implementation that sets both net_bid and gross_bid to the optimized bid
pub struct BidDeterminationNoMargin;

impl BidDeterminationNoMargin {
    /// Create a new BidDeterminationNoMargin instance
    pub fn new() -> Self {
        Self
    }
}

impl BidDeterminationTrait for BidDeterminationNoMargin {
    fn get_complete_bid(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CompleteBid {
        CompleteBid {
            net_bid: optimized_bid,
            gross_bid: optimized_bid,
        }
    }
}

/// Margin implementation that applies a fixed percentage margin to the optimized bid
/// gross_bid = optimized_bid / (1 - margin)
/// net_bid = optimized_bid
pub struct BidDeterminationBidFixedMargin {
    margin: f64,
}

impl BidDeterminationBidFixedMargin {
    /// Create a new BidDeterminationBidFixedMargin instance with the specified margin
    /// 
    /// # Arguments
    /// * `margin` - The margin percentage (e.g., 0.1 for 10% margin)
    pub fn new(margin: f64) -> Self {
        Self { margin }
    }
}

impl BidDeterminationTrait for BidDeterminationBidFixedMargin {
    fn get_complete_bid(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CompleteBid {
        CompleteBid {
            net_bid: optimized_bid,
            gross_bid: optimized_bid / (1.0 - self.margin),
        }
    }
}

/// Margin implementation that applies a fixed percentage margin to the optimized bid by modifying net_bid
/// This is the unoptimal way to apply margin since we are messing around with net_bid instead of gross_bid
/// net_bid = optimized_bid * (1 - margin)
/// gross_bid = optimized_bid
pub struct BidDeterminationFixedMarginUnoptimal {
    margin: f64,
}

impl BidDeterminationFixedMarginUnoptimal {
    /// Create a new BidDeterminationFixedMarginUnoptimal instance with the specified margin
    /// 
    /// # Arguments
    /// * `margin` - The margin percentage (e.g., 0.1 for 10% margin)
    pub fn new(margin: f64) -> Self {
        Self { margin }
    }
}

impl BidDeterminationTrait for BidDeterminationFixedMarginUnoptimal {
    fn get_complete_bid(&self, _impression: &Impression, _campaign: &dyn CampaignTrait, optimized_bid: f64) -> CompleteBid {
        CompleteBid {
            net_bid: optimized_bid * (1.0 - self.margin),
            gross_bid: optimized_bid,
        }
    }
}

