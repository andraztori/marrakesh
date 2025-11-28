use crate::competition::{ImpressionCompetition, CompetitionGeneratorTrait};
use crate::floors::FloorGeneratorTrait;
use crate::controllers::ControllerTrait;
use crate::seller_targets::SellerTargetTrait;
use crate::seller_chargers::SellerCharger;
use rand::rngs::StdRng;
use std::any::Any;

/// Trait for sellers participating in auctions
pub trait SellerTrait: Any {
    /// Get the seller ID
    fn seller_id(&self) -> usize;
    
    /// Get the seller name
    fn seller_name(&self) -> &str;
    
    /// Get the number of impressions on offer
    fn get_impressions_on_offer(&self) -> usize;
    
    /// Get the supply cost in CPM for a given buyer winning bid CPM
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64;
    
    /// Generate impression parameters (Option<ImpressionCompetition>, floor_cpm) using the provided distributions
    /// 
    /// # Arguments
    /// * `base_value` - Base value parameter for floor generation
    /// * `rng` - Random number generator
    /// 
    /// # Returns
    /// Tuple of (Option<ImpressionCompetition>, floor_cpm)
    fn generate_impression(&self, base_value: f64, rng_competition: &mut StdRng, rng_floor: &mut StdRng) -> (Option<ImpressionCompetition>, f64);
    
    /// Get a string representation of the seller type and convergence for logging
    fn type_target_and_controller_state_string(&self, controller_states: &[&dyn crate::controllers::ControllerState]) -> String;
    
    /// Create a new convergence parameter for this seller type
    fn create_controller_state(&self) -> Vec<Box<dyn crate::controllers::ControllerState>>;
    
    /// Calculate the next controller state
    /// This method encapsulates the convergence logic for each seller type
    /// 
    /// # Arguments
    /// * `previous_states` - Previous controller states (immutable slice of Boxes)
    /// * `next_states` - Next controller states to be updated (mutable slice of Boxes)
    /// * `seller_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if boost_factor was changed, `false` if it remained the same
    fn next_controller_state(&self, previous_states: &[Box<dyn crate::controllers::ControllerState>], next_states: &mut [Box<dyn crate::controllers::ControllerState>], seller_stat: &crate::simulationrun::SellerStat) -> bool;
    
    /// Get the control variable (boost factor) from the controller state
    /// 
    /// # Arguments
    /// * `controller_state` - Controller state to extract the boost factor from
    /// 
    /// # Returns
    /// The control variable value (boost factor)
    fn get_control_variable(&self, controller_state: &dyn crate::controllers::ControllerState) -> f64;
    
    /// Get reference to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Get mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// General seller structure that can use any charging strategy
/// SellerGeneral supports any number of converge targets and controllers
pub struct SellerGeneral {
    pub seller_id: usize,
    pub seller_name: String,
    pub impressions_on_offer: usize,
    pub converge_targets: Vec<Box<dyn SellerTargetTrait>>,
    pub converge_controllers: Vec<Box<dyn ControllerTrait>>,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
    pub seller_charger: Box<dyn SellerCharger>,
}

impl SellerTrait for SellerGeneral {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn get_impressions_on_offer(&self) -> usize { self.impressions_on_offer }
    
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64 {
        self.seller_charger.get_supply_cost_cpm(buyer_win_cpm)
    }
    
    fn generate_impression(&self, base_value: f64, rng_competition: &mut StdRng, rng_floor: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(base_value, rng_competition);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng_floor);
        (competition, floor_cpm)
    }
    
    fn type_target_and_controller_state_string(&self, controller_states: &[&dyn crate::controllers::ControllerState]) -> String {
        let mut parts = Vec::new();
        for (index, (converge_target, converge_controller)) in self.converge_targets.iter().zip(self.converge_controllers.iter()).enumerate() {
            parts.push(format!("Target {}: {} (Cntrl: {})", 
                index + 1,
                converge_target.converge_target_string(),
                converge_controller.controller_string(controller_states[index])
            ));
        }
        format!("{} ({})", self.seller_charger.get_charging_type(), parts.join(", "))
    }
    
    fn create_controller_state(&self) -> Vec<Box<dyn crate::controllers::ControllerState>> {
        self.converge_controllers.iter().map(|c| c.create_controller_state()).collect()
    }
    
    fn next_controller_state(&self, previous_states: &[Box<dyn crate::controllers::ControllerState>], next_states: &mut [Box<dyn crate::controllers::ControllerState>], seller_stat: &crate::simulationrun::SellerStat) -> bool {
        let mut any_changed = false;
        for (index, (converge_target, converge_controller)) in self.converge_targets.iter().zip(self.converge_controllers.iter()).enumerate() {
            let (actual, target) = converge_target.get_actual_and_target(seller_stat);
            let changed = converge_controller.next_controller_state(previous_states[index].as_ref(), next_states[index].as_mut(), actual, target);
            any_changed = any_changed || changed;
        }
        any_changed
    }
    
    fn get_control_variable(&self, controller_state: &dyn crate::controllers::ControllerState) -> f64 {
        // For sellers, we typically use the first controller's control variable
        // This maintains backward compatibility with existing code
        self.converge_controllers[0].get_control_variable(controller_state)
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

