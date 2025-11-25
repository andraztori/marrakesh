// Re-export ControllerState types from controller_state module
pub use crate::controller_state::*;

// Re-export ControllerProportional from controller_core module
pub use crate::controller_core::ControllerProportional;

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
        format!("Proportional, pacing: {:.4}", self.get_control_variable(converge))
    }
}

/// Trait for controlling convergence behavior with dual targets
/// Similar to ConvergeController but handles two targets simultaneously
pub trait ConvergeControllerDouble {
    /// Calculate the next controller state with dual targets
    /// 
    /// # Arguments
    /// * `previous_state` - Previous controller state
    /// * `next_state` - Next controller state to update
    /// * `actual_primary` - Actual value achieved for primary target
    /// * `target_primary` - Target value for primary target
    /// * `actual_secondary` - Actual value achieved for secondary target
    /// * `target_secondary` - Target value for secondary target
    /// 
    /// # Returns
    /// `true` if the convergence value changed, `false` if it remained the same
    fn next_controller_state(
        &self,
        previous_state: &dyn ControllerState,
        next_state: &mut dyn ControllerState,
        actual_primary: f64,
        target_primary: f64,
        actual_secondary: f64,
        target_secondary: f64,
    ) -> bool;
    
    /// Get the primary control variable (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Controller state to extract the primary pacing value from
    fn get_control_variable_primary(&self, converge: &dyn ControllerState) -> f64;
    
    /// Get the secondary control variable (pacing value)
    /// 
    /// # Arguments
    /// * `converge` - Controller state to extract the secondary pacing value from
    fn get_control_variable_secondary(&self, converge: &dyn ControllerState) -> f64;
    
    /// Create initial controller state
    fn create_controller_state(&self) -> Box<dyn ControllerState>;
    
    /// Get a string representation of the controller state
    /// 
    /// # Arguments
    /// * `converge` - Controller state to include pacing information
    fn controller_string(&self, converge: &dyn ControllerState) -> String;
}

/// Proportional controller implementation for dual targets
/// Uses two ControllerProportional instances to control both primary and secondary targets
pub struct ConvergeDoubleProportionalController {
    pub controller_primary: ControllerProportional,
    pub controller_secondary: ControllerProportional,
}

impl ConvergeDoubleProportionalController {
    /// Create a new ConvergeDoubleProportionalController with default parameters
    pub fn new() -> Self {
        Self {
            controller_primary: ControllerProportional::new(),
            controller_secondary: ControllerProportional::new_advanced(
                0.005, // tolerance_fraction
                0.5,   // max_adjustment_factor
                1.0,   // proportional_gain
            ),
        }
    }
}

impl ConvergeControllerDouble for ConvergeDoubleProportionalController {
    fn next_controller_state(
        &self,
        previous_state: &dyn ControllerState,
        next_state: &mut dyn ControllerState,
        actual_primary: f64,
        target_primary: f64,
        actual_secondary: f64,
        target_secondary: f64,
    ) -> bool {
        // Extract previous dual state
        let prev_dual = previous_state.as_any().downcast_ref::<ControllerStateDualVariable>().unwrap();
        let next_dual = next_state.as_any_mut().downcast_mut::<ControllerStateDualVariable>().unwrap();
        //println!("actual_primary: {:.4}, target_primary: {:.4}", actual_primary, target_primary);
        //println!("actual_secondary: {:.4}, target_secondary: {:.4}", actual_secondary, target_secondary);
        // Update both controllers independently using the new signature
        let (primary_changed, next_primary_pacing) = self.controller_primary.controller_next_state(
            target_primary,
            actual_primary,
            prev_dual.converging_variable_1,
        );
        
        let (secondary_changed, next_secondary_pacing) = self.controller_secondary.controller_next_state(
            target_secondary,
            actual_secondary,
            prev_dual.converging_variable_2,
        );
        
        // Update the dual state
        next_dual.converging_variable_1 = next_primary_pacing;
        next_dual.converging_variable_2 = next_secondary_pacing;
        
        primary_changed || secondary_changed
    }
    
    fn get_control_variable_primary(&self, converge: &dyn ControllerState) -> f64 {
        let dual_state = converge.as_any().downcast_ref::<ControllerStateDualVariable>().unwrap();
        
        dual_state.converging_variable_1
    }
    
    fn get_control_variable_secondary(&self, converge: &dyn ControllerState) -> f64 {
        let dual_state = converge.as_any().downcast_ref::<ControllerStateDualVariable>().unwrap();
        
        dual_state.converging_variable_2
    }
    
    fn create_controller_state(&self) -> Box<dyn ControllerState> {
        // Directly create ControllerStateDualVariable with initial values of 1.0
        Box::new(ControllerStateDualVariable {
            converging_variable_1: 1.0,
            converging_variable_2: 1.0,
        })
    }
    
    fn controller_string(&self, converge: &dyn ControllerState) -> String {
        let dual_state = converge.as_any().downcast_ref::<ControllerStateDualVariable>().unwrap();
        
        format!(
            "Proportional, primary_pacing: {:.4}, secondary_pacing: {:.4}",
            dual_state.converging_variable_1,
            dual_state.converging_variable_2
        )
    }
}

