// Re-export ControllerStateTrait and types from controller_state module
pub use crate::controller_state::*;

// Re-export ControllerProportionalDerivativeCore from controller_core module
pub use crate::controller_core::ControllerProportionalDerivativeCore;

/// Trait for controlling convergence behavior in campaigns
pub trait ControllerTrait {
    /// Calculate the next controller state
    /// 
    /// # Arguments
    /// * `previous_state` - Previous controller state
    /// * `next_state` - Next controller state to update
    /// * `actual` - Actual value achieved
    /// * `target` - Target value to converge towards
    /// 
    /// # Returns
    /// `true` if the convergence value changed, `false` if it remained the same
    fn next_controller_state(&self, previous_state: &dyn ControllerStateTrait, next_state: &mut dyn ControllerStateTrait, actual: f64, target: f64) -> bool;
    
    /// Get the control variable (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Controller state to extract the pacing value from
    fn get_control_variable(&self, converge: &dyn ControllerStateTrait) -> f64;
    
    /// Create initial controller state
    fn create_controller_state(&self) -> Box<dyn ControllerStateTrait>;
    
    /// Get a string representation of the controller state
    /// 
    /// # Arguments
    /// * `converge` - Controller state to include pacing information
    fn controller_string(&self, converge: &dyn ControllerStateTrait) -> String;
}

/// Constant implementation of ControllerTrait
pub struct ControllerConstant {
    pub default_value: f64,
}

impl ControllerConstant {
    /// Create a new ControllerConstant with the given default value
    pub fn new(default_value: f64) -> Self {
        Self { default_value }
    }
}

impl ControllerTrait for ControllerConstant {
    fn next_controller_state(&self, _previous_state: &dyn ControllerStateTrait, _next_state: &mut dyn ControllerStateTrait, _actual: f64, _target: f64) -> bool {
        // Constant controller - no convergence, always return false
        false
    }
    
    fn get_control_variable(&self, _converge: &dyn ControllerStateTrait) -> f64 {
        // Return the controller's own default value, not from the state
        self.default_value
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerStateTrait> {
        Box::new(ControllerStateEmpty)
    }
    
    fn controller_string(&self, converge: &dyn ControllerStateTrait) -> String {
        format!("Constant: {:.4}", self.get_control_variable(converge))
    }
}

/// Proportional-Derivative controller implementation of ControllerTrait
/// 
/// Note: To get proportional-only behavior (no derivative term), use `new_advanced()` with `derivative_gain = 0.0`.
/// This makes the controller equivalent to a pure proportional controller.
pub struct ControllerProportionalDerivative {
    pub controller: ControllerProportionalDerivativeCore,
}

impl ControllerProportionalDerivative {
    /// Create a new ControllerProportionalDerivative
    /// 
    /// Note: To get proportional-only behavior, use `new_advanced()` with `derivative_gain = 0.0`
    pub fn new() -> Self {
        Self {
            controller: ControllerProportionalDerivativeCore::new(),
        }
    }

    /// Create a new ControllerProportionalDerivative with custom parameters
    /// 
    /// # Arguments
    /// * `tolerance_fraction` - Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    /// * `max_adjustment_factor` - Maximum adjustment factor (e.g., 0.2 = 20%)
    /// * `proportional_gain` - Proportional gain (e.g., 0.1 = 10% of error)
    /// * `derivative_gain` - Derivative gain (e.g., 0.05 = 5% of error rate). Set to 0.0 for proportional-only behavior.
    /// * `rescaling` - Whether to apply rescaling (reversal of proportions) based on previous_state. Default is true.
    pub fn new_advanced(tolerance_fraction: f64, max_adjustment_factor: f64, proportional_gain: f64, derivative_gain: f64, rescaling: bool) -> Self {
        Self {
            controller: ControllerProportionalDerivativeCore::new_advanced(tolerance_fraction, max_adjustment_factor, proportional_gain, derivative_gain, rescaling),
        }
    }
}

impl ControllerTrait for ControllerProportionalDerivative {
    fn next_controller_state(&self, previous_state: &dyn ControllerStateTrait, next_state: &mut dyn ControllerStateTrait, actual: f64, target: f64) -> bool {
        // Extract previous state values (variable1 = pacing, variable2 = previous_error)
        let previous_state_double = previous_state.as_any().downcast_ref::<ControllerStateDoubleVariable>().unwrap();
        let previous_state_value = previous_state_double.variable1;
        let previous_error = previous_state_double.variable2;
        
        // Calculate next state using the controller
        let (changed, next_state_value, next_error) = self.controller.controller_next_state(target, actual, previous_state_value, previous_error);
        
        // Save the next state values
        let next_state_mut = next_state.as_any_mut().downcast_mut::<ControllerStateDoubleVariable>().unwrap();
        next_state_mut.variable1 = next_state_value;
        next_state_mut.variable2 = Some(next_error);
        
        changed
    }
    
    fn get_control_variable(&self, converge: &dyn ControllerStateTrait) -> f64 {
        converge.as_any().downcast_ref::<ControllerStateDoubleVariable>().unwrap().variable1
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerStateTrait> {
        Box::new(ControllerStateDoubleVariable { 
            variable1: 1.0,  // Initial pacing value
            variable2: None,  // No previous error on first iteration
        })
    }
    
    fn controller_string(&self, converge: &dyn ControllerStateTrait) -> String {
        let state = converge.as_any().downcast_ref::<ControllerStateDoubleVariable>().unwrap();
        match state.variable2 {
            None => format!("PD cntrl: {:.4} (no prev err)", state.variable1),
            Some(prev_err) => format!("PD cntrl: {:.4} (prev_err: {:.4})", state.variable1, prev_err),
        }
    }
}


