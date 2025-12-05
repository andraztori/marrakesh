/// This is a file where campaign bidders reside
/// Bidder is a sub-component that calculates the value of an impressino to a campaign and based on it 
/// calculates the bid for the campaign.
/// 
/// In this file specifically we place bidders that take one control variable on the demand side
/// (plus possibly the sell side control variable)
/// A single controlling variable is usually seen simply as a pacing parameter, but it can be used for other purposes as well.


use crate::impressions::Impression;
use crate::logger::Logger;
use crate::campaign_targets::CampaignTargetTrait;


// BidValuerTrait trait is now in campaign.rs
pub use crate::campaign::BidValuerTrait;

/// Bid valuer for multiplicative pacing strategy
/// This is the standard bid valuation: campaign_control_factor * value_to_campaign * seller_control_factor
pub struct BidValuerMultiplicative;

impl BidValuerTrait for BidValuerMultiplicative {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, _logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "BidValuerMultiplicative requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        let bid = campaign_control_factor * value_to_campaign * seller_control_factor;
        
        Some(bid)
    }
    
    fn get_valuer_type(&self) -> String {
        "Multiplicative pacing".to_string()
    }
}

/// Bid valuer for multiplicative pacing with additive seller control factor
/// Uses additive supply boost: campaign_control_factor * value_to_campaign + seller_control_factor
pub struct BidValuerMultiplicative_AdditiveSupply;

impl BidValuerTrait for BidValuerMultiplicative_AdditiveSupply {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, _logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "BidValuerMultiplicative_AdditiveSupply requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        let bid = campaign_control_factor * value_to_campaign + seller_control_factor;
        
        Some(bid)
    }
    
    fn get_valuer_type(&self) -> String {
        "Multiplicative additive supply".to_string()
    }
}

/// Bid valuer for multiplicative pacing with exponential supply factor
/// Uses exponential supply boost: (campaign_control_factor * value_to_campaign) ^ seller_control_factor
pub struct BidValuerMultiplicative_ExponentialSupply;

impl BidValuerTrait for BidValuerMultiplicative_ExponentialSupply {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], _converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 1, "BidValuerMultiplicative_ExponentialSupply requires exactly 1 control variable");
        let campaign_control_factor = control_variables[0];
        
        let boosted_price = (campaign_control_factor * value_to_campaign).powf(seller_control_factor);
        
        Some(boosted_price)
    }
    
    fn get_valuer_type(&self) -> String {
        "Multiplicative exponential supply".to_string()
    }
}



