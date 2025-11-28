/// This is a file where campaign bidders reside
/// Bidder is a sub-component that calculates the value of an impressino to a campaign and based on it 
/// calculates the bid for the campaign.
/// 
/// In this file specifically we place bidders that take one control variable on the demand side
/// (plus possibly the sell side control variable)
/// A single controlling variable is usually seen simply as a pacing parameter, but it can be used for other purposes as well.


use crate::impressions::Impression;
use crate::sigmoid::Sigmoid;
use crate::logger::{Logger, LogEvent};
use crate::warnln;
use crate::campaign_targets::CampaignTargetTrait;


// CampaignBidderTrait trait is now in campaign.rs
pub use crate::campaign::CampaignBidderTrait;

/// Bidder for multiplicative pacing strategy
pub struct CampaignBidderMultiplicative;

impl CampaignBidderTrait for CampaignBidderMultiplicative {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, _logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "CampaignBidderMultiplicative requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        let bid = campaign_control_factor * value_to_campaign * seller_control_factor;
        
        // Don't bid if bid is below floor
        if bid < impression.floor_cpm {
            return None;
        }
        
        Some(bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Multiplicative pacing".to_string()
    }
}

/// Bidder for multiplicative pacing with additive seller control factor
pub struct CampaignBidderMultiplicativeAdditive;

impl CampaignBidderTrait for CampaignBidderMultiplicativeAdditive {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, _logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "CampaignBidderMultiplicativeAdditive requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        let bid = campaign_control_factor * value_to_campaign + seller_control_factor;
        
        // Don't bid if bid is below floor
        if bid < impression.floor_cpm {
            return None;
        }
        
        Some(bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Multiplicative additive".to_string()
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

impl CampaignBidderTrait for CampaignBidderOptimal {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "CampaignBidderOptimal requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        // Handle zero or very small campaign_control_factor to avoid division by zero
        if campaign_control_factor <= 1e-10 {
            warnln!(logger, LogEvent::Simulation, "Campaign control factor is too small, returning 0.0");
            return Some(0.0);
        }
        
        // a) Calculate marginal_utility_of_spend as 1.0 / campaign_control_factor
        // In campaign control factor converger we assume higher campaign_control_factor leads to more spend
        // but marginal utility of spend actually has to decrease to have more spend
        // so we do this non-linear transform. works well enough, but could probably be improved.
        let marginal_utility_of_spend = 1.0 / campaign_control_factor;
        
        // b) Calculate value as multiplication between seller_control_factor and impression value to campaign
        let value = seller_control_factor * value_to_campaign;
        
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
        
        if bid < impression.floor_cpm { 
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

impl CampaignBidderTrait for BidderMaxMargin {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "BidderMaxMargin requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        // Calculate full_price (maximum we're willing to pay)
        let boosted_price = campaign_control_factor * seller_control_factor * value_to_campaign;
        
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
        

        sigmoid.max_margin_bid_bisection(boosted_price, impression.floor_cpm)
    }
    
    fn get_bidding_type(&self) -> String {
        "Max margin bidding".to_string()
    }
}

/// Bidder for max margin bidding strategy with additive supply factor
/// Similar to BidderMaxMargin but uses additive supply boost: full_price = campaign_control_factor * value_to_campaign + seller_control_factor
pub struct BidderMaxMarginAdditiveSupply;

impl CampaignBidderTrait for BidderMaxMarginAdditiveSupply {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "BidderMaxMarginAdditiveSupply requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        // Calculate full_price (maximum we're willing to pay) with additive supply boost
        let boosted_price = campaign_control_factor * value_to_campaign + seller_control_factor;
        
        // Get competition data (required for max margin bidding)
        let competition = match &impression.competition {
            Some(comp) => comp,
            None => {
                warnln!(logger, LogEvent::Simulation, 
                    "Max margin bidding (additive supply) requires competition data. This impression has no competition data.");
                return None;
            }
        };
        
        // Initialize sigmoid with competition parameters
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            1.0,  // Using normalized value of 1.0
        );
        

        sigmoid.max_margin_bid_bisection(boosted_price, impression.floor_cpm)
    }
    
    fn get_bidding_type(&self) -> String {
        "Max margin bidding (additive supply)".to_string()
    }
}

/// Bidder for cheater/last look bidding strategy
pub struct CampaignBidderCheaterLastLook;

impl CampaignBidderTrait for CampaignBidderCheaterLastLook {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, _logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "CampaignBidderCheaterLastLook requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        // Calculate value as multiplication between seller_control_factor and impression value to campaign
        let max_affordable_bid = campaign_control_factor * seller_control_factor * value_to_campaign;
        
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

/// Bidder for Median Bidding strategy, sometimes called ALB (Auction Level Bid)
/// Uses multiplicative pacing, but if the bid is above the predicted offset point,
/// bids with the predicted offset point instead. Otherwise, does not bid.
/// Interesting observation from this research:
/// Median Bidding improves vs. multiplicative bidding when there is abundance of impressions
/// Median Bidding is worse than multiplicative bidding when there is scarcity of impressions and we have high fill rates
pub struct CampaignBidderMedian;

impl CampaignBidderTrait for CampaignBidderMedian {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "CampaignBidderMedian requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        // Calculate multiplicative bid
        let campaign_control_bid = campaign_control_factor * value_to_campaign * seller_control_factor;
        
        // Get competition data (required for Median Bidding)
        let competition = match &impression.competition {
            Some(comp) => comp,
            None => {
                warnln!(logger, LogEvent::Simulation, 
                    "Median Bidding requires competition data. This impression has no competition data.");
                return None;
            }
        };
        
        // Get the predicted offset point
        let predicted_offset = competition.win_rate_prediction_sigmoid_offset;
        //println!("actual offset: {:.4}, predicted offset: {:.4}", competition.win_rate_actual_sigmoid_offset, competition.win_rate_prediction_sigmoid_offset);
        // Only bid if campaign control bid is above predicted offset point
        if campaign_control_bid <= predicted_offset {
            return None;
        }
        
        // If floor is above predicted_offset but below campaign control bid, bid with floor
        if impression.floor_cpm > predicted_offset && impression.floor_cpm < campaign_control_bid {
            return Some(impression.floor_cpm);
        }
        
        // Otherwise, bid with predicted offset
        Some(predicted_offset)
    }
    
    fn get_bidding_type(&self) -> String {
        "Median Bidding".to_string()
    }
}

