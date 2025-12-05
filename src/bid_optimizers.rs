/// Bid optimizer trait and implementations
/// 
/// This module provides a trait for bid optimization strategies that take a value
/// (typically a boosted price or max affordable bid) and optimize it based on the
/// impression's competition and floor data.

use crate::impressions::Impression;
use crate::sigmoid::Sigmoid;

/// Trait for bid optimization strategies
pub trait BidOptimizerTrait {
    /// Optimize a bid value based on the impression's competition and floor data
    /// 
    /// # Arguments
    /// * `value` - The base value to optimize (e.g., boosted price, max affordable bid)
    /// * `impression` - The impression being bid on
    /// 
    /// # Returns
    /// The optimized bid value, or None if no bid should be made
    fn get_optimized_bid(&self, value: f64, impression: &Impression) -> Option<f64>;
    
    /// Get the name/type of this optimizer
    fn get_optimizer_type(&self) -> String;
}

/// Truthful bid optimizer that returns the value as-is without optimization
pub struct BidOptimizerTrutful;

impl BidOptimizerTrait for BidOptimizerTrutful {
    fn get_optimized_bid(&self, value: f64, _impression: &Impression) -> Option<f64> {
        Some(value)
    }
    
    fn get_optimizer_type(&self) -> String {
        "Truthful".to_string()
    }
}

/// Maximum margin bid optimizer that uses sigmoid-based optimization
pub struct BidOptimizerMaximumMargin;

impl BidOptimizerTrait for BidOptimizerMaximumMargin {
    fn get_optimized_bid(&self, value: f64, impression: &Impression) -> Option<f64> {
        let competition = impression.competition.as_ref()
            .expect("Maximum margin optimizer requires competition data. This impression has no competition data.");
        
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            1.0,  // Using normalized value of 1.0
        );
        
        sigmoid.max_margin_bid_bisection(value, impression.floor_cpm)
    }
    
    fn get_optimizer_type(&self) -> String {
        "MaxMargin".to_string()
    }
}

/// Cheater bid optimizer that bids just above the minimum winning bid
pub struct BidOptimizerCheater;

impl BidOptimizerTrait for BidOptimizerCheater {
    fn get_optimized_bid(&self, value: f64, impression: &Impression) -> Option<f64> {
        // value is the max_affordable_bid
        
        // Calculate minimum winning bid as maximum of floor and competing bid, plus 0.00001
        let mut minimum_winning_bid = impression.floor_cpm;
        if let Some(competition) = &impression.competition {
            minimum_winning_bid = minimum_winning_bid.max(competition.bid_cpm);
        }
        
        minimum_winning_bid += 0.00001;
        
        // Check if we can afford the minimum winning bid
        if value < minimum_winning_bid {
            return None;
        }
        
        Some(minimum_winning_bid)
    }
    
    fn get_optimizer_type(&self) -> String {
        "Cheater".to_string()
    }
}

/// Median bid optimizer that bids at the predicted offset point
pub struct BidOptimizerMedian;

impl BidOptimizerTrait for BidOptimizerMedian {
    fn get_optimized_bid(&self, value: f64, impression: &Impression) -> Option<f64> {
        // value is the campaign_control_bid
        
        let competition = impression.competition.as_ref()
            .expect("Median optimizer requires competition data. This impression has no competition data.");
        
        let predicted_offset = competition.win_rate_prediction_sigmoid_offset;
        
        // Only bid if campaign control bid is above predicted offset point
        if value <= predicted_offset {
            return None;
        }
        
        // If floor is above predicted_offset but below campaign control bid, bid with floor
        if impression.floor_cpm > predicted_offset && impression.floor_cpm < value {
            return Some(impression.floor_cpm + 0.00001);
        }
        
        // Otherwise, bid with predicted offset
        Some(predicted_offset + 0.00001)
    }
    
    fn get_optimizer_type(&self) -> String {
        "Median".to_string()
    }
}

