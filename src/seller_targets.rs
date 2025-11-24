use crate::converge::ConvergeTargetAny;

/// Convergence strategy for sellers that don't converge (no boost adjustment)
pub struct ConvergeNone;

impl ConvergeTargetAny<crate::simulationrun::SellerStat> for ConvergeNone {
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
pub struct ConvergeTargetTotalCost {
    pub target_cost: f64,
}

impl ConvergeTargetAny<crate::simulationrun::SellerStat> for ConvergeTargetTotalCost {
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

