use crate::impressions::Impression;
pub use crate::converge::ConvergeTargetAny;
pub use crate::controllers::{ConvergeController, ConvergeControllerDouble};
pub use crate::controller_state::ControllerState;
pub use crate::campaign_bidders::CampaignBidderSingle;
use crate::sigmoid::Sigmoid;
use crate::logger::{LogEvent};
use crate::warnln;



/// Campaign type determining the bidding strategy
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    MULTIPLICATIVE_PACING,
    MULTIPLICATIVE_ADDITIVE,
    OPTIMAL,
    CHEATER,
    MAX_MARGIN,
    MAX_MARGIN_DOUBLE_TARGET,
    ALB,
}

/// Convergence target determining what the campaign converges on
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ConvergeTarget {
    TOTAL_BUDGET { target_total_budget: f64 },
    TOTAL_IMPRESSIONS { target_total_impressions: i32 },
    AVG_VALUE { avg_impression_value_to_campaign: f64 },
    NONE { default_pacing: f64 },
}


/// Architecture explanation:
/// CampaignTrait is used to describe a campaign
/// Each campaign can use a convergence mechanism that needs to store data locally (for example pacing parameter)
/// These can be anything a trait implementation wants it to be, so they need to be dnyamically created by create_controller_state
/// Simulation then creates such converge parameters for each campaign and uses them to be able to call next_controller_state() 

// Re-export convergence target types for convenience
pub use crate::campaign_targets::{ConvergeTargetTotalImpressions, ConvergeTargetTotalBudget, ConvergeTargetAvgValue, ConvergeNone};


/// Trait for campaigns participating in auctions
pub trait CampaignTrait {
    /// Get the campaign ID
    fn campaign_id(&self) -> usize;
    
    /// Get the campaign name
    fn campaign_name(&self) -> &str;
    
    /// Calculate the bid for this campaign given an impression, convergence parameter, and seller control factor
    /// Bid = campaign_control_factor * value_to_campaign * seller_control_factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    fn get_bid(&self, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, seller_control_factor: f64, value_to_campaign: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    
  
    
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

/// While in theory one can write any kind of campaign, in practice it is possible to break it down to key elements
/// that can operate separately: 
/// - what outcome is the campaign looking to converge to
/// - what is the controller taking care of convergence
/// - what is the bidding (pricing) strategy
/// Single means a single demand side control parameter
pub struct CampaignSingle {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
    pub converge_controller: Box<dyn crate::controllers::ConvergeController>,
    pub bidder: Box<dyn CampaignBidderSingle>,
}

impl CampaignTrait for CampaignSingle {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, controller_state: &dyn crate::controllers::ControllerState, seller_control_factor: f64, value_to_campaign: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        let campaign_control_factor = self.converge_controller.get_control_variable(controller_state);
        // Pass the value_to_campaign to the bidder
        self.bidder.get_bid(value_to_campaign, impression, campaign_control_factor, seller_control_factor, logger)
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

// Re-export bidder types for convenience
pub use crate::campaign_bidders::{CampaignBidderMultiplicative, CampaignBidderMultiplicativeAdditive, CampaignBidderOptimal, BidderMaxMargin, CampaignBidderCheaterLastLook};

/// Campaign type that converges to both primary and secondary targets
/// Based on CampaignSingle but with dual converge targets
pub struct CampaignDouble {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_target_primary: Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
    pub converge_target_secondary: Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
    pub converge_controller: Box<dyn ConvergeControllerDouble>,
    pub bidder: Box<dyn CampaignBidderSingle>,
}

impl CampaignTrait for CampaignDouble {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, controller_state: &dyn ControllerState, seller_control_factor: f64, value_to_campaign: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        // Save primary control variable as lambda, secondary as mu
        let lambda = self.converge_controller.get_control_variable_primary(controller_state);
        let mu = self.converge_controller.get_control_variable_secondary(controller_state);
        
        // Get secondary converge target value
        let secondary_target = self.converge_target_secondary.get_target_value();
        
        // Calculate base_value: lambda + mu * (value_to_campaign - secondary_target)
        // for viwablity lambda + mu * (viewability - targeted_viability)

        let base_value = lambda * seller_control_factor + mu * (value_to_campaign - secondary_target);
        
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
        

        sigmoid.max_margin_bid_bisection(base_value, impression.floor_cpm)
    }
    
    fn next_controller_state(&self, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let (actual_primary, target_primary) = self.converge_target_primary.get_actual_and_target(campaign_stat);
        let (actual_secondary, target_secondary) = self.converge_target_secondary.get_actual_and_target(campaign_stat);
        self.converge_controller.next_controller_state(
            previous_state,
            next_state,
            actual_primary,
            target_primary,
            actual_secondary,
            target_secondary,
        )
    }
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn ControllerState) -> String {
        format!(
            "{} ({}, Secondary: {}, {})",
            self.bidder.get_bidding_type(),
            self.converge_target_primary.converge_target_string(),
            self.converge_target_secondary.converge_target_string(),
            self.converge_controller.controller_string(controller_state)
        )
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        self.converge_controller.create_controller_state()
    }
}

/// Container for campaigns with methods to add campaigns
/// Uses trait objects to support different campaign types
pub struct Campaigns {
    pub campaigns: Vec<Box<dyn CampaignTrait>>,
    pub value_groups: Vec<Vec<usize>>,
    pub campaign_to_value_group_mapping: Vec<usize>,
}

impl Campaigns {
    pub fn new() -> Self {
        Self {
            campaigns: Vec::new(),
            value_groups: Vec::new(),
            campaign_to_value_group_mapping: Vec::new(),
        }
    }

    /// Convert a ConvergeTarget into a converge target box and controller
    /// 
    /// # Arguments
    /// * `converge_target` - The convergence target to convert
    /// 
    /// # Returns
    /// A tuple of (converge_target_box, converge_controller)
    fn convert_converge_target(
        converge_target: ConvergeTarget,
    ) -> (
        Box<dyn ConvergeTargetAny<crate::simulationrun::CampaignStat>>,
        Box<dyn crate::controllers::ConvergeController>,
    ) {
        match converge_target {
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
            ConvergeTarget::AVG_VALUE { avg_impression_value_to_campaign } => {
                (
                    Box::new(ConvergeTargetAvgValue {
                        avg_impression_value_to_campaign: avg_impression_value_to_campaign,
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
        }
    }

    /// Add a campaign to the collection
    /// 
    /// # Arguments
    /// * `campaign_name` - Name of the campaign
    /// * `campaign_type` - Type of campaign (bidding strategy)
    /// * `converge_targets` - Vector of targets for convergence
    /// 
    /// # Returns
    /// The campaign_id of the just added campaign
    pub fn add(&mut self, campaign_name: String, campaign_type: CampaignType, converge_targets: Vec<ConvergeTarget>) -> usize {
        // No limit on number of campaigns
        let campaign_id = self.campaigns.len();
        
        // Create campaign based on campaign_type
        match campaign_type {
            CampaignType::MULTIPLICATIVE_PACING => {
                assert_eq!(converge_targets.len(), 1, "MULTIPLICATIVE_PACING requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(CampaignBidderMultiplicative) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::MULTIPLICATIVE_ADDITIVE => {
                assert_eq!(converge_targets.len(), 1, "MULTIPLICATIVE_ADDITIVE requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(CampaignBidderMultiplicativeAdditive) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::OPTIMAL => {
                assert_eq!(converge_targets.len(), 1, "OPTIMAL requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(CampaignBidderOptimal) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::CHEATER => {
                assert_eq!(converge_targets.len(), 1, "CHEATER requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(CampaignBidderCheaterLastLook) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::MAX_MARGIN => {
                assert_eq!(converge_targets.len(), 1, "MAX_MARGIN requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(BidderMaxMargin) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::MAX_MARGIN_DOUBLE_TARGET => {
                assert_eq!(converge_targets.len(), 2, "MAX_MARGIN_DOUBLE_TARGET requires exactly two converge targets");
                let (converge_target_primary, _) = Self::convert_converge_target(converge_targets[0].clone());
                let (converge_target_secondary, _) = Self::convert_converge_target(converge_targets[1].clone());
                let bidder = Box::new(BidderMaxMargin) as Box<dyn CampaignBidderSingle>;
                let converge_controller = Box::new(crate::controllers::ConvergeDoubleProportionalController::new()) as Box<dyn crate::controllers::ConvergeControllerDouble>;
                self.campaigns.push(Box::new(CampaignDouble {
                    campaign_id,
                    campaign_name,
                    converge_target_primary,
                    converge_target_secondary,
                    converge_controller,
                    bidder,
                }));
            }
            CampaignType::ALB => {
                assert_eq!(converge_targets.len(), 1, "ALB requires exactly one converge target");
                let (converge_target_box, converge_controller) = Self::convert_converge_target(converge_targets[0].clone());
                let bidder = Box::new(crate::campaign_bidders::CampaignBidderALB) as Box<dyn CampaignBidderSingle>;
                self.campaigns.push(Box::new(CampaignSingle {
                    campaign_id,
                    campaign_name,
                    converge_target: converge_target_box,
                    converge_controller,
                    bidder,
                }));
            }
        }
        
        campaign_id
    }
    
    /// Create a value group by appending a vector of campaign IDs
    /// 
    /// # Arguments
    /// * `campaign_ids` - Vector of campaign IDs to add as a group
    /// 
    /// # Panics
    /// Panics if any campaign_id is invalid (not between 0 and num_campaigns) or
    /// if any campaign is already in another group
    pub fn create_value_group(&mut self, campaign_ids: Vec<usize>) {
        let num_campaigns = self.campaigns.len();
        
        // Check that all campaign IDs are valid
        for &campaign_id in &campaign_ids {
            if campaign_id >= num_campaigns {
                panic!(
                    "Invalid campaign_id {}: must be between 0 and {} (num_campaigns)",
                    campaign_id, num_campaigns
                );
            }
        }
        
        // Check that no campaign is already in any group
        for &campaign_id in &campaign_ids {
            for group in &self.value_groups {
                if group.contains(&campaign_id) {
                    panic!(
                        "Campaign {} is already in a value group. Cannot add it to another group.",
                        campaign_id
                    );
                }
            }
        }
        
        self.value_groups.push(campaign_ids);
    }
    
    /// Finalize group mappings for all campaigns
    /// 
    /// For each campaign:
    /// - If it is in any group, write that group index to campaign_to_value_group_mapping
    /// - If it is not in any group, assign a new group mapping starting with indexes of all groups + 1
    pub fn finalize_groups(&mut self) {
        let num_campaigns = self.campaigns.len();
        
        // Initialize mapping with a sentinel value to track unassigned campaigns
        self.campaign_to_value_group_mapping = vec![usize::MAX; num_campaigns];
        
        // First pass: assign campaigns that are in groups
        for (group_index, group) in self.value_groups.iter().enumerate() {
            for &campaign_id in group {
                self.campaign_to_value_group_mapping[campaign_id] = group_index;
            }
        }
        
        // Second pass: assign new group indices to campaigns not in any group
        // Also create new groups in value_groups for each ungrouped campaign
        for campaign_id in 0..num_campaigns {
            if self.campaign_to_value_group_mapping[campaign_id] == usize::MAX {
                // Create a new group containing just this campaign
                self.value_groups.push(vec![campaign_id]);
                // The group index is the last index in value_groups (length - 1)
                self.campaign_to_value_group_mapping[campaign_id] = self.value_groups.len() - 1;
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
        let bidder = Box::new(CampaignBidderMultiplicative) as Box<dyn CampaignBidderSingle>;
        let campaign = CampaignSingle {
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

        // Create an impression with value_to_campaign_group[0] = 20.0
        let value_to_campaign_group = vec![20.0];

        let impression = Impression {
            seller_id: 0,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_group,
            base_impression_value: 10.0,
        };

        // Expected bid = 0.5 * 20.0 * 1.0 = 10.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, 20.0, &mut logger);
        assert_eq!(bid, Some(10.0));
    }

    #[test]
    fn test_get_bid_with_different_campaign_id() {
        // Create a campaign with campaign_id = 0
        let bidder = Box::new(CampaignBidderMultiplicative) as Box<dyn CampaignBidderSingle>;
        let campaign = CampaignSingle {
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

        // Create an impression with value_to_campaign_group[0] = 15.0
        let value_to_campaign_group = vec![15.0];

        let impression = Impression {
            seller_id: 1,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_group,
            base_impression_value: 10.0,
        };

        // Expected bid = 1.0 * 15.0 * 1.0 = 15.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, 15.0, &mut logger);
        assert_eq!(bid, Some(15.0));
    }

    #[test]
    fn test_get_bid_with_zero_pacing() {
        // Create a campaign with campaign_id = 1
        let bidder = Box::new(CampaignBidderMultiplicative) as Box<dyn CampaignBidderSingle>;
        let campaign = CampaignSingle {
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

        // Create an impression with value_to_campaign_group[0] = 100.0
        let value_to_campaign_group = vec![100.0];

        let impression = Impression {
            seller_id: 0,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_group,
            base_impression_value: 10.0,
        };

        // Expected bid = 0.0 * 100.0 * 1.0 = 0.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, 100.0, &mut logger);
        assert_eq!(bid, Some(0.0));
    }

    #[test]
    fn test_converge_target_none() {
        // Test creating a campaign with ConvergeTarget::NONE
        let mut campaigns = Campaigns::new();
        campaigns.add(
            "Fixed Pacing Campaign".to_string(),
            CampaignType::MULTIPLICATIVE_PACING,
            vec![ConvergeTarget::NONE { default_pacing: 0.75 }],
        );

        assert_eq!(campaigns.campaigns.len(), 1);
        let campaign = &campaigns.campaigns[0];

        // Test that ConvergeNone works correctly
        // Test create_controller_state returns the default pacing
        let converge_vars = campaign.create_controller_state();
        // Use ControllerStateSingleVariable to extract the pacing value
        if let Some(state) = converge_vars.as_any().downcast_ref::<crate::controllers::ControllerStateSingleVariable>() {
            assert_eq!(state.converging_variable, 0.75);
        } else {
            panic!("Expected ControllerStateSingleVariable");
        }

        // Test that next_controller_state always returns false (no convergence)
        let campaign_stat = crate::simulationrun::CampaignStat {
            impressions_obtained: 100.0,
            total_supply_cost: 0.0,
            total_virtual_cost: 0.0,
            total_buyer_charge: 50.0,
            total_value: 200.0,
        };
        let mut next_state = campaign.create_controller_state();
        let converged = campaign.next_controller_state(converge_vars.as_ref(), next_state.as_mut(), &campaign_stat);
        assert_eq!(converged, false);

        // Test that pacing remains unchanged after next_controller_state
        if let Some(state) = next_state.as_any().downcast_ref::<crate::controllers::ControllerStateSingleVariable>() {
            assert_eq!(state.converging_variable, 0.75);
        } else {
            panic!("Expected ControllerStateSingleVariable");
        }

        // Test that bidding works correctly with fixed pacing
        let value_to_campaign_group = vec![30.0];

        let impression = Impression {
            seller_id: 0,
            competition: Some(crate::competition::ImpressionCompetition {
                bid_cpm: 0.0,
                win_rate_actual_sigmoid_offset: 0.0,
                win_rate_actual_sigmoid_scale: 0.0,
                win_rate_prediction_sigmoid_offset: 0.0,
                win_rate_prediction_sigmoid_scale: 0.0,
            }),
            floor_cpm: 0.0,
            value_to_campaign_group,
            base_impression_value: 10.0,
        };

        // Expected bid = 0.75 * 30.0 * 1.0 = 22.5
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, converge_vars.as_ref(), 1.0, 30.0, &mut logger);
        assert_eq!(bid, Some(22.5));
    }

    #[test]
    fn test_create_value_group_success() {
        let mut campaigns = Campaigns::new();
        
        // Add some campaigns
        campaigns.add("Campaign 0".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        campaigns.add("Campaign 1".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        campaigns.add("Campaign 2".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        
        // Create a value group with campaigns 0 and 1
        campaigns.create_value_group(vec![0, 1]);
        
        assert_eq!(campaigns.value_groups.len(), 1);
        assert_eq!(campaigns.value_groups[0], vec![0, 1]);
    }

    #[test]
    #[should_panic(expected = "Invalid campaign_id")]
    fn test_create_value_group_invalid_campaign_id() {
        let mut campaigns = Campaigns::new();
        
        // Add only one campaign (ID 0)
        campaigns.add("Campaign 0".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        
        // Try to create a group with invalid campaign_id (1 doesn't exist)
        campaigns.create_value_group(vec![0, 1]);
    }

    #[test]
    #[should_panic(expected = "already in a value group")]
    fn test_create_value_group_duplicate_campaign() {
        let mut campaigns = Campaigns::new();
        
        // Add campaigns
        campaigns.add("Campaign 0".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        campaigns.add("Campaign 1".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        campaigns.add("Campaign 2".to_string(), CampaignType::MULTIPLICATIVE_PACING, vec![ConvergeTarget::NONE { default_pacing: 1.0 }]);
        
        // Create first group with campaign 0
        campaigns.create_value_group(vec![0]);
        
        // Try to create another group with campaign 0 (should panic)
        campaigns.create_value_group(vec![0, 1]);
    }

    #[test]
    fn test_create_value_group_multiple_groups() {
        let mut campaigns = Campaigns::new();
        
        // Add 5 campaigns
        for i in 0..5 {
            campaigns.add(
                format!("Campaign {}", i),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::NONE { default_pacing: 1.0 }]
            );
        }
        
        // Create multiple groups
        campaigns.create_value_group(vec![0, 1]);
        campaigns.create_value_group(vec![2, 3]);
        
        assert_eq!(campaigns.value_groups.len(), 2);
        assert_eq!(campaigns.value_groups[0], vec![0, 1]);
        assert_eq!(campaigns.value_groups[1], vec![2, 3]);
    }

    #[test]
    fn test_finalize_groups_with_grouped_campaigns() {
        let mut campaigns = Campaigns::new();
        
        // Add 4 campaigns
        for i in 0..4 {
            campaigns.add(
                format!("Campaign {}", i),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::NONE { default_pacing: 1.0 }]
            );
        }
        
        // Create groups: [0, 1] and [2]
        campaigns.create_value_group(vec![0, 1]);
        campaigns.create_value_group(vec![2]);
        
        // Finalize groups
        campaigns.finalize_groups();
        
        // Check mappings:
        // Campaign 0 -> group 0
        // Campaign 1 -> group 0
        // Campaign 2 -> group 1
        // Campaign 3 -> group 2 (new group, since num_groups = 2, starts at 2)
        assert_eq!(campaigns.campaign_to_value_group_mapping.len(), 4);
        assert_eq!(campaigns.campaign_to_value_group_mapping[0], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[1], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[2], 1);
        assert_eq!(campaigns.campaign_to_value_group_mapping[3], 2);
    }

    #[test]
    fn test_finalize_groups_with_no_groups() {
        let mut campaigns = Campaigns::new();
        
        // Add 3 campaigns but don't create any groups
        for i in 0..3 {
            campaigns.add(
                format!("Campaign {}", i),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::NONE { default_pacing: 1.0 }]
            );
        }
        
        // Finalize groups (no groups exist, so all should get new indices starting from 0)
        campaigns.finalize_groups();
        
        // Check mappings: all campaigns should get new group indices starting from 0
        assert_eq!(campaigns.campaign_to_value_group_mapping.len(), 3);
        assert_eq!(campaigns.campaign_to_value_group_mapping[0], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[1], 1);
        assert_eq!(campaigns.campaign_to_value_group_mapping[2], 2);
        
        // Verify that new groups were added to value_groups
        assert_eq!(campaigns.value_groups.len(), 3);
        assert_eq!(campaigns.value_groups[0], vec![0]);
        assert_eq!(campaigns.value_groups[1], vec![1]);
        assert_eq!(campaigns.value_groups[2], vec![2]);
    }

    #[test]
    fn test_finalize_groups_mixed() {
        let mut campaigns = Campaigns::new();
        
        // Add 6 campaigns
        for i in 0..6 {
            campaigns.add(
                format!("Campaign {}", i),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::NONE { default_pacing: 1.0 }]
            );
        }
        
        // Create groups: [0, 1] and [2, 3]
        campaigns.create_value_group(vec![0, 1]);
        campaigns.create_value_group(vec![2, 3]);
        // Campaigns 4 and 5 are not in any group
        
        // Finalize groups
        campaigns.finalize_groups();
        
        // Check mappings:
        // Campaign 0 -> group 0
        // Campaign 1 -> group 0
        // Campaign 2 -> group 1
        // Campaign 3 -> group 1
        // Campaign 4 -> group 2 (new group, since num_groups = 2, starts at 2)
        // Campaign 5 -> group 3 (new group)
        assert_eq!(campaigns.campaign_to_value_group_mapping.len(), 6);
        assert_eq!(campaigns.campaign_to_value_group_mapping[0], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[1], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[2], 1);
        assert_eq!(campaigns.campaign_to_value_group_mapping[3], 1);
        assert_eq!(campaigns.campaign_to_value_group_mapping[4], 2);
        assert_eq!(campaigns.campaign_to_value_group_mapping[5], 3);
        
        // Verify that new groups were added to value_groups
        assert_eq!(campaigns.value_groups.len(), 4); // 2 existing + 2 new
        assert_eq!(campaigns.value_groups[0], vec![0, 1]);
        assert_eq!(campaigns.value_groups[1], vec![2, 3]);
        assert_eq!(campaigns.value_groups[2], vec![4]);
        assert_eq!(campaigns.value_groups[3], vec![5]);
    }

    #[test]
    fn test_finalize_groups_empty_campaigns() {
        let mut campaigns = Campaigns::new();
        
        // Don't add any campaigns
        
        // Finalize groups
        campaigns.finalize_groups();
        
        // Mapping should be empty
        assert_eq!(campaigns.campaign_to_value_group_mapping.len(), 0);
    }

    #[test]
    fn test_finalize_groups_single_campaign_in_group() {
        let mut campaigns = Campaigns::new();
        
        // Add 3 campaigns
        for i in 0..3 {
            campaigns.add(
                format!("Campaign {}", i),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::NONE { default_pacing: 1.0 }]
            );
        }
        
        // Create a group with just campaign 1
        campaigns.create_value_group(vec![1]);
        
        // Finalize groups
        campaigns.finalize_groups();
        
        // Check mappings:
        // Campaign 0 -> group 1 (new group, since num_groups = 1, starts at 1)
        // Campaign 1 -> group 0
        // Campaign 2 -> group 2 (new group)
        assert_eq!(campaigns.campaign_to_value_group_mapping.len(), 3);
        assert_eq!(campaigns.campaign_to_value_group_mapping[0], 1);
        assert_eq!(campaigns.campaign_to_value_group_mapping[1], 0);
        assert_eq!(campaigns.campaign_to_value_group_mapping[2], 2);
        
        // Verify that new groups were added to value_groups
        assert_eq!(campaigns.value_groups.len(), 3); // 1 existing + 2 new
        assert_eq!(campaigns.value_groups[0], vec![1]);
        assert_eq!(campaigns.value_groups[1], vec![0]);
        assert_eq!(campaigns.value_groups[2], vec![2]);
    }
}


