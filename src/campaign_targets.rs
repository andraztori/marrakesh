use crate::simulationrun::CampaignStat;

/// Trait for campaign convergence strategies
pub trait CampaignTargetTrait {
    /// Get the actual and target values for convergence
    /// 
    /// # Arguments
    /// * `stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// A tuple `(actual, target)` representing the actual value achieved and the target value
    fn get_actual_and_target(&self, stat: &CampaignStat) -> (f64, f64);
    
    /// Get the target value for convergence
    /// 
    /// # Returns
    /// The target value that the campaign is trying to converge to
    fn get_target_value(&self) -> f64;
    
    /// Get a string representation of the convergence target
    fn converge_target_string(&self) -> String;
}

/// Convergence strategy for total impressions target
pub struct CampaignTargetTotalImpressions {
    pub total_impressions_target: i32,
}

impl CampaignTargetTrait for CampaignTargetTotalImpressions {
    fn get_actual_and_target(&self, campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        (campaign_stat.impressions_obtained, self.total_impressions_target as f64)
    }
    
    fn get_target_value(&self) -> f64 {
        self.total_impressions_target as f64
    }
    
    fn converge_target_string(&self) -> String {
        format!("Fixed impressions ({})", self.total_impressions_target)
    }
}

/// Convergence strategy for total budget target
pub struct CampaignTargetTotalBudget {
    pub total_budget_target: f64,
}

impl CampaignTargetTrait for CampaignTargetTotalBudget {
    fn get_actual_and_target(&self, campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        (campaign_stat.total_buyer_charge, self.total_budget_target)
    }
    
    fn get_target_value(&self) -> f64 {
        self.total_budget_target
    }
    
    fn converge_target_string(&self) -> String {
        format!("Fixed budget target: {:.2}", self.total_budget_target)
    }
}

/// Convergence strategy for average value target
/// For example, may be we want viewability to be 80% ...
pub struct CampaignTargetAvgValue {
    pub avg_impression_value_to_campaign: f64,
}

impl CampaignTargetTrait for CampaignTargetAvgValue {
    fn get_actual_and_target(&self, campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        // Calculate average value as total_value / impressions_obtained
        // If no impressions were obtained, return 0.0 as actual
        let actual = if campaign_stat.impressions_obtained > 0.0 {
            campaign_stat.total_value / campaign_stat.impressions_obtained
        } else {
            0.0
        };
        (actual, self.avg_impression_value_to_campaign)
    }
    
    fn get_target_value(&self) -> f64 {
        self.avg_impression_value_to_campaign
    }
    
    fn converge_target_string(&self) -> String {
        format!("Average value target: {:.4}", self.avg_impression_value_to_campaign)
    }
}

/// Convergence strategy for no convergence (fixed pacing)
pub struct CampaignTargetNone;

impl CampaignTargetTrait for CampaignTargetNone {
    fn get_actual_and_target(&self, _campaign_stat: &crate::simulationrun::CampaignStat) -> (f64, f64) {
        // No convergence, so no target or actual values
        (0.0, 0.0)
    }
    
    fn get_target_value(&self) -> f64 {
        0.0
    }
    
    fn converge_target_string(&self) -> String {
        "No convergence target".to_string()
    }
}

