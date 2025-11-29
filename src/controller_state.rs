/// Unified trait for controller state
/// Used for both campaigns and sellers
pub trait ControllerStateTrait: std::any::Any {
    fn clone_box(&self) -> Box<dyn ControllerStateTrait>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Empty controller state with no data
/// Useful for controllers that don't need to store any state = constnat or random
#[derive(Clone)]
pub struct ControllerStateEmpty;

impl ControllerStateTrait for ControllerStateEmpty {
    fn clone_box(&self) -> Box<dyn ControllerStateTrait> { Box::new(ControllerStateEmpty) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}


/// Unified controller state for both campaigns and sellers
#[derive(Clone)]
pub struct ControllerStateSingleVariable {
    pub converging_variable: f64,
}

impl ControllerStateTrait for ControllerStateSingleVariable {
    fn clone_box(&self) -> Box<dyn ControllerStateTrait> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Controller state with two variables (e.g., for PD controller: pacing and previous error)
#[derive(Clone)]
pub struct ControllerStateDoubleVariable {
    pub variable1: f64,
    pub variable2: Option<f64>,
}

impl ControllerStateTrait for ControllerStateDoubleVariable {
    fn clone_box(&self) -> Box<dyn ControllerStateTrait> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

