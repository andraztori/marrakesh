// ControllerProportional doesn't need any imports from controller_state

/// Proportional controller for adjusting campaign pacing based on target vs actual performance
/// Full PID was tried, but always something became unstable
pub struct ControllerProportional {
    tolerance_fraction: f64,      // Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    max_adjustment_factor: f64,   // Maximum adjustment factor (e.g., 0.2 = 20%)
    proportional_gain: f64,       // Proportional gain (e.g., 0.2 = 20% of error)
}

impl ControllerProportional {
    /// Create a new proportional controller with default parameters
    pub fn new() -> Self {
        Self {
            tolerance_fraction: 0.005,  // 0.5% tolerance
            max_adjustment_factor: 0.2,  // Max 20% adjustment
            proportional_gain: 0.1,      // 20% of error
    }
        }

    /// Create a new proportional controller with custom parameters
    /// 
    /// # Arguments
    /// * `tolerance_fraction` - Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    /// * `max_adjustment_factor` - Maximum adjustment factor (e.g., 0.2 = 20%)
    /// * `proportional_gain` - Proportional gain (e.g., 0.1 = 10% of error)
    pub fn new_advanced(tolerance_fraction: f64, max_adjustment_factor: f64, proportional_gain: f64) -> Self {
        Self {
            tolerance_fraction,
            max_adjustment_factor,
            proportional_gain,
    }
    }

    /// Calculate pacing for next iteration based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `previous_state` - Previous controller state value (f64)
    /// 
    /// # Returns
    /// A tuple `(changed, next_state)` where:
    /// - `changed` is `true` if pacing was changed, `false` if it remained the same
    /// - `next_state` is the new controller state value
    pub fn controller_next_state(&self, target: f64, actual: f64, previous_state: f64) -> (bool, f64) {
        let tolerance = target * self.tolerance_fraction;
        // target is never zero
        if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = previous_state * (1.0 + adjustment_factor);
            (true, new_pacing)
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = previous_state * (1.0 - adjustment_factor);
            (true, new_pacing)
        } else {
            // Within tolerance - keep constant
            (false, previous_state)
        }
    }
}

