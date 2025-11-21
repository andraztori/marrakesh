/// Unified trait for controller state
/// Used for both campaigns and sellers
pub trait ControllerState: std::any::Any {
    fn clone_box(&self) -> Box<dyn ControllerState>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Unified controller state for both campaigns and sellers
#[derive(Clone)]
pub struct ControllerStateSingleVariable {
    pub converging_variable: f64,
}

impl ControllerState for ControllerStateSingleVariable {
    fn clone_box(&self) -> Box<dyn ControllerState> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

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

    /// Create initial controller state
    pub fn create_controller_state(&self) -> Box<dyn ControllerState> {
        Box::new(ControllerStateSingleVariable { converging_variable: 1.0 })
    }

    /// Calculate pacing for next iteration based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `previous_state` - Previous controller state
    /// * `next_state` - Next controller state to be updated (mutable)
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    pub fn controller_next_state(&self, target: f64, actual: f64, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState) -> bool {
        let current_pacing = previous_state.as_any().downcast_ref::<ControllerStateSingleVariable>().unwrap().converging_variable;
        let next_state_mut = next_state.as_any_mut().downcast_mut::<ControllerStateSingleVariable>().unwrap();
        
        let tolerance = target * self.tolerance_fraction;
        
        if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 + adjustment_factor);
            next_state_mut.converging_variable = new_pacing;
            true
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 - adjustment_factor);
            next_state_mut.converging_variable = new_pacing;
            true
        } else {
            // Within tolerance - keep constant
            next_state_mut.converging_variable = current_pacing;
            false
        }
    }
    
    /// Get the converging variable from the controller state
    /// 
    /// # Arguments
    /// * `converge` - Controller state to extract the variable from
    /// 
    /// # Returns
    /// The control variable value
    pub fn get_control_variable(&self, converge: &dyn ControllerState) -> f64 {
        converge.as_any().downcast_ref::<ControllerStateSingleVariable>().unwrap().converging_variable
    }
}

/// Trait for controlling convergence behavior in campaigns
pub trait ConvergeController {
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

/// Constant implementation of ConvergeController
pub struct ConvergeControllerConstant {
    pub default_value: f64,
}

impl ConvergeControllerConstant {
    /// Create a new ConvergeControllerConstant with the given default value
    pub fn new(default_value: f64) -> Self {
        Self { default_value }
    }
}

impl ConvergeController for ConvergeControllerConstant {
    fn next_controller_state(&self, _previous_state: &dyn ControllerState, _next_state: &mut dyn ControllerState, _actual: f64, _target: f64) -> bool {
        // Constant controller - no convergence, always return false
        false
    }
    
    fn get_control_variable(&self, converge: &dyn ControllerState) -> f64 {
        converge.as_any().downcast_ref::<ControllerStateSingleVariable>().unwrap().converging_variable
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        Box::new(ControllerStateSingleVariable {
            converging_variable: self.default_value,
        })
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        format!("Constant value: {:.4}", self.get_control_variable(converge))
    }
}

/// Proportional controller implementation of ConvergeController
pub struct ConvergeControllerProportional {
    pub controller: ControllerProportional,
}

impl ConvergeControllerProportional {
    /// Create a new ConvergeControllerProportional
    pub fn new() -> Self {
        Self {
            controller: ControllerProportional::new(),
        }
    }
}

impl ConvergeController for ConvergeControllerProportional {
    fn next_controller_state(&self, previous_state: &dyn ControllerState, next_state: &mut dyn ControllerState, actual: f64, target: f64) -> bool {
        // Use the controller to calculate the next value
        self.controller.controller_next_state(target, actual, previous_state, next_state)
    }
    
    fn get_control_variable(&self, converge: &dyn ControllerState) -> f64 {
        self.controller.get_control_variable(converge)
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        self.controller.create_controller_state()
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        format!("Proportional controller, pacing: {:.4}", self.get_control_variable(converge))
    }
}

