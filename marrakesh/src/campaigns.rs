use crate::impressions::Impression;

/// Maximum number of campaigns supported (determines size of value_to_campaign_id array)
pub const MAX_CAMPAIGNS: usize = 10;

/// Campaign type determining the constraint model
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    FIXED_IMPRESSIONS { total_impressions_target: i32 },
    FIXED_BUDGET { total_budget_target: f64 },
}

/// Trait for campaign convergence parameters
/// Each campaign type has its own associated convergence parameter type
pub trait CampaignConverge {
    /// Get the current pacing value
    fn pacing(&self) -> f64;
    
    /// Set the pacing value
    fn set_pacing(&mut self, pacing: f64);
    
    /// Clone the convergence parameter
    fn clone_box(&self) -> Box<dyn CampaignConverge>;
}

/// Trait for campaigns participating in auctions
pub trait CampaignTrait {
    /// Get the campaign ID
    fn campaign_id(&self) -> usize;
    
    /// Get the campaign name
    fn campaign_name(&self) -> &str;
    
    /// Calculate the bid for this campaign given an impression and convergence parameter
    /// Bid = converge_param.pacing() * impression.value_to_campaign_id[campaign_id]
    fn get_bid(&self, impression: &Impression, converge_param: &dyn CampaignConverge) -> f64;
    
    /// Perform one iteration of convergence, updating the next convergence parameter
    /// This method encapsulates the convergence logic for each campaign type
    /// 
    /// # Arguments
    /// * `current_converge` - Current convergence parameter (immutable)
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// * `controller` - Proportional controller for adjusting pacing
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat, controller: &crate::utils::ControllerProportional) -> bool;
    
    /// Get a string representation of the campaign type and target
    fn type_and_target_string(&self) -> String;
    
    /// Create a new convergence parameter for this campaign type with default pacing
    fn create_converge_param(&self, pacing: f64) -> Box<dyn CampaignConverge>;
}

/// Convergence parameter for fixed impressions campaigns
#[derive(Clone)]
pub struct CampaignFixedImpressionsParam {
    pub pacing: f64,
}

impl CampaignConverge for CampaignFixedImpressionsParam {
    fn pacing(&self) -> f64 {
        self.pacing
    }
    
    fn set_pacing(&mut self, pacing: f64) {
        self.pacing = pacing;
    }
    
    fn clone_box(&self) -> Box<dyn CampaignConverge> {
        Box::new(self.clone())
    }
}

/// Convergence parameter for fixed budget campaigns
#[derive(Clone)]
pub struct CampaignFixedBudgetParam {
    pub pacing: f64,
}

impl CampaignConverge for CampaignFixedBudgetParam {
    fn pacing(&self) -> f64 {
        self.pacing
    }
    
    fn set_pacing(&mut self, pacing: f64) {
        self.pacing = pacing;
    }
    
    fn clone_box(&self) -> Box<dyn CampaignConverge> {
        Box::new(self.clone())
    }
}

/// Campaign with fixed impressions target
pub struct CampaignFixedImpressions {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub total_impressions_target: i32,
}

impl CampaignTrait for CampaignFixedImpressions {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge_param: &dyn CampaignConverge) -> f64 {
        converge_param.pacing() * impression.value_to_campaign_id[self.campaign_id]
    }
    
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat, controller: &crate::utils::ControllerProportional) -> bool {
        let target = self.total_impressions_target as f64;
        let actual = campaign_stat.impressions_obtained as f64;
        let current_pacing = current_converge.pacing();
        let (new_pacing, changed) = controller.adjust_pacing(target, actual, current_pacing);
        next_converge.set_pacing(new_pacing);
        changed
    }
    
    fn type_and_target_string(&self) -> String {
        format!("FIXED_IMPRESSIONS (target: {})", self.total_impressions_target)
    }
    
    fn create_converge_param(&self, pacing: f64) -> Box<dyn CampaignConverge> {
        Box::new(CampaignFixedImpressionsParam { pacing })
    }
}

/// Campaign with fixed budget target
pub struct CampaignFixedBudget {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub total_budget_target: f64,
}

impl CampaignTrait for CampaignFixedBudget {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, converge_param: &dyn CampaignConverge) -> f64 {
        converge_param.pacing() * impression.value_to_campaign_id[self.campaign_id]
    }
    
    fn converge_iteration(&self, current_converge: &dyn CampaignConverge, next_converge: &mut dyn CampaignConverge, campaign_stat: &crate::simulationrun::CampaignStat, controller: &crate::utils::ControllerProportional) -> bool {
        let target = self.total_budget_target;
        let actual = campaign_stat.total_buyer_charge;
        let current_pacing = current_converge.pacing();
        let (new_pacing, changed) = controller.adjust_pacing(target, actual, current_pacing);
        next_converge.set_pacing(new_pacing);
        changed
    }
    
    fn type_and_target_string(&self) -> String {
        format!("FIXED_BUDGET (target: {:.2})", self.total_budget_target)
    }
    
    fn create_converge_param(&self, pacing: f64) -> Box<dyn CampaignConverge> {
        Box::new(CampaignFixedBudgetParam { pacing })
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
    /// * `campaign_type` - Type of campaign (FIXED_IMPRESSIONS or FIXED_BUDGET)
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
            CampaignType::FIXED_IMPRESSIONS { total_impressions_target } => {
                self.campaigns.push(Box::new(CampaignFixedImpressions {
                    campaign_id,
                    campaign_name,
                    total_impressions_target,
                }));
            }
            CampaignType::FIXED_BUDGET { total_budget_target } => {
                self.campaigns.push(Box::new(CampaignFixedBudget {
                    campaign_id,
                    campaign_name,
                    total_budget_target,
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChargeType;

    #[test]
    fn test_get_bid() {
        // Create a campaign with campaign_id = 2
        let campaign = CampaignFixedImpressions {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            total_impressions_target: 1000,
        };

        // Create a campaign parameter with pacing = 0.5
        let campaign_param: Box<dyn CampaignConverge> = Box::new(CampaignFixedImpressionsParam {
            pacing: 0.5,
        });

        // Create an impression with value_to_campaign_id[2] = 20.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[2] = 20.0;

        let impression = Impression {
            seller_id: 0,
            charge_type: ChargeType::FIRST_PRICE,
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            value_to_campaign_id,
        };

        // Expected bid = 0.5 * 20.0 = 10.0
        let bid = campaign.get_bid(&impression, campaign_param.as_ref());
        assert_eq!(bid, 10.0);
    }

    #[test]
    fn test_get_bid_with_different_campaign_id() {
        // Create a campaign with campaign_id = 0
        let campaign = CampaignFixedBudget {
            campaign_id: 0,
            campaign_name: "Test Campaign".to_string(),
            total_budget_target: 5000.0,
        };

        // Create a campaign parameter with pacing = 1.0
        let campaign_param: Box<dyn CampaignConverge> = Box::new(CampaignFixedBudgetParam {
            pacing: 1.0,
        });

        // Create an impression with value_to_campaign_id[0] = 15.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[0] = 15.0;

        let impression = Impression {
            seller_id: 1,
            charge_type: ChargeType::FIXED_COST {
                fixed_cost_cpm: 10.0,
            },
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            value_to_campaign_id,
        };

        // Expected bid = 1.0 * 15.0 = 15.0
        let bid = campaign.get_bid(&impression, campaign_param.as_ref());
        assert_eq!(bid, 15.0);
    }

    #[test]
    fn test_get_bid_with_zero_pacing() {
        // Create a campaign with campaign_id = 1
        let campaign = CampaignFixedImpressions {
            campaign_id: 1,
            campaign_name: "Test Campaign".to_string(),
            total_impressions_target: 1000,
        };

        // Create a campaign parameter with pacing = 0.0
        let campaign_param: Box<dyn CampaignConverge> = Box::new(CampaignFixedImpressionsParam {
            pacing: 0.0,
        });

        // Create an impression with value_to_campaign_id[1] = 100.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[1] = 100.0;

        let impression = Impression {
            seller_id: 0,
            charge_type: ChargeType::FIRST_PRICE,
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            value_to_campaign_id,
        };

        // Expected bid = 0.0 * 100.0 = 0.0
        let bid = campaign.get_bid(&impression, campaign_param.as_ref());
        assert_eq!(bid, 0.0);
    }
}

