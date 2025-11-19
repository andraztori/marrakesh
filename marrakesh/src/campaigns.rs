use crate::impressions::Impression;
use crate::converge::ControllerProportional;
pub use crate::converge::ConvergeAny;
use crate::sigmoid::Sigmoid;
use crate::logger::{Logger, LogEvent};
use crate::warnln;

/// Maximum number of campaigns supported (determines size of value_to_campaign_id array)
pub const MAX_CAMPAIGNS: usize = 10;

/// Campaign type determining the bidding strategy
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    MULTIPLICATIVE_PACING,
    OPTIMAL,
    CHEATER,
    MAX_MARGIN,
}

/// Convergence target determining what the campaign converges on
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ConvergeTarget {
    TOTAL_BUDGET { target_total_budget: f64 },
    TOTAL_IMPRESSIONS { target_total_impressions: i32 },
    NONE { default_pacing: f64 },
}


/// Architecture explanation:
/// CampaignTrait is used to describe a campaign
/// Each campaign can use a convergence mechanism that needs to store data locally (for example pacing parameter)
/// These can be anything a trait implementation wants it to be, so they need to be dnyamically created by create_converging_variables
/// Simulation then creates such converge parameters for each campaign and uses them to be able to call converge_iteration() 

/// Convergence strategy for total impressions target
pub struct ConvergeTotalImpressions {
    pub total_impressions_target: i32,
    pub controller: ControllerProportional,
}

impl ConvergeAny<crate::simulationrun::CampaignStat> for ConvergeTotalImpressions {
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let target = self.total_impressions_target as f64;
        let actual = campaign_stat.impressions_obtained as f64;
        
        self.controller.converge_next_iteration(target, actual, current_converge, next_converge)
    }
    
    fn get_converging_variable(&self, converge: &dyn crate::converge::ConvergingVariables) -> f64 {
        self.controller.get_converging_variable(converge)
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.controller.create_converging_variables()
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Fixed impressions ({}), pacing: {:.2}", self.total_impressions_target, self.get_converging_variable(converge))
    }
}

/// Convergence strategy for total budget target
pub struct ConvergeTotalBudget {
    pub total_budget_target: f64,
    pub controller: ControllerProportional,
}

impl ConvergeAny<crate::simulationrun::CampaignStat> for ConvergeTotalBudget {
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let target = self.total_budget_target;
        let actual = campaign_stat.total_buyer_charge;
        
        self.controller.converge_next_iteration(target, actual, current_converge, next_converge)
    }
    
    fn get_converging_variable(&self, converge: &dyn crate::converge::ConvergingVariables) -> f64 {
        self.controller.get_converging_variable(converge)
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.controller.create_converging_variables()
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Fixed budget ({:.2}), pacing: {:.4}", self.total_budget_target, self.get_converging_variable(converge))
    }
}

/// Convergence strategy for no convergence (fixed pacing)
pub struct ConvergeNone {
    pub default_pacing: f64,
}

impl ConvergeAny<crate::simulationrun::CampaignStat> for ConvergeNone {
    fn converge_iteration(&self, _current_converge: &dyn crate::converge::ConvergingVariables, _next_converge: &mut dyn crate::converge::ConvergingVariables, _campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        false
    }
    
    fn get_converging_variable(&self, converge: &dyn crate::converge::ConvergingVariables) -> f64 {
        converge.as_any().downcast_ref::<crate::converge::ConvergingSingleVariable>().unwrap().converging_variable
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        Box::new(crate::converge::ConvergingSingleVariable {
            converging_variable: self.default_pacing,
        })
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("No convergence, fixed pacing: {:.4}", self.get_converging_variable(converge))
    }
}

/// Trait for campaigns participating in auctions
pub trait CampaignTrait {
    /// Get the campaign ID
    fn campaign_id(&self) -> usize;
    
    /// Get the campaign name
    fn campaign_name(&self) -> &str;
    
    /// Calculate the bid for this campaign given an impression, convergence parameter, and seller boost factor
    /// Bid = converge.pacing() * impression.value_to_campaign_id[campaign_id] * seller_boost_factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::ConvergingVariables, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    

    
    
    /// Create a new convergence parameter for this campaign type
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables>;

    /// Perform one iteration of convergence, updating the next convergence parameter
    /// This method encapsulates the convergence logic for each campaign type
    /// 
    /// # Arguments
    /// * `current_converge` - Current convergence parameter (immutable)
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool;

    /// Get a string representation of the campaign type and convergence strategy
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to include pacing information
    fn type_and_converge_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String;

}

/// Campaign with multiplicative pacing
pub struct CampaignMultiplicativePacing {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::CampaignStat>>,
}

impl CampaignTrait for CampaignMultiplicativePacing {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::ConvergingVariables, seller_boost_factor: f64, _logger: &mut crate::logger::Logger) -> Option<f64> {
        let pacing = self.converger.get_converging_variable(converge);
        Some(pacing * impression.value_to_campaign_id[self.campaign_id] * seller_boost_factor)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Multiplicative pacing ({})", self.converger.converge_target_string(converge))
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
}

/// Campaign with fixed budget target using optimal bidding
/// Optimal bidding means that all bids are made at the same marginal utility of spend
/// That gives an optimal total expected value for the total expected budget
/// This is achieved by using a sigmoid function to model the win probability and then using the Newton-Raphson method to find the bid that maximizes the marginal utility of spend
/// The sigmoid function is initialized with the competition parameters and the value of the impression
/// The Newton-Raphson method is used to find the bid that keeps the marginal utility of spend constant 
/// The quantity of the marginal utility of spend is what needs to converge (for example based on target impressions or budget)
/// 

pub struct CampaignOptimalBidding {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::CampaignStat>>,
}

impl CampaignTrait for CampaignOptimalBidding {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::ConvergingVariables, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64> {
        let pacing = self.converger.get_converging_variable(converge);
        
        // Handle zero or very small pacing to avoid division by zero
        if pacing <= 1e-10 {
            println!("Pacing is too small, returning 0.0");
            return Some(0.0);
        }
        
        // a) Calculate marginal_utility_of_spend as 1.0 / pacing
        // In pacing converger we assume higher pacing leads to more spend
        // but marginal utility of spend actually has to decrease to have more spend
        // so we do this non-linear transform. works well enough, but could probably be improved.
        let marginal_utility_of_spend = 1.0 / pacing;
        
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
        let mut bid = match sigmoid.marginal_utility_of_spend_inverse_numerical_2(marginal_utility_of_spend, impression.floor_cpm.max(0.0)) {
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
//        println!("optimal bid: {:.4}", bid);
        if bid  < impression.floor_cpm.max(0.0) { 
            return None;
        }
  //      let bid = impression.floor_cpm.max(bid);
//        println!("bid: {:.4}", bid);
        Some(bid)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Optimal bidding ({})", self.converger.converge_target_string(converge))
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
}

/// Campaign with max margin bidding - finds bid that maximizes expected margin
pub struct CampaignMaxMargin {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::CampaignStat>>,
}

impl CampaignTrait for CampaignMaxMargin {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::ConvergingVariables, seller_boost_factor: f64, logger: &mut Logger) -> Option<f64> {
        let pacing = self.converger.get_converging_variable(converge);
        
        // Calculate full_price (maximum we're willing to pay)
        let full_price = pacing * seller_boost_factor * impression.value_to_campaign_id[self.campaign_id];
       // println!("full_price: {:.4}", full_price);
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
            competition.win_rate_actual_sigmoid_offset,
            competition.win_rate_actual_sigmoid_scale,
            1.0,  // Using normalized value of 1.0
        );
        
        // Find the bid that maximizes margin = P(win) * (full_price - bid)
        let min_bid = impression.floor_cpm.max(0.0);
      //  println!("min_bid: {:.4}, full_price: {:.4}", min_bid, full_price);
        sigmoid.max_margin_bid_bisection(full_price, min_bid)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Max margin bidding ({})", self.converger.converge_target_string(converge))
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
}

/// Campaign with fixed budget target using cheater bidding
pub struct CampaignCheaterLastLook {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::CampaignStat>>,
}

impl CampaignTrait for CampaignCheaterLastLook {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::ConvergingVariables, seller_boost_factor: f64, _logger: &mut Logger) -> Option<f64> {
        let pacing = self.converger.get_converging_variable(converge);
        
        // Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let max_affordable_bid = pacing * seller_boost_factor * impression.value_to_campaign_id[self.campaign_id];
        
        // Calculate minimum winning bid as minimum of floor and competing bid, plus 0.00001
        let mut minimum_winning_bid = impression.floor_cpm;
        if let Some(competition) = &impression.competition {
            minimum_winning_bid = minimum_winning_bid.max(competition.bid_cpm);
        }
//        println!("minimum_winning_bid: {:.4}", minimum_winning_bid);
        minimum_winning_bid += 0.00001;
        
        // Check if we can afford the minimum winning bid
        if max_affordable_bid < minimum_winning_bid {
            return None;
        }
        
        Some(minimum_winning_bid)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("Cheater - last look/2nd price ({})", self.converger.converge_target_string(converge))
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
}

/// Container for campaigns with methods to add campaigns
/// Uses trait objects to support different campaign types
pub struct Campaigns {
    pub campaigns: Vec<Box<dyn CampaignTrait>>,
}

impl Campaigns {
    pub fn new() -> Self {
        Self {
            campaigns: Vec::new(),
        }
    }

    /// Add a campaign to the collection
    /// 
    /// # Arguments
    /// * `campaign_name` - Name of the campaign
    /// * `campaign_type` - Type of campaign (bidding strategy)
    /// * `converge_target` - Target for convergence
    pub fn add(&mut self, campaign_name: String, campaign_type: CampaignType, converge_target: ConvergeTarget) {
        if self.campaigns.len() >= MAX_CAMPAIGNS {
            panic!(
                "Cannot add campaign: maximum number of campaigns ({}) exceeded. Current count: {}",
                MAX_CAMPAIGNS,
                self.campaigns.len()
            );
        }
        let campaign_id = self.campaigns.len();
        
        // Create converger based on converge_target
        let converger: Box<dyn ConvergeAny<crate::simulationrun::CampaignStat>> = match converge_target {
            ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions } => {
                Box::new(ConvergeTotalImpressions {
                    total_impressions_target: target_total_impressions,
                    controller: ControllerProportional::new(),
                })
            }
            ConvergeTarget::TOTAL_BUDGET { target_total_budget } => {
                Box::new(ConvergeTotalBudget {
                    total_budget_target: target_total_budget,
                    controller: ControllerProportional::new(),
                })
            }
            ConvergeTarget::NONE { default_pacing } => {
                Box::new(ConvergeNone {
                    default_pacing,
                })
            }
        };
        
        // Create campaign based on campaign_type
        match campaign_type {
            CampaignType::MULTIPLICATIVE_PACING => {
                self.campaigns.push(Box::new(CampaignMultiplicativePacing {
                    campaign_id,
                    campaign_name,
                    converger,
                }));
            }
            CampaignType::OPTIMAL => {
                self.campaigns.push(Box::new(CampaignOptimalBidding {
                    campaign_id,
                    campaign_name,
                    converger,
                }));
            }
            CampaignType::CHEATER => {
                self.campaigns.push(Box::new(CampaignCheaterLastLook {
                    campaign_id,
                    campaign_name,
                    converger,
                }));
            }
            CampaignType::MAX_MARGIN => {
                self.campaigns.push(Box::new(CampaignMaxMargin {
                    campaign_id,
                    campaign_name,
                    converger,
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converge::ConvergingSingleVariable;

    #[test]
    fn test_get_bid() {
        // Create a campaign with campaign_id = 2
        let campaign = CampaignMultiplicativePacing {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            converger: Box::new(ConvergeTotalImpressions {
                total_impressions_target: 1000,
                controller: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 0.5
        let campaign_converge: Box<dyn crate::converge::ConvergingVariables> = Box::new(ConvergingSingleVariable {
            converging_variable: 0.5,
        });

        // Create an impression with value_to_campaign_id[2] = 20.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[2] = 20.0;

        let impression = Impression {
            seller_id: 0,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_id,
            base_impression_value: 10.0,
        };

        // Expected bid = 0.5 * 20.0 * 1.0 = 10.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, &mut logger);
        assert_eq!(bid, Some(10.0));
    }

    #[test]
    fn test_get_bid_with_different_campaign_id() {
        // Create a campaign with campaign_id = 0
        let campaign = CampaignMultiplicativePacing {
            campaign_id: 0,
            campaign_name: "Test Campaign".to_string(),
            converger: Box::new(ConvergeTotalBudget {
                total_budget_target: 5000.0,
                controller: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 1.0
        let campaign_converge: Box<dyn crate::converge::ConvergingVariables> = Box::new(ConvergingSingleVariable {
            converging_variable: 1.0,
        });

        // Create an impression with value_to_campaign_id[0] = 15.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[0] = 15.0;

        let impression = Impression {
            seller_id: 1,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_id,
            base_impression_value: 10.0,
        };

        // Expected bid = 1.0 * 15.0 * 1.0 = 15.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, &mut logger);
        assert_eq!(bid, Some(15.0));
    }

    #[test]
    fn test_get_bid_with_zero_pacing() {
        // Create a campaign with campaign_id = 1
        let campaign = CampaignMultiplicativePacing {
            campaign_id: 1,
            campaign_name: "Test Campaign".to_string(),
            converger: Box::new(ConvergeTotalImpressions {
                total_impressions_target: 1000,
                controller: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 0.0
        let campaign_converge: Box<dyn crate::converge::ConvergingVariables> = Box::new(ConvergingSingleVariable {
            converging_variable: 0.0,
        });

        // Create an impression with value_to_campaign_id[1] = 100.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[1] = 100.0;

        let impression = Impression {
            seller_id: 0,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_id,
            base_impression_value: 10.0,
        };

        // Expected bid = 0.0 * 100.0 * 1.0 = 0.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, &mut logger);
        assert_eq!(bid, Some(0.0));
    }
}

