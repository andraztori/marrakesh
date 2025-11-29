// ControllerProportionalCore doesn't need any imports from controller_state

/// Proportional controller for adjusting campaign pacing based on target vs actual performance
/// Full PID was tried, but always something became unstable
pub struct ControllerProportionalCore {
    tolerance_fraction: f64,      // Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    max_adjustment_factor: f64,   // Maximum adjustment factor (e.g., 0.2 = 20%)
    proportional_gain: f64,       // Proportional gain (e.g., 0.2 = 20% of error)
}

impl ControllerProportionalCore {
    /// Create a new proportional controller with default parameters
    pub fn new() -> Self {
        Self {
            tolerance_fraction: 0.005,  // 0.5% tolerance
            max_adjustment_factor: 0.1,  // Max 20% adjustment
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
        
        // Calculate change in pacing
        let mut change_in_pacing = if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            previous_state * adjustment_factor
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            -previous_state * adjustment_factor
        } else {
            // Within tolerance - no change
            0.0
        };
        
        // Now this is a trick we do here given how cotrollers behave 
        if previous_state > 1.0 {
            change_in_pacing *= previous_state;
        } else {
            change_in_pacing /= previous_state;
        }
        
        // Calculate next state by adding change
        let next_state = previous_state + change_in_pacing;
        let changed = change_in_pacing != 0.0;
        
        // Return the calculated values
        (changed, next_state)
    }
}

/// Proportional-Derivative controller for adjusting campaign pacing based on target vs actual performance
/// Adds derivative term to reduce overshoot and improve stability
pub struct ControllerProportionalDerivativeCore {
    tolerance_fraction: f64,      // Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    max_adjustment_factor: f64,   // Maximum adjustment factor (e.g., 0.2 = 20%)
    proportional_gain: f64,       // Proportional gain (e.g., 0.1 = 10% of error)
    derivative_gain: f64,         // Derivative gain (e.g., 0.05 = 5% of error rate)
}

impl ControllerProportionalDerivativeCore {
    /// Create a new proportional-derivative controller with default parameters
    pub fn new() -> Self {
        Self {
            tolerance_fraction: 0.005,  // 0.5% tolerance
            max_adjustment_factor: 0.1,  // Max 20% adjustment
            proportional_gain: 0.1,      // 10% of error
            derivative_gain: 0.1,       // 5% of error rate
        }
    }

    /// Create a new proportional-derivative controller with custom parameters
    /// 
    /// # Arguments
    /// * `tolerance_fraction` - Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    /// * `max_adjustment_factor` - Maximum adjustment factor (e.g., 0.2 = 20%)
    /// * `proportional_gain` - Proportional gain (e.g., 0.1 = 10% of error)
    /// * `derivative_gain` - Derivative gain (e.g., 0.05 = 5% of error rate)
    pub fn new_advanced(tolerance_fraction: f64, max_adjustment_factor: f64, proportional_gain: f64, derivative_gain: f64) -> Self {
        Self {
            tolerance_fraction,
            max_adjustment_factor,
            proportional_gain,
            derivative_gain,
        }
    }

    /// Calculate pacing for next iteration based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `previous_state` - Previous controller state value (f64) - the pacing value
    /// * `previous_error` - Previous error value (Option<f64>) - None for first iteration
    /// 
    /// # Returns
    /// A tuple `(changed, next_state, next_error)` where:
    /// - `changed` is `true` if pacing was changed, `false` if it remained the same
    /// - `next_state` is the new controller state value (pacing)
    /// - `next_error` is the new error value to store for next iteration
    pub fn controller_next_state(&self, mut target: f64, mut actual: f64, mut previous_state: f64, previous_error: Option<f64>) -> (bool, f64, f64) {
        let mut invert:bool = false;

        let tolerance = target * self.tolerance_fraction;
        // target is never zero
        
        // Calculate current error (normalized)
        let current_error = if actual < target {
            (target - actual) / target  // Positive error when below target
        } else {
            (actual - target) / target  // Positive error when above target
        };
        
        // Calculate derivative term (rate of change of error)
        // The derivative term dampens the response when error is changing rapidly
        let derivative_term = if let Some(prev_error) = previous_error {
            // Derivative is the change in error
            // When error is decreasing (negative change), derivative_term is negative, reducing adjustment
            // When error is increasing (positive change), derivative_term is positive, increasing adjustment
            let error_change = current_error - prev_error;
            error_change * self.derivative_gain
        } else {
            // First iteration - no derivative term
            0.0
        };
        
        // Calculate change in pacing
        let mut change_in_pacing = if actual < target - tolerance {
            // Below target - increase pacing
            let proportional_term = current_error * self.proportional_gain;
            // Derivative term: negative when error decreasing (reduces adjustment), positive when error increasing (increases adjustment)
            let adjustment_factor = (proportional_term + derivative_term).min(self.max_adjustment_factor).max(0.0);
            previous_state * adjustment_factor
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let proportional_term = current_error * self.proportional_gain;
            // Derivative term: negative when error decreasing (reduces adjustment), positive when error increasing (increases adjustment)
            let adjustment_factor = (proportional_term + derivative_term).min(self.max_adjustment_factor).max(0.0);
            -previous_state * adjustment_factor
        } else {
            // Within tolerance - no change
            0.0
        };
        
        if previous_state > 1.0 {
            change_in_pacing *= previous_state;
        } else {
            change_in_pacing /= previous_state;
        }
        // Calculate next state by adding change
        let mut next_state = previous_state + change_in_pacing;
        let changed = change_in_pacing != 0.0;
        let next_error = current_error;
            
        // Return the calculated values
        (changed, next_state, next_error)
    }
}

