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

/// Convergence strategy for sellers that don't converge (no boost adjustment)
pub struct ConvergeNone;

impl ConvergeTargetAny<crate::simulationrun::SellerStat> for ConvergeNone {
    fn get_actual_and_target(&self, _seller_stat: &crate::simulationrun::SellerStat) -> (f64, f64) {
        // No convergence, so no target or actual values
        (0.0, 0.0)
    }
    
    fn converge_target_string(&self) -> String {
        "No convergence".to_string()
    }
}

/// Convergence strategy for sellers that converge boost to match target cost
pub struct ConvergeTargetTotalCost {
    pub target_cost: f64,
}

impl ConvergeTargetAny<crate::simulationrun::SellerStat> for ConvergeTargetTotalCost {
    fn get_actual_and_target(&self, seller_stat: &crate::simulationrun::SellerStat) -> (f64, f64) {
        let actual = seller_stat.total_virtual_cost;
        let target = self.target_cost;
        (actual, target)
    }
    
    fn converge_target_string(&self) -> String {
        format!("Converge target cost: {:.2}", self.target_cost)
    }
}

/// Trait for sellers participating in auctions
pub trait SellerTrait {
    /// Get the seller ID
    fn seller_id(&self) -> usize;
    
    /// Get the seller name
    fn seller_name(&self) -> &str;
    
    /// Get the number of impressions on offer
    fn get_impressions_on_offer(&self) -> usize;
    
    /// Get the supply cost in CPM for a given buyer winning bid CPM
    /// For fixed cost sellers, returns the fixed_cost_cpm
    /// For first price sellers, returns the buyer_win_cpm
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

/// Seller with first price auction
pub struct SellerFirstPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub impressions_on_offer: usize,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::SellerStat>>,
    pub converge_controller: Box<dyn ConvergeController>,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
}

impl SellerTrait for SellerFirstPrice {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn get_impressions_on_offer(&self) -> usize { self.impressions_on_offer }
    
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64 {
        buyer_win_cpm
    }
    
    fn generate_impression(&self, base_value: f64, rng_competition: &mut StdRng, rng_floor: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(base_value, rng_competition);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng_floor);
        (competition, floor_cpm)
    }
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String {
        format!("First price ({}, {})", self.converge_target.converge_target_string(), self.converge_controller.controller_string(controller_state))
    }
    
    fn create_controller_state(&self) -> Box<dyn crate::controllers::ControllerState> {
        self.converge_controller.create_controller_state()
    }
    
    fn next_controller_state(&self, previous_state: &dyn crate::controllers::ControllerState, next_state: &mut dyn crate::controllers::ControllerState, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        let (actual, target) = self.converge_target.get_actual_and_target(seller_stat);
        self.converge_controller.next_controller_state(previous_state, next_state, actual, target)
    }
}

/// Seller with fixed price (cost per mille)
pub struct SellerFixedPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub impressions_on_offer: usize,
    pub converge_target: Box<dyn ConvergeTargetAny<crate::simulationrun::SellerStat>>,
    pub converge_controller: Box<dyn ConvergeController>,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
}

impl SellerTrait for SellerFixedPrice {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn get_impressions_on_offer(&self) -> usize { self.impressions_on_offer }
    
    fn get_supply_cost_cpm(&self, _buyer_win_cpm: f64) -> f64 {
        self.fixed_cost_cpm
    }
    
    fn generate_impression(&self, base_value: f64, rng_competition: &mut StdRng, rng_floor: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(base_value, rng_competition);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng_floor);
        (competition, floor_cpm)
    }
    
    fn type_target_and_controller_state_string(&self, controller_state: &dyn crate::controllers::ControllerState) -> String {
        format!("Fixed price CPM: {:.2} ({}, {})", self.fixed_cost_cpm, self.converge_target.converge_target_string(), self.converge_controller.controller_string(controller_state))
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
                self.sellers.push(Box::new(SellerFirstPrice {
                    seller_id,
                    seller_name,
                    impressions_on_offer,
                    converge_target,
                    converge_controller,
                    competition_generator,
                    floor_generator,
                }));
            }
            SellerType::FIXED_PRICE { fixed_cost_cpm } => {
                self.sellers.push(Box::new(SellerFixedPrice {
                    seller_id,
                    seller_name,
                    fixed_cost_cpm,
                    impressions_on_offer,
                    converge_target,
                    converge_controller,
                    competition_generator,
                    floor_generator,
                }));
            }
        }
    }
}
