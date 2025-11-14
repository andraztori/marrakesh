use crate::impressions::Impression;
use crate::utils::ControllerProportional;
pub use crate::converge::ConvergingParam;

/// Maximum number of campaigns supported (determines size of value_to_campaign_id array)
pub const MAX_CAMPAIGNS: usize = 10;

/// Campaign type determining the constraint model
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    FIXED_IMPRESSIONS_MULTIPLICATIVE_PACING { total_impressions_target: i32 },
    FIXED_BUDGET_MULTIPLICATIVE_PACING { total_budget_target: f64 },
    FIXED_BUDGET_OPTIMAL_BIDDING { total_budget_target: f64 },
}


/// Architecture explanation:
/// CampaignTrait is used to describe a campaign
/// Each campaign can use a convergence mechanism that needs to store data locally (for example pacing parameter)
/// These can be anything a trait implementation wants it to be, so they need to be dnyamically created by create_converge
/// Simulation then creates such converge parameters for each campaign and uses them to be able to call converge_iteration() 

/// Trait for campaign convergence strategies
pub trait CampaignConverge: std::any::Any {
    /// Perform one iteration of convergence, updating the next convergence parameter
    /// 
    /// # Arguments
    /// * `current_converge` - Current convergence parameter
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool;
    
    /// Get the converging parameter (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to extract the pacing value from
    fn get_parameter(&self, converge: &dyn crate::converge::Converge) -> f64;
    
    /// Get a string representation of the convergence target and pacing
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to include pacing information
    fn converge_target_string(&self, converge: &dyn crate::converge::Converge) -> String;
    
    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Convergence strategy for total impressions target
pub struct ConvergeTotalImpressions {
    pub total_impressions_target: i32,
    pub pacing_converger: ControllerProportional,
}

impl CampaignConverge for ConvergeTotalImpressions {
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        // Downcast to concrete types at the beginning
        let current_converge = current_converge.as_any().downcast_ref::<ConvergingParam>().unwrap();
        let next_converge = next_converge.as_any_mut().downcast_mut::<ConvergingParam>().unwrap();
        
        let target = self.total_impressions_target as f64;
        let actual = campaign_stat.impressions_obtained as f64;
        let current_pacing = current_converge.converging_param;
        
        let (new_pacing, changed) = self.pacing_converger.pacing_in_next_iteration(target, actual, current_pacing);
        next_converge.converging_param = new_pacing;
        
        changed
    }
    
    fn get_parameter(&self, converge: &dyn crate::converge::Converge) -> f64 {
        converge.as_any().downcast_ref::<ConvergingParam>().unwrap().converging_param
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::Converge) -> String {
        let pacing = converge.as_any().downcast_ref::<ConvergingParam>().unwrap().converging_param;
        format!("Fixed impressions ({}), pacing: {:.2}", self.total_impressions_target, pacing)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Convergence strategy for total budget target
pub struct ConvergeTotalBudget {
    pub total_budget_target: f64,
    pub pacing_converger: ControllerProportional,
}

impl CampaignConverge for ConvergeTotalBudget {
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        // Downcast to concrete types at the beginning
        let current_converge = current_converge.as_any().downcast_ref::<ConvergingParam>().unwrap();
        let next_converge = next_converge.as_any_mut().downcast_mut::<ConvergingParam>().unwrap();
        
        let target = self.total_budget_target;
        let actual = campaign_stat.total_buyer_charge;
        let current_pacing = current_converge.converging_param;
        
        let (new_pacing, changed) = self.pacing_converger.pacing_in_next_iteration(target, actual, current_pacing);
        next_converge.converging_param = new_pacing;
        
        changed
    }
    
    fn get_parameter(&self, converge: &dyn crate::converge::Converge) -> f64 {
        converge.as_any().downcast_ref::<ConvergingParam>().unwrap().converging_param
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::Converge) -> String {
        let pacing = converge.as_any().downcast_ref::<ConvergingParam>().unwrap().converging_param;
        format!("Fixed budget ({:.2}), pacing: {:.2}", self.total_budget_target, pacing)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
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
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::Converge, seller_boost_factor: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    

    
    
    /// Create a new convergence parameter for this campaign type
    fn create_converge(&self) -> Box<dyn crate::converge::Converge>;

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
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool;

    /// Get a string representation of the campaign type and convergence strategy
    /// 
    /// # Arguments
    /// * `converge` - Convergence parameter to include pacing information
    fn type_and_converge_string(&self, converge: &dyn crate::converge::Converge) -> String;

}

/// Campaign with multiplicative pacing
pub struct CampaignMultiplicativePacing {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub pacing_converger: ControllerProportional,
    pub converge_strategy: Box<dyn CampaignConverge>,
}

impl CampaignTrait for CampaignMultiplicativePacing {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn crate::converge::Converge, seller_boost_factor: f64, _logger: &mut crate::logger::Logger) -> Option<f64> {
        let converge = converge.as_any().downcast_ref::<ConvergingParam>().unwrap();
        Some(converge.converging_param * impression.value_to_campaign_id[self.campaign_id] * seller_boost_factor)
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::Converge, next_converge: &mut dyn crate::converge::Converge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        self.converge_strategy.converge_iteration(current_converge, next_converge, campaign_stat)
    }
    
    fn type_and_converge_string(&self, converge: &dyn crate::converge::Converge) -> String {
        format!("Multiplicative pacing ({})", self.converge_strategy.converge_target_string(converge))
    }
    
    fn create_converge(&self) -> Box<dyn crate::converge::Converge> {
        Box::new(ConvergingParam { converging_param: 1.0 })
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
    /// * `campaign_type` - Type of campaign
    pub fn add(&mut self, campaign_name: String, campaign_type: CampaignType) {
        if self.campaigns.len() >= MAX_CAMPAIGNS {
            panic!(
                "Cannot add campaign: maximum number of campaigns ({}) exceeded. Current count: {}",
                MAX_CAMPAIGNS,
                self.campaigns.len()
            );
        }
        let campaign_id = self.campaigns.len();
        match campaign_type {
            CampaignType::FIXED_IMPRESSIONS_MULTIPLICATIVE_PACING { total_impressions_target } => {
                self.campaigns.push(Box::new(CampaignMultiplicativePacing {
                    campaign_id,
                    campaign_name,
                    pacing_converger: ControllerProportional::new(),
                    converge_strategy: Box::new(ConvergeTotalImpressions {
                        total_impressions_target,
                        pacing_converger: ControllerProportional::new(),
                    }),
                }));
            }
            CampaignType::FIXED_BUDGET_MULTIPLICATIVE_PACING { total_budget_target } => {
                self.campaigns.push(Box::new(CampaignMultiplicativePacing {
                    campaign_id,
                    campaign_name,
                    pacing_converger: ControllerProportional::new(),
                    converge_strategy: Box::new(ConvergeTotalBudget {
                        total_budget_target,
                        pacing_converger: ControllerProportional::new(),
                    }),
                }));
            }
            CampaignType::FIXED_BUDGET_OPTIMAL_BIDDING { total_budget_target } => {
                self.campaigns.push(Box::new(crate::campaigns_optimal_bidding::CampaignFixedBudgetOptimalBidding {
                    campaign_id,
                    campaign_name,
                    converge_strategy: Box::new(ConvergeTotalBudget {
                        total_budget_target,
                        pacing_converger: ControllerProportional::new(),
                    }),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bid() {
        // Create a campaign with campaign_id = 2
        let campaign = CampaignMultiplicativePacing {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            pacing_converger: ControllerProportional::new(),
            converge_strategy: Box::new(ConvergeTotalImpressions {
                total_impressions_target: 1000,
                pacing_converger: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 0.5
        let campaign_converge: Box<dyn crate::converge::Converge> = Box::new(ConvergingParam {
            converging_param: 0.5,
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
            pacing_converger: ControllerProportional::new(),
            converge_strategy: Box::new(ConvergeTotalBudget {
                total_budget_target: 5000.0,
                pacing_converger: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 1.0
        let campaign_converge: Box<dyn crate::converge::Converge> = Box::new(ConvergingParam {
            converging_param: 1.0,
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
            pacing_converger: ControllerProportional::new(),
            converge_strategy: Box::new(ConvergeTotalImpressions {
                total_impressions_target: 1000,
                pacing_converger: ControllerProportional::new(),
            }),
        };

        // Create a campaign converge with pacing = 0.0
        let campaign_converge: Box<dyn crate::converge::Converge> = Box::new(ConvergingParam {
            converging_param: 0.0,
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
        };

        // Expected bid = 0.0 * 100.0 * 1.0 = 0.0
        let mut logger = crate::logger::Logger::new();
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0, &mut logger);
        assert_eq!(bid, Some(0.0));
    }
}

