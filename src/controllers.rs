// Re-export ControllerState types from controller_state module
pub use crate::controller_state::*;

// Re-export ControllerProportionalCore from controller_core module
pub use crate::controller_core::ControllerProportionalCore;
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
    fn next_controller_state(&self, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState, actual: f64, target: f64) -> bool;
    
    /// Get the control variable (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Controller state to extract the pacing value from
    fn get_control_variable(&self, converge: &dyn ControllerState) -> f64;
    
    /// Create initial controller state
    fn create_controller_state(&self) -> Box<dyn ControllerState>;
    
    /// Get a string representation of the controller state
    /// 
    /// # Arguments
    /// * `converge` - Controller state to include pacing information
    fn controller_string(&self, converge: &dyn ControllerState) -> String;
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
    fn next_controller_state(&self, _previous_state: &dyn ControllerState, _next_state: &mut dyn ControllerState, _actual: f64, _target: f64) -> bool {
        // Constant controller - no convergence, always return false
        false
    }
    
    fn get_control_variable(&self, _converge: &dyn ControllerState) -> f64 {
        // Return the controller's own default value, not from the state
        self.default_value
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        Box::new(ControllerStateEmpty)
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        format!("Constant: {:.4}", self.get_control_variable(converge))
    }
}

/// Proportional controller implementation of ControllerTrait
pub struct ControllerProportional {
    pub controller: ControllerProportionalCore,
}

impl ControllerProportional {
    /// Create a new ControllerProportional
    pub fn new() -> Self {
        Self {
            controller: ControllerProportionalCore::new(),
        }
    }

    /// Create a new ControllerProportional with custom parameters
    /// 
    /// # Arguments
    /// * `tolerance_fraction` - Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    /// * `max_adjustment_factor` - Maximum adjustment factor (e.g., 0.2 = 20%)
    /// * `proportional_gain` - Proportional gain (e.g., 0.1 = 10% of error)
    pub fn new_advanced(tolerance_fraction: f64, max_adjustment_factor: f64, proportional_gain: f64) -> Self {
        Self {
            controller: ControllerProportionalCore::new_advanced(tolerance_fraction, max_adjustment_factor, proportional_gain),
        }
    }
}

impl ControllerTrait for ControllerProportional {
    fn next_controller_state(&self, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState, actual: f64, target: f64) -> bool {
        // Extract previous state value
        let previous_state_value = previous_state.as_any().downcast_ref::<ControllerStateSingleVariable>().unwrap().converging_variable;
        
        // Calculate next state using the controller
        let (changed, next_state_value) = self.controller.controller_next_state(target, actual, previous_state_value);
        
        // Save the next state value
        let next_state_mut = next_state.as_any_mut().downcast_mut::<ControllerStateSingleVariable>().unwrap();
        next_state_mut.converging_variable = next_state_value;
        
        changed
    }
    
    fn get_control_variable(&self, converge: &dyn ControllerState) -> f64 {
        converge.as_any().downcast_ref::<ControllerStateSingleVariable>().unwrap().converging_variable
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        Box::new(ControllerStateSingleVariable { converging_variable: 1.0 })
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        format!("P cntrl: {:.4}", self.get_control_variable(converge))
    }
}

/// Proportional-Derivative controller implementation of ControllerTrait
pub struct ControllerProportionalDerivative {
    pub controller: ControllerProportionalDerivativeCore,
}

impl ControllerProportionalDerivative {
    /// Create a new ControllerProportionalDerivative
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
    /// * `derivative_gain` - Derivative gain (e.g., 0.05 = 5% of error rate)
    pub fn new_advanced(tolerance_fraction: f64, max_adjustment_factor: f64, proportional_gain: f64, derivative_gain: f64) -> Self {
        Self {
            controller: ControllerProportionalDerivativeCore::new_advanced(tolerance_fraction, max_adjustment_factor, proportional_gain, derivative_gain),
        }
    }
}

impl ControllerTrait for ControllerProportionalDerivative {
    fn next_controller_state(&self, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState, actual: f64, target: f64) -> bool {
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
    
    fn get_control_variable(&self, converge: &dyn ControllerState) -> f64 {
        converge.as_any().downcast_ref::<ControllerStateDoubleVariable>().unwrap().variable1
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        Box::new(ControllerStateDoubleVariable { 
            variable1: 1.0,  // Initial pacing value
            variable2: None,  // No previous error on first iteration
        })
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        let state = converge.as_any().downcast_ref::<ControllerStateDoubleVariable>().unwrap();
        match state.variable2 {
            None => format!("PD cntrl: {:.4} (no prev err)", state.variable1),
            Some(prev_err) => format!("PD cntrl: {:.4} (prev_err: {:.4})", state.variable1, prev_err),
        }
    }
}


