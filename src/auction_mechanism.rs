/// Auction mechanism trait and implementations
/// 
/// This module provides different auction mechanisms for computing internal scores
/// used in auction winner determination. Different mechanisms can optimize for
/// different objectives (net spend, gross revenue, expected margin, etc.)

use crate::competition::ImpressionCompetition;

/// Trait for auction mechanisms that compute internal scores for winner determination
pub trait AuctionMechanismTrait {
    /// Compute the internal score for an auction bid
    /// 
    /// # Arguments
    /// * `net_bid` - The net bid amount (CPM)
    /// * `gross_bid` - The gross bid amount (CPM)
    /// * `competition` - Optional competition data containing sigmoid parameters for win probability
    /// 
    /// # Returns
    /// The internal score used for winner determination
    fn compute_internal_score(&self, net_bid: f64, gross_bid: f64, competition: Option<&ImpressionCompetition>) -> f64;
}

/// Plain net bid mechanism - uses net bid as the score
pub struct AuctionMechanismPlainNet;

impl AuctionMechanismPlainNet {
    /// Create a new AuctionMechanismPlainNet
    pub fn new() -> Self {
        Self
    }
}

impl AuctionMechanismTrait for AuctionMechanismPlainNet {
    fn compute_internal_score(&self, net_bid: f64, _gross_bid: f64, _competition: Option<&ImpressionCompetition>) -> f64 {
        net_bid
    }
}

/// Plain gross bid mechanism - uses gross bid as the score
pub struct AuctionMechanismPlainGross;

impl AuctionMechanismPlainGross {
    /// Create a new AuctionMechanismPlainGross
    pub fn new() -> Self {
        Self
    }
}

impl AuctionMechanismTrait for AuctionMechanismPlainGross {
    fn compute_internal_score(&self, _net_bid: f64, gross_bid: f64, _competition: Option<&ImpressionCompetition>) -> f64 {
        gross_bid
    }
}

/// Expected net spend mechanism - multiplies net bid by win probability at net bid
pub struct AuctionMechanismExpectedNetSpend;

impl AuctionMechanismExpectedNetSpend {
    /// Create a new AuctionMechanismExpectedNetSpend
    pub fn new() -> Self {
        Self
    }
}

impl AuctionMechanismTrait for AuctionMechanismExpectedNetSpend {
    fn compute_internal_score(&self, net_bid: f64, _gross_bid: f64, competition: Option<&ImpressionCompetition>) -> f64 {
        let comp = competition.expect("AuctionMechanismExpectedNetSpend requires competition data");
        let win_probability = comp.get_predicted_win_probability(net_bid); // we want predicted win probability not "actual" that would be an oracle
        net_bid * win_probability
    }
}

/// Expected gross revenue mechanism - multiplies gross bid by win probability at net bid
pub struct AuctionMechanismExpectedGrossRevenue;

impl AuctionMechanismExpectedGrossRevenue {
    /// Create a new AuctionMechanismExpectedGrossRevenue
    pub fn new() -> Self {
        Self
    }
}

impl AuctionMechanismTrait for AuctionMechanismExpectedGrossRevenue {
    fn compute_internal_score(&self, net_bid: f64, gross_bid: f64, competition: Option<&ImpressionCompetition>) -> f64 {
        let comp = competition.expect("AuctionMechanismExpectedGrossRevenue requires competition data");
        let win_probability = comp.get_predicted_win_probability(net_bid); // we want predicted win probability not "actual" that would be an oracle
        gross_bid * win_probability
    }
}

/// Expected margin mechanism - multiplies margin (gross - net) by win probability at net bid
pub struct AuctionMechanismExpectedMargin;

impl AuctionMechanismExpectedMargin {
    /// Create a new AuctionMechanismExpectedMargin
    pub fn new() -> Self {
        Self
    }
}

impl AuctionMechanismTrait for AuctionMechanismExpectedMargin {
    fn compute_internal_score(&self, net_bid: f64, gross_bid: f64, competition: Option<&ImpressionCompetition>) -> f64 {
        let comp = competition.expect("AuctionMechanismExpectedMargin requires competition data");
        let win_probability = comp.get_predicted_win_probability(net_bid); // we want predicted win probability not "actual" that would be an oracle
        let margin = gross_bid - net_bid;
        margin * win_probability
    }
}

