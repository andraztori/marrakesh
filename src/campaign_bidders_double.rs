/// This is a file where campaign bidders reside
/// Bidder is a sub-component that calculates the value of an impressino to a campaign and based on it 
/// calculates the bid for the campaign.
/// 
/// In this file specifically we place bidders that take two control variables on the demand side
/// (plus possibly the sell side control variable)

use crate::impressions::Impression;
use crate::sigmoid::Sigmoid;
use crate::logger::{Logger, LogEvent};
use crate::warnln;
use crate::campaign_targets::CampaignTargetTrait;
use crate::campaign::CampaignBidderTrait;




/// Bidder for dual control factor bidding strategy (max margin with lambda and mu)
/// Used by CampaignGeneral for campaigns that converge to both primary and secondary targets
pub struct CampaignBidderDouble;

impl CampaignBidderTrait for CampaignBidderDouble {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 2, "CampaignBidderDouble requires exactly 2 control variables");
        // Get control variables (lambda and mu)
        let lambda = control_variables[0];
        let mu = control_variables[1];
        
        // Get secondary converge target value
        let secondary_target = converge_targets[1].get_target_value();
        
        // Calculate base_value: lambda + mu * (value_to_campaign - secondary_target)
        // for viewability lambda + mu * (viewability - targeted_viewability)
        let base_value = lambda * seller_control_factor + mu * (value_to_campaign - secondary_target);
        
        // Get competition data (required for max margin bidding)
        let competition = impression.competition.as_ref()
            .expect("Max margin bidding (dual control) requires competition data. This impression has no competition data.");
        
        // Initialize sigmoid with competition parameters
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            1.0,  // Using normalized value of 1.0
        );
        
        sigmoid.max_margin_bid_bisection(base_value, impression.floor_cpm)
    }
    
    fn get_bidding_type(&self) -> String {
        "Max margin bidding (dual control)".to_string()
    }
}

