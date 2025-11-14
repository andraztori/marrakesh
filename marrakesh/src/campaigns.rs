use crate::impressions::Impression;
use crate::utils::ControllerProportional;

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


/// Trait for campaign convergence parameters
/// Each campaign type has its own associated convergence parameter type
pub trait CampaignConverge: std::any::Any {
    /// Clone the convergence parameter
    fn clone_box(&self) -> Box<dyn CampaignConverge>;
    
    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get a mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Trait for campaigns participating in auctions
pub trait CampaignTrait {
    /// Get the campaign ID
    fn campaign_id(&self) -> usize;
    
    /// Get the campaign name
    fn campaign_name(&self) -> &str;
    
    /// Calculate the bid for this campaign given an impression, convergence parameter, and seller boost factor
    /// Bid = converge.pacing() * impression.value_to_campaign_id[campaign_id] * seller_boost_factor
    fn get_bid(&self, impression: &Impression, converge: &dyn CampaignConverge, seller_boost_factor: f64) -> f64;
    

    
    
    /// Create a new convergence parameter for this campaign type
    fn create_converge(&self) -> Box<dyn CampaignConverge>;

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
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool;

    /// Get a string representation of the campaign type and target
    fn type_and_target_string(&self) -> String;
    
    /// Get a formatted string representation of the convergence parameters
    fn converge_string(&self, converge: &dyn CampaignConverge) -> String;

}

/// Convergence parameter for campaign pacing
#[derive(Clone)]
pub struct CampaignConvergePacing {
    pub pacing: f64,
}

impl CampaignConverge for CampaignConvergePacing {
    fn clone_box(&self) -> Box<dyn CampaignConverge> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Campaign with fixed impressions target
pub struct CampaignFixedImpressionsMultiplicativePacing {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub total_impressions_target: i32,
    pub pacing_converger: ControllerProportional,
}

impl CampaignTrait for CampaignFixedImpressionsMultiplicativePacing {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn CampaignConverge, seller_boost_factor: f64) -> f64 {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        converge.pacing * impression.value_to_campaign_id[self.campaign_id] * seller_boost_factor
    }
    
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        // Downcast to concrete types at the beginning
        let current_converge = current_converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        let next_converge = next_converge.as_any_mut().downcast_mut::<CampaignConvergePacing>().unwrap();
        
        let target = self.total_impressions_target as f64;
        let actual = campaign_stat.impressions_obtained as f64;
        let current_pacing = current_converge.pacing;
        
        let (new_pacing, changed) = self.pacing_converger.pacing_in_next_iteration(target, actual, current_pacing);
        next_converge.pacing = new_pacing;
        
        changed
    }
    
    fn type_and_target_string(&self) -> String {
        format!("FIXED_IMPRESSIONS_MULTIPLICATIVE_PACING (target: {})", self.total_impressions_target)
    }
    
    fn converge_string(&self, converge: &dyn CampaignConverge) -> String {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        format!("Pacing: {:.2}", converge.pacing)
    }
    
    fn create_converge(&self) -> Box<dyn CampaignConverge> {
        Box::new(CampaignConvergePacing { pacing: 1.0 })
    }
}

/// Campaign with fixed budget target
pub struct CampaignFixedBudgetMutiplicativePacing {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub total_budget_target: f64,
    pub pacing_converger: ControllerProportional,
}

impl CampaignTrait for CampaignFixedBudgetMutiplicativePacing {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge: &dyn CampaignConverge, seller_boost_factor: f64) -> f64 {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        converge.pacing * impression.value_to_campaign_id[self.campaign_id] * seller_boost_factor
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
        format!("FIXED_BUDGET_MULTIPLICATIVE_PACING (target: {:.2})", self.total_budget_target)
    }
    
    fn converge_string(&self, converge: &dyn CampaignConverge) -> String {
        let converge = converge.as_any().downcast_ref::<CampaignConvergePacing>().unwrap();
        format!("Pacing: {:.2}", converge.pacing)
    }
    
    fn create_converge(&self) -> Box<dyn CampaignConverge> {
        Box::new(CampaignConvergePacing { pacing: 1.0 })
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
                self.campaigns.push(Box::new(CampaignFixedImpressionsMultiplicativePacing {
                    campaign_id,
                    campaign_name,
                    total_impressions_target,
                    pacing_converger: ControllerProportional::new(),
                }));
            }
            CampaignType::FIXED_BUDGET_MULTIPLICATIVE_PACING { total_budget_target } => {
                self.campaigns.push(Box::new(CampaignFixedBudgetMutiplicativePacing {
                    campaign_id,
                    campaign_name,
                    total_budget_target,
                    pacing_converger: ControllerProportional::new(),
                }));
            }
            CampaignType::FIXED_BUDGET_OPTIMAL_BIDDING { total_budget_target } => {
                self.campaigns.push(Box::new(crate::campaigns_optimal_bidding::CampaignFixedBudgetOptimalBidding {
                    campaign_id,
                    campaign_name,
                    total_budget_target,
                    pacing_converger: ControllerProportional::new(),
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
        let campaign = CampaignFixedImpressionsMultiplicativePacing {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            total_impressions_target: 1000,
            pacing_converger: ControllerProportional::new(),
        };

        // Create a campaign converge with pacing = 0.5
        let campaign_converge: Box<dyn CampaignConverge> = Box::new(CampaignConvergePacing {
            pacing: 0.5,
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
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0);
        assert_eq!(bid, 10.0);
    }

    #[test]
    fn test_get_bid_with_different_campaign_id() {
        // Create a campaign with campaign_id = 0
        let campaign = CampaignFixedBudgetMutiplicativePacing {
            campaign_id: 0,
            campaign_name: "Test Campaign".to_string(),
            total_budget_target: 5000.0,
            pacing_converger: ControllerProportional::new(),
        };

        // Create a campaign converge with pacing = 1.0
        let campaign_converge: Box<dyn CampaignConverge> = Box::new(CampaignConvergePacing {
            pacing: 1.0,
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
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0);
        assert_eq!(bid, 15.0);
    }

    #[test]
    fn test_get_bid_with_zero_pacing() {
        // Create a campaign with campaign_id = 1
        let campaign = CampaignFixedImpressionsMultiplicativePacing {
            campaign_id: 1,
            campaign_name: "Test Campaign".to_string(),
            total_impressions_target: 1000,
            pacing_converger: ControllerProportional::new(),
        };

        // Create a campaign converge with pacing = 0.0
        let campaign_converge: Box<dyn CampaignConverge> = Box::new(CampaignConvergePacing {
            pacing: 0.0,
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
        let bid = campaign.get_bid(&impression, campaign_converge.as_ref(), 1.0);
        assert_eq!(bid, 0.0);
    }
}

