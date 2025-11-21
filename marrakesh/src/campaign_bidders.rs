use crate::impressions::Impression;
use crate::sigmoid::Sigmoid;
use crate::logger::{Logger, LogEvent};
use crate::warnln;

// These are bidders that can be used by CampaignGeneral. In theory we could give them more flexibility, but 
// vast majority of strategies require just one pacing parameter, so if one needs more complex state
// one can implement a full CampaignTrait

/// Trait for campaign bidding strategies
pub trait CampaignBidder {
    /// Calculate the bid for this campaign given an impression, pacing, and seller boost factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    fn get_bid(&self, campaign_id: usize, impression: &Impression, pacing: f64, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64>;
    
    /// Get a string representation of the bidding type
    fn get_bidding_type(&self) -> String;
}

/// Bidder for multiplicative pacing strategy
pub struct CampaignBidderMultiplicativePacing;

impl CampaignBidder for CampaignBidderMultiplicativePacing {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, pacing: f64, seller_boost_factor: f64, _logger: &mut Logger) -> Option<f64> {
        Some(pacing * impression.value_to_campaign_id[campaign_id] * seller_boost_factor)
    }
    
    fn get_bidding_type(&self) -> String {
        "Multiplicative pacing".to_string()
    }
}

/// Bidder for optimal bidding strategy
/// Optimal bidding means that all bids are made at the same marginal utility of spend
/// That gives an optimal total expected value for the total expected budget
/// This is achieved by using a sigmoid function to model the win probability and then using the Newton-Raphson method to find the bid that maximizes the marginal utility of spend
/// The sigmoid function is initialized with the competition parameters and the value of the impression
/// The Newton-Raphson method is used to find the bid that keeps the marginal utility of spend constant 
/// The quantity of the marginal utility of spend is what needs to converge (for example based on target impressions or budget)
pub struct CampaignBidderOptimal;

impl CampaignBidder for CampaignBidderOptimal {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, pacing: f64, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64> {
        // Handle zero or very small pacing to avoid division by zero
        if pacing <= 1e-10 {
            warnln!(logger, LogEvent::Simulation, "Pacing is too small, returning 0.0");
            return Some(0.0);
        }
        
        // a) Calculate marginal_utility_of_spend as 1.0 / pacing
        // In pacing converger we assume higher pacing leads to more spend
        // but marginal utility of spend actually has to decrease to have more spend
        // so we do this non-linear transform. works well enough, but could probably be improved.
        let marginal_utility_of_spend = 1.0 / pacing;
        
        // b) Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let value = seller_boost_factor * impression.value_to_campaign_id[campaign_id];
        
        // Get competition data (required for optimal bidding)
        let competition = match &impression.competition {
            Some(comp) => comp,
            None => {
                warnln!(logger, LogEvent::Simulation, 
                    "Optimal bidding is only possible when competition can be modeled. This impression has no competition data.");
                return None;
            }
        };
        
        // c) Initialize sigmoid with offset and scale from impression competition predicted offset and scale, and value from value
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            value,
        );
        
        // d) Calculate the bid using marginal_utility_of_spend_inverse
        let bid = match sigmoid.marginal_utility_of_spend_inverse_numerical_2(marginal_utility_of_spend, impression.floor_cpm.max(0.0)) {
            Some(bid) => bid,
            None => {
                warnln!(logger, LogEvent::Simulation,
                    "Failed to calculate marginal_utility_of_spend_inverse. \
                    Sigmoid parameters: offset={:.2}, scale={:.2}, value={:.2}. \
                    Marginal utility of spend={:.2}. \
                    Competing bid={:.2}. \
                    Optimal bidding requires this calculation to succeed.",
                    sigmoid.offset,
                    sigmoid.scale,
                    sigmoid.value,
                    marginal_utility_of_spend,
                    competition.bid_cpm);
                return None;
            }
        };
        
        if bid < impression.floor_cpm.max(0.0) { 
            return None;
        }
        
        Some(bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Optimal bidding".to_string()
    }
}

/// Bidder for max margin bidding strategy
pub struct BidderMaxMargin;

impl CampaignBidder for BidderMaxMargin {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, pacing: f64, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64> {
        // Calculate full_price (maximum we're willing to pay)
        let full_price = pacing * seller_boost_factor * impression.value_to_campaign_id[campaign_id];
        
        // Get competition data (required for max margin bidding)
        let competition = match &impression.competition {
            Some(comp) => comp,
            None => {
                warnln!(logger, LogEvent::Simulation, 
                    "Max margin bidding requires competition data. This impression has no competition data.");
                return None;
            }
        };
        
        // Initialize sigmoid with competition parameters
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            1.0,  // Using normalized value of 1.0
        );
        
        // Find the bid that maximizes margin = P(win) * (full_price - bid)
        let min_bid = impression.floor_cpm.max(0.0);
      //  println!("min_bid: {:.4}, full_price: {:.4}", min_bid, full_price);

        sigmoid.max_margin_bid_bisection(full_price, min_bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Max margin bidding".to_string()
    }
}

/// Bidder for cheater/last look bidding strategy
pub struct CampaignBidderCheaterLastLook;

impl CampaignBidder for CampaignBidderCheaterLastLook {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, pacing: f64, seller_boost_factor: f64, _logger: &mut Logger) -> Option<f64> {
        // Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let max_affordable_bid = pacing * seller_boost_factor * impression.value_to_campaign_id[campaign_id];
        
        // Calculate minimum winning bid as minimum of floor and competing bid, plus 0.00001
        let mut minimum_winning_bid = impression.floor_cpm;
        if let Some(competition) = &impression.competition {
            minimum_winning_bid = minimum_winning_bid.max(competition.bid_cpm);
        }

        minimum_winning_bid += 0.00001;
        
        // Check if we can afford the minimum winning bid
        if max_affordable_bid < minimum_winning_bid {
            return None;
        }
        
        Some(minimum_winning_bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Cheater - last look/2nd price".to_string()
    }
}

