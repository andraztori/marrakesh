use crate::competition::{ImpressionCompetition, CompetitionGeneratorTrait};
use crate::floors::FloorGeneratorTrait;
use crate::controllers::ConvergeController;
use rand::rngs::StdRng;
pub use crate::converge::ConvergeTargetAny;
use std::any::Any;

/// Seller type for different pricing models
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum SellerType {
    FIRST_PRICE,
    FIXED_PRICE { fixed_cost_cpm: f64 },
}

/// Convergence strategy for sellers
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum SellerConvergeStrategy {
    NONE { default_value: f64 },
    TOTAL_COST { target_total_cost: f64 },
}

// Re-export convergence target types for convenience
pub use crate::seller_targets::{ConvergeNone, ConvergeTargetTotalCost};
// Re-export charger types for convenience
pub use crate::seller_chargers::{SellerCharger, SellerChargerFirstPrice, SellerChargerFixedPrice};

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
    /// * `previous_states` - Previous controller states (immutable slice)
    /// * `next_states` - Next controller states to be updated (mutable slice)
    /// * `seller_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if boost_factor was changed, `false` if it remained the same
    fn next_controller_state(&self, previous_states: &[&dyn crate::controllers::ControllerState], next_states: &mut [&mut dyn crate::controllers::ControllerState], seller_stat: &crate::simulationrun::SellerStat) -> bool;
    
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
    pub converge_targets: Vec<Box<dyn ConvergeTargetAny<crate::simulationrun::SellerStat>>>,
    pub converge_controllers: Vec<Box<dyn ConvergeController>>,
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
    
    fn next_controller_state(&self, previous_states: &[&dyn crate::controllers::ControllerState], next_states: &mut [&mut dyn crate::controllers::ControllerState], seller_stat: &crate::simulationrun::SellerStat) -> bool {
        let mut any_changed = false;
        for (index, (converge_target, converge_controller)) in self.converge_targets.iter().zip(self.converge_controllers.iter()).enumerate() {
            let (actual, target) = converge_target.get_actual_and_target(seller_stat);
            let changed = converge_controller.next_controller_state(previous_states[index], next_states[index], actual, target);
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

/// Container for sellers with methods to add sellers
/// Uses trait objects to support different seller types
pub struct Sellers {
    pub sellers: Vec<Box<dyn SellerTrait>>,
}

impl Sellers {
    pub fn new() -> Self {
        Self {
            sellers: Vec::new(),
        }
    }

    /// Add a seller to the collection
    /// 
    /// # Arguments
    /// * `seller_name` - Name of the seller
    /// * `seller_type` - Seller type (FIRST_PRICE or FIXED_PRICE)
    /// * `seller_converge` - Convergence strategy (NONE or TOTAL_COST)
    /// * `impressions_on_offer` - Number of impressions this seller will offer
    /// * `competition_generator` - Generator for impression competition data
    /// * `floor_generator` - Generator for floor CPM values
    pub fn add(&mut self, seller_name: String, seller_type: SellerType, seller_converge: SellerConvergeStrategy, impressions_on_offer: usize, competition_generator: Box<dyn CompetitionGeneratorTrait>, floor_generator: Box<dyn FloorGeneratorTrait>) {
        let seller_id = self.sellers.len();
        
        // Create converge_targets and converge_controllers based on seller_converge
        let (converge_target, converge_controller): (Box<dyn ConvergeTargetAny<crate::simulationrun::SellerStat>>, Box<dyn ConvergeController>) = match seller_converge {
            SellerConvergeStrategy::NONE { default_value } => {
                (
                    Box::new(ConvergeNone),
                    Box::new(crate::controllers::ConvergeControllerConstant::new(default_value))
                )
            }
            SellerConvergeStrategy::TOTAL_COST { target_total_cost } => {
                (
                    Box::new(ConvergeTargetTotalCost {
                        target_cost: target_total_cost,
                    }),
                    Box::new(crate::controllers::ConvergeControllerProportional::new())
                )
            }
        };
        
        // Create seller based on seller_type
        match seller_type {
            SellerType::FIRST_PRICE => {
                let seller_charger = Box::new(SellerChargerFirstPrice) as Box<dyn SellerCharger>;
                self.sellers.push(Box::new(SellerGeneral {
                    seller_id,
                    seller_name,
                    impressions_on_offer,
                    converge_targets: vec![converge_target],
                    converge_controllers: vec![converge_controller],
                    competition_generator,
                    floor_generator,
                    seller_charger,
                }));
            }
            SellerType::FIXED_PRICE { fixed_cost_cpm } => {
                let seller_charger = Box::new(SellerChargerFixedPrice {
                    fixed_cost_cpm,
                }) as Box<dyn SellerCharger>;
                self.sellers.push(Box::new(SellerGeneral {
                    seller_id,
                    seller_name,
                    impressions_on_offer,
                    converge_targets: vec![converge_target],
                    converge_controllers: vec![converge_controller],
                    competition_generator,
                    floor_generator,
                    seller_charger,
                }));
            }
        }
    }

    /// Add a seller using an advanced method that accepts a pre-constructed SellerTrait
    /// 
    /// # Arguments
    /// * `seller` - A boxed SellerTrait object. The seller_id will be set to the current length of sellers.
    pub fn add_advanced(&mut self, mut seller: Box<dyn SellerTrait>) {
        let seller_id = self.sellers.len();
        
        // Try to downcast to SellerGeneral to set the seller_id
        if let Some(seller_general) = seller.as_mut().as_any_mut().downcast_mut::<SellerGeneral>() {
            seller_general.seller_id = seller_id;
        }
        
        self.sellers.push(seller);
    }
}
