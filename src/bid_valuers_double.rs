/// This is a file where campaign bidders reside
/// Bidder is a sub-component that calculates the value of an impressino to a campaign and based on it 
/// calculates the bid for the campaign.
/// 
/// In this file specifically we place bidders that take two control variables on the demand side
/// (plus possibly the sell side control variable)

use crate::impressions::Impression;
use crate::logger::Logger;
use crate::campaign_targets::CampaignTargetTrait;
use crate::campaign::BidValuerTrait;




/// Bid valuer for dual control factor bidding strategy (max margin with lambda and mu)
/// Used by CampaignGeneral for campaigns that converge to both primary and secondary targets
pub struct BidValuerDualTarget;

impl BidValuerTrait for BidValuerDualTarget {
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64> {
        assert_eq!(control_variables.len(), 2, "BidValuerDualTarget requires exactly 2 control variables");
        // Get control variables (lambda and mu)
        let lambda = control_variables[0];
        let mu = control_variables[1];
        
        // Get secondary converge target value
        let secondary_target = converge_targets[1].get_target_value();
        
        // Calculate base_value: lambda + mu * (value_to_campaign - secondary_target)
        // for viewability lambda + mu * (viewability - targeted_viewability)
        let base_value = lambda * seller_control_factor + mu * (value_to_campaign - secondary_target);
        
        Some(base_value)
    }
    
    fn get_valuer_type(&self) -> String {
        "Max margin dual opt)".to_string()
    }
}

