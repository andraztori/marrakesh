/// Unified trait for controller state
/// Used for both campaigns and sellers
pub trait ControllerState: std::any::Any {
    fn clone_box(&self) -> Box<dyn ControllerState>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Empty controller state with no data
/// Useful for controllers that don't need to store any state = constnat or random
#[derive(Clone)]
pub struct ControllerStateEmpty;

impl ControllerState for ControllerStateEmpty {
    fn clone_box(&self) -> Box<dyn ControllerState> { Box::new(ControllerStateEmpty) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
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

/// Controller state with two variables (for campaigns with dual convergence targets)
#[derive(Clone)]
pub struct ControllerStateDualVariable {
    pub converging_variable_1: f64,
    pub converging_variable_2: f64,
}

impl ControllerState for ControllerStateDualVariable {
    fn clone_box(&self) -> Box<dyn ControllerState> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
