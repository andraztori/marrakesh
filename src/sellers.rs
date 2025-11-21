use crate::competition::{ImpressionCompetition, CompetitionGeneratorTrait};
use crate::floors::FloorGeneratorTrait;
use crate::controllers::ConvergeController;
use rand::rngs::StdRng;
pub use crate::converge::ConvergeTargetAny;

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
pub trait SellerTrait {
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
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String;
    
    /// Create a new convergence parameter for this seller type
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState>;
    
    /// Calculate the next controller state
    /// This method encapsulates the convergence logic for each seller type
    /// 
    /// # Arguments
    /// * `previous_state` - Previous controller state (immutable)
    /// * `next_state` - Next controller state to be updated (mutable)
    /// * `seller_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if boost_factor was changed, `false` if it remained the same
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, seller_stat: &crate::simulationrun::SellerStat) -> bool;
    
}

/// General seller structure that can use any charging strategy
pub struct SellerGeneral {
    pub seller_id: usize,
    pub seller_name: String,
    pub impressions_on_offer: usize,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::SellerStat>>,
    pub converge_controller: Box<dyn ConvergeController>,
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
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String {
        format!("{} ({}, {})", self.seller_charger.get_charging_type(), self.converge_target.converge_target_string(), self.converge_controller.controller_string(controller_state))
    }
    
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState> {
        self.converge_controller.create_controller_state()
    }
    
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        let (actual, target) = self.converge_target.get_actual_and_target(seller_stat);
        self.converge_controller.next_controller_state(previous_state, next_state, actual, target)
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
        
        // Create converge_target and converge_controller based on seller_converge
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
                    converge_target,
                    converge_controller,
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
                    converge_target,
                    converge_controller,
                    competition_generator,
                    floor_generator,
                    seller_charger,
                }));
            }
        }
    }
}
