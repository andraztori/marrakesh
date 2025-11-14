use crate::impressions::Impression;
use crate::campaigns::{CampaignTrait, CampaignConverge};
use crate::converge::ConvergingParam;
use crate::sigmoid::Sigmoid;
use crate::logger::{Logger, LogEvent};
use crate::warnln;

/// Campaign with fixed budget target using optimal bidding
pub struct CampaignFixedBudgetOptimalBidding {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_strategy: Box<dyn CampaignConverge>,
}

impl CampaignTrait for CampaignFixedBudgetOptimalBidding {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::Converge, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64> {
        let converge = converge.as_any().downcast_ref::<ConvergingParam>().unwrap();
        
        // Handle zero or very small pacing to avoid division by zero
        if converge.converging_param <= 1e-10 {
            return Some(0.0);
        }
        
        // a) Calculate marginal_utility_of_spend as 1.0 / converge.converging_param
        // In pacing converger we assume higher pacing leads to more spend
        // but marginal utility of spend actually has to decrease to have more spend
        // so we do this non-linear transform. works well enough, but could probably be improved.
        let marginal_utility_of_spend = 1.0 / converge.converging_param;
        
        // b) Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let value = seller_boost_factor * impression.value_to_campaign_id[self.campaign_id];
        
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
        let bid = match sigmoid.marginal_utility_of_spend_inverse(marginal_utility_of_spend) {
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
                    competition.bid_cpm
                );
                return None;
            }
        };
        
        let _probability = sigmoid.get_probability(bid);
        //println!("Final bid: {:.4}, offset: {:.4}, scale: {:.4}, value: {:.4}, competing_bid: {:.4}, probability: {:.4}", 
        //         bid, sigmoid.offset, sigmoid.scale, sigmoid.value, competition.bid_cpm, _probability);
        Some(bid)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converge_strategy.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::Converge) -> String {
        format!("Optimal bidding ({})", self.converge_strategy.converge_target_string(converge))
    }
    
    fn create_converge(&self) -> Box<dyn crate::converge::Converge> {
        Box::new(ConvergingParam { converging_param: 1.0 })
    }
}

