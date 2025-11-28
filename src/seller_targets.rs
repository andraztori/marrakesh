use crate::simulationrun::SellerStat;

/// Trait for seller convergence strategies
pub trait SellerTargetTrait {
    /// Get the actual and target values for convergence
    /// 
    /// # Arguments
    /// * `stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// A tuple `(actual, target)` representing the actual value achieved and the target value
    fn get_actual_and_target(&self, stat: &SellerStat) -> (f64, f64);
    
    /// Get the target value for convergence
    /// 
    /// # Returns
    /// The target value that the seller is trying to converge to
    fn get_target_value(&self) -> f64;
    
    /// Get a string representation of the convergence target
    fn converge_target_string(&self) -> String;
}

/// Convergence strategy for sellers that don't converge (no boost adjustment)
pub struct SellerTargetNone;

impl SellerTargetTrait for SellerTargetNone {
    fn get_actual_and_target(&self, _seller_stat: &crate::simulationrun::SellerStat) -> (f64, f64) {
        // No convergence, so no target or actual values
        (0.0, 0.0)
    }
    
    fn get_target_value(&self) -> f64 {
        0.0
    }
    
    fn converge_target_string(&self) -> String {
        "No convergence".to_string()
    }
}

/// Convergence strategy for sellers that converge boost to match target cost
pub struct SellerTargetTotalCost {
    pub target_cost: f64,
}

impl SellerTargetTrait for SellerTargetTotalCost {
    fn get_actual_and_target(&self, seller_stat: &crate::simulationrun::SellerStat) -> (f64, f64) {
        let actual = seller_stat.total_virtual_cost;
        let target = self.target_cost;
        (actual, target)
    }
    
    fn get_target_value(&self) -> f64 {
        self.target_cost
    }
    
    fn converge_target_string(&self) -> String {
        format!("Converge target cost: {:.2}", self.target_cost)
    }
}

