use crate::impressions::Impression;
pub use crate::converge::ConvergeTargetAny;
use crate::sigmoid::Sigmoid;
use crate::logger::LogEvent;
use crate::warnln;
pub use crate::controllers::ConvergeController;


/// Maximum number of campaigns supported (determines size of value_to_campaign_id array)
pub const MAX_CAMPAIGNS: usize = 10;

/// Trait for campaign bidding strategies
pub trait CampaignBidder: Send + Sync {
    /// Calculate the bid for this campaign given an impression, controller state, converge controller, and seller boost factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    fn get_bid(&self, campaign_id: usize, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, converge_controller: &dyn crate::controllers::ConvergeController, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    
    /// Get a string representation of the bidding type
    fn get_bidding_type(&self) -> String;
}

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
/// These can be anything a trait implementation wants it to be, so they need to be dnyamically created by create_controller_state
/// Simulation then creates such converge parameters for each campaign and uses them to be able to call next_controller_state() 

/// Convergence strategy for total impressions target
pub struct ConvergeTargetTotalImpressions {
    pub total_impressions_target: i32,
}

impl ConvergeTargetAny<crate::simulationrun::CampaignStat> for ConvergeTargetTotalImpressions {
    fn get_actual_and_target(&self, campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        (campaign_stat.impressions_obtained as f64, self.total_impressions_target as f64)
    }
    
    fn converge_target_string(&self) -> String {
        format!("Fixed impressions ({})", self.total_impressions_target)
    }
}

/// Convergence strategy for total budget target
pub struct ConvergeTargetTotalBudget {
    pub total_budget_target: f64,
}

impl ConvergeTargetAny<crate::simulationrun::CampaignStat> for ConvergeTargetTotalBudget {
    fn get_actual_and_target(&self, campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        (campaign_stat.total_buyer_charge, self.total_budget_target)
    }
    
    fn converge_target_string(&self) -> String {
        format!("Fixed budget target: {:.2}", self.total_budget_target)
    }
}

/// Convergence strategy for no convergence (fixed pacing)
pub struct ConvergeNone;

impl ConvergeTargetAny<crate::simulationrun::CampaignStat> for ConvergeNone {
    fn get_actual_and_target(&self, _campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        // No convergence, so no target or actual values
        (0.0, 0.0)
    }
    
    fn converge_target_string(&self) -> String {
        "No convergence target".to_string()
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
    fn get_bid(&self, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    

    
    
    /// Create a new convergence parameter for this campaign type
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState>;

    /// Perform one iteration of convergence, updating the next convergence parameter
    /// This method encapsulates the convergence logic for each campaign type
    /// 
    /// # Arguments
    /// * `previous_state` - Previous controller state (immutable)
    /// * `next_state` - Next controller state to be updated (mutable)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, campaign_stat: &crate::simulationrun::CampaignStat) -> bool;

    /// Get a string representation of the campaign type and convergence strategy
    /// 
    /// # Arguments
    /// * `controller_state` - Controller state to include pacing information
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String;

}

/// Bidder for multiplicative pacing strategy
pub struct BidderMultiplicativePacing;

impl CampaignBidder for BidderMultiplicativePacing {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, converge_controller: &dyn crate::controllers::ConvergeController, seller_boost_factor: f64, _logger: &mut crate::logger::Logger) -> Option<f64> {
        let pacing = converge_controller.get_control_variable(controller_state);
        Some(pacing * impression.value_to_campaign_id[campaign_id] * seller_boost_factor)
    }
    
    fn get_bidding_type(&self) -> String {
        "Multiplicative pacing".to_string()
    }
}


/// General campaign structure that can use any bidding strategy
pub struct CampaignGeneral {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
    pub converge_controller: Box<dyn crate::controllers::ConvergeController>,
    pub bidder: Box<dyn CampaignBidder>,
}

impl CampaignTrait for CampaignGeneral {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        self.bidder.get_bid(self.campaign_id, impression, controller_state, self.converge_controller.as_ref(), seller_boost_factor, logger)
    }
    
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let (actual, target) = self.converge_target.get_actual_and_target(campaign_stat);
        self.converge_controller.next_controller_state(previous_state, next_state, actual, target)
    }
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String {
        format!("{} ({}, {})", self.bidder.get_bidding_type(), self.converge_target.converge_target_string(), self.converge_controller.controller_string(controller_state))
    }
    
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState> {
        self.converge_controller.create_controller_state()
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
    fn get_bid(&self, campaign_id: usize, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, converge_controller: &dyn crate::controllers::ConvergeController, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        let pacing = converge_controller.get_control_variable(controller_state);
        
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
//        println!("optimal bid: {:.4}", bid);
        if bid < impression.floor_cpm.max(0.0) { 
            return None;
        }
  //      let bid = impression.floor_cpm.max(bid);
//        println!("bid: {:.4}", bid);
        Some(bid)
    }
    
    fn get_bidding_type(&self) -> String {
        "Optimal bidding".to_string()
    }
}


/// Bidder for max margin bidding strategy
pub struct BidderMaxMargin;

impl CampaignBidder for BidderMaxMargin {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, converge_controller: &dyn crate::controllers::ConvergeController, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        let pacing = converge_controller.get_control_variable(controller_state);
        
        // Calculate full_price (maximum we're willing to pay)
        let full_price = pacing * seller_boost_factor * impression.value_to_campaign_id[campaign_id];
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
pub struct BidderCheaterLastLook;

impl CampaignBidder for BidderCheaterLastLook {
    fn get_bid(&self, campaign_id: usize, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, converge_controller: &dyn crate::controllers::ConvergeController, seller_boost_factor: f64, _logger: &mut crate::logger::Logger) -> Option<f64> {
        let pacing = converge_controller.get_control_variable(controller_state);
        
        // Calculate value as multiplication between seller_boost_factor and impression value to campaign id
        let max_affordable_bid = pacing * seller_boost_factor * impression.value_to_campaign_id[campaign_id];
        
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
    
    fn get_bidding_type(&self) -> String {
        "Cheater - last look/2nd price".to_string()
    }
}

/// Campaign with fixed budget target using cheater bidding
pub struct CampaignCheaterLastLook {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
    pub converge_controller: Box<dyn crate::controllers::ConvergeController>,
    pub bidder: Box<dyn CampaignBidder>,
}

impl CampaignTrait for CampaignCheaterLastLook {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        self.bidder.get_bid(self.campaign_id, impression, controller_state, self.converge_controller.as_ref(), seller_boost_factor, logger)
    }
    
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let (actual, target) = self.converge_target.get_actual_and_target(campaign_stat);
        self.converge_controller.next_controller_state(previous_state, next_state, actual, target)
    }
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String {
        format!("{} ({}, {})", self.bidder.get_bidding_type(), self.converge_target.converge_target_string(), self.converge_controller.controller_string(controller_state))
    }
    
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState> {
        self.converge_controller.create_controller_state()
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
        
        // Create converge_target and converge_controller based on converge_target
        let (converge_target_box, converge_controller): (Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>, Box<dyn crate::controllers::ConvergeController>) = match converge_target {
            ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions } => {
                (
                    Box::new(ConvergeTargetTotalImpressions {
                        total_impressions_target: target_total_impressions,
                    }),
                    Box::new(crate::controllers::ConvergeControllerProportional::new())
                )
            }
            ConvergeTarget::TOTAL_BUDGET { target_total_budget } => {
                (
                    Box::new(ConvergeTargetTotalBudget {
                        total_budget_target: target_total_budget,
                    }),
                    Box::new(crate::controllers::ConvergeControllerProportional::new())
                )
            }
            ConvergeTarget::NONE { default_pacing } => {
                (
                    Box::new(ConvergeNone),
                    Box::new(crate::controllers::ConvergeControllerConstant::new(default_pacing))
                )
            }
        };
        
        // Create campaign based on campaign_type
        match campaign_type {
            CampaignType::MULTIPLICATIVE_PACING => {
                let bidder = Box::new(BidderMultiplicativePacing) as Box<dyn CampaignBidder>;
                self.campaigns.push(Box::new(CampaignGeneral {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::OPTIMAL => {
                let bidder = Box::new(CampaignBidderOptimal) as Box<dyn CampaignBidder>;
                self.campaigns.push(Box::new(CampaignGeneral {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::CHEATER => {
                let bidder = Box::new(BidderCheaterLastLook) as Box<dyn CampaignBidder>;
                self.campaigns.push(Box::new(CampaignCheaterLastLook {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::MAX_MARGIN => {
                let bidder = Box::new(BidderMaxMargin) as Box<dyn CampaignBidder>;
                self.campaigns.push(Box::new(CampaignGeneral {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
        use crate::controllers::ControllerStateSingleVariable;

    #[test]
    fn test_get_bid() {
        // Create a campaign with campaign_id = 2
        let bidder = Box::new(BidderMultiplicativePacing) as Box<dyn CampaignBidder>;
        let campaign = CampaignGeneral {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            converge_target: Box::new(ConvergeTargetTotalImpressions {
                total_impressions_target: 1000,
            }),
            converge_controller: Box::new(crate::controllers::ConvergeControllerConstant::new(1.0)),
            bidder,
        };

        // Create a campaign converge with pacing = 0.5
        let campaign_converge: Box<dyn crate::controllers::ControllerState> = Box::new(ControllerStateSingleVariable {
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
        let bidder = Box::new(BidderMultiplicativePacing) as Box<dyn CampaignBidder>;
        let campaign = CampaignGeneral {
            campaign_id: 0,
            campaign_name: "Test Campaign".to_string(),
            converge_target: Box::new(ConvergeTargetTotalBudget {
                total_budget_target: 5000.0,
            }),
            converge_controller: Box::new(crate::controllers::ConvergeControllerConstant::new(1.0)),
            bidder,
        };

        // Create a campaign converge with pacing = 1.0
        let campaign_converge: Box<dyn crate::controllers::ControllerState> = Box::new(ControllerStateSingleVariable {
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
        let bidder = Box::new(BidderMultiplicativePacing) as Box<dyn CampaignBidder>;
        let campaign = CampaignGeneral {
            campaign_id: 1,
            campaign_name: "Test Campaign".to_string(),
            converge_target: Box::new(ConvergeTargetTotalImpressions {
                total_impressions_target: 1000,
            }),
            converge_controller: Box::new(crate::controllers::ConvergeControllerConstant::new(1.0)),
            bidder,
        };

        // Create a campaign converge with pacing = 0.0
        let campaign_converge: Box<dyn crate::controllers::ControllerState> = Box::new(ControllerStateSingleVariable {
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

    #[test]
    fn test_converge_target_none() {
        // Test creating a campaign with ConvergeTarget::NONE
        let mut campaigns = Campaigns::new();
        campaigns.add(
            "Fixed Pacing Campaign".to_string(),
            CampaignType::MULTIPLICATIVE_PACING,
            ConvergeTarget::NONE { default_pacing: 0.75 },
        );

        assert_eq!(campaigns.campaigns.len(), 1);
        let campaign = &campaigns.campaigns[0];

        // Test that ConvergeNone works correctly
        // Test create_controller_state returns the default pacing
        let converge_vars = campaign.create_controller_state();
        let pacing = campaign.converge_controller.get_control_variable(converge_vars.as_ref());
        assert_eq!(pacing, 0.75);

        // Test that next_controller_state always returns false (no convergence)
        let campaign_stat = crate::simulationrun::CampaignStat {
            impressions_obtained: 100,
            total_buyer_charge: 50.0,
            total_value: 200.0,
        };
        let mut next_state = campaign.create_controller_state();
        let converged = campaign.next_controller_state(converge_vars.as_ref(), next_state.as_mut(), &campaign_stat);
        assert_eq!(converged, false);

        // Test that pacing remains unchanged after next_controller_state
        let pacing_after = campaign.converge_controller.get_control_variable(next_state.as_ref());
        assert_eq!(pacing_after, 0.75);

        // Test that bidding works correctly with fixed pacing
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[0] = 30.0;

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

        // Expected bid = 0.75 * 30.0 * 1.0 = 22.5
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, converge_vars.as_ref(), 1.0, &mut logger);
        assert_eq!(bid, Some(22.5));
    }
}


