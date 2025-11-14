use crate::impressions::Impression;
use crate::utils::ControllerProportional;
use crate::campaigns::{CampaignTrait, CampaignConverge, CampaignConvergePacing};
use crate::sigmoid::Sigmoid;

/// Campaign with fixed budget target using optimal bidding
pub struct CampaignFixedBudgetOptimalBidding {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub total_budget_target: f64,
    pub pacing_converger: ControllerProportional,
}

impl CampaignTrait for CampaignFixedBudgetOptimalBidding {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn CampaignConverge, seller_boost_factor: f64) -> f64 {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        
        // Handle zero or very small pacing to avoid division by zero
        if converge.pacing <= 1e-10 {
            return 0.0;
        }
        
        // a) Calculate marginal_utility_of_spend as 1.0 / converge.pacing
        let marginal_utility_of_spend = 1.0 / converge.pacing;
        
        // b) Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let value = seller_boost_factor * impression.value_to_campaign_id[self.campaign_id];
        
        // Get competition data (required for optimal bidding)
        let competition = match &impression.competition {
            Some(comp) => comp,
            None => {
                panic!("Optimal bidding is only possible when competition can be modeled. This impression has no competition data.");
            }
        };
        
        // c) Initialize sigmoid with offset and scale from impression competition predicted offset and scale, and value from value
        let sigmoid = Sigmoid::new(
            competition.win_rate_prediction_sigmoid_offset,
            competition.win_rate_prediction_sigmoid_scale,
            value,
        );
        
        // d) Calculate the bid using marginal_utility_of_spend_inverse
        let bid = match sigmoid.marginal_utility_of_spend_inverse(marginal_utility_of_spend) {
            Some(bid) => bid,
            None => {
                panic!(
                    "Failed to calculate marginal_utility_of_spend_inverse. \
                    Sigmoid parameters: scale={}, offset={}, value={}. \
                    Marginal utility of spend={}. \
                    Competing bid={}. \
                    Optimal bidding requires this calculation to succeed.",
                    sigmoid.scale,
                    sigmoid.offset,
                    sigmoid.value,
                    marginal_utility_of_spend,
                    competition.bid_cpm
                );
            }
        };
        
        println!("Final bid: {:.6}, offset: {:.6}, scale: {:.6}, value: {:.6}, competing_bid: {:.6}", bid, sigmoid.offset, sigmoid.scale, sigmoid.value, competition.bid_cpm);
        bid
    }
    
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        // Downcast to concrete types at the beginning
        let current_converge = current_converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        let next_converge = next_converge.as_any_mut().downcast_mut::<CampaignConvergePacing>().unwrap();
        
        let target = self.total_budget_target;
        let actual = campaign_stat.total_buyer_charge;
        let current_pacing = current_converge.pacing;
        
        let (new_pacing, changed) = self.pacing_converger.pacing_in_next_iteration(target, actual, current_pacing);
        next_converge.pacing = new_pacing;
        
        changed
    }
    
    fn type_and_target_string(&self) -> String {
        format!("FIXED_BUDGET_OPTIMAL_BIDDING (target: {:.2})", self.total_budget_target)
    }
    
    fn converge_string(&self, converge: &dyn CampaignConverge) -> String {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        format!("Pacing: {:.2}", converge.pacing)
    }
    
    fn create_converge(&self) -> Box<dyn CampaignConverge> {
        Box::new(CampaignConvergePacing { pacing: 1.0 })
    }
}

