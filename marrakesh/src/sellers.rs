use crate::competition::{ImpressionCompetition, CompetitionGeneratorTrait};
use crate::floors::FloorGeneratorTrait;
use crate::converge::ControllerProportional;
use rand::rngs::StdRng;
pub use crate::converge::{ConvergingSingleVariable, ConvergeAny};

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
pub struct ConvergeNone {
    pub default_value: f64,
}

impl ConvergeAny<crate::simulationrun::SellerStat> for ConvergeNone {
    fn converge_iteration(&self, _current_converge: &dyn crate::converge::ConvergingVariables, _next_converge: &mut dyn crate::converge::ConvergingVariables, _seller_stat: &crate::simulationrun::SellerStat) -> bool {
        // No convergence - boost stays the same
        false
    }
    
    fn get_converging_variable(&self, _converge: &dyn crate::converge::ConvergingVariables) -> f64 {
        self.default_value
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        // Ideally we could simply not allocate anything... but simpler to keep something
        Box::new(ConvergingSingleVariable { converging_variable: self.default_value })
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        format!("No convergence, boost: {:.2}", self.get_converging_variable(converge))
    }
}

/// Convergence strategy for sellers that converge boost to match target cost
pub struct ConvergeTotalCost {
    pub target_cost: f64,
    pub controller: ControllerProportional,
}

impl ConvergeAny<crate::simulationrun::SellerStat> for ConvergeTotalCost {
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        let target = self.target_cost;
        let actual = seller_stat.total_virtual_cost;
        
        // Use the same controller logic as campaigns, but for boost_factor
        self.controller.converge_next_iteration(target, actual, current_converge, next_converge)
    }
    
    fn get_converging_variable(&self, converge: &dyn crate::converge::ConvergingVariables) -> f64 {
        self.controller.get_converging_variable(converge)
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.controller.create_converging_variables()
    }
    
    fn converge_target_string(&self, converge: &dyn crate::converge::ConvergingVariables) -> String {
        let boost = converge.as_any().downcast_ref::<ConvergingSingleVariable>().unwrap().converging_variable;
        format!("Target cost: {:.2}, boost: {:.2}", self.target_cost, boost)
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
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64);
    
    /// Get a string representation of the charge type for logging
    fn charge_type_string(&self) -> String;
    
    /// Create a new convergence parameter for this seller type
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables>;
    
    /// Perform one iteration of convergence, updating the next convergence parameter
    /// This method encapsulates the convergence logic for each seller type
    /// 
    /// # Arguments
    /// * `current_converge` - Current convergence parameter (immutable)
    /// * `next_converge` - Next convergence parameter to be updated (mutable)
    /// * `seller_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if boost_factor was changed, `false` if it remained the same
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, seller_stat: &crate::simulationrun::SellerStat) -> bool;
    
}

/// Seller with first price auction
pub struct SellerFirstPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub impressions_on_offer: usize,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::SellerStat>>,
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
    
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(base_value, rng);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng);
        (competition, floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        "FIRST_PRICE".to_string()
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, seller_stat)
    }
}

/// Seller with fixed price (cost per mille)
pub struct SellerFixedPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub impressions_on_offer: usize,
    pub converger: Box<dyn ConvergeAny<crate::simulationrun::SellerStat>>,
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
    
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(base_value, rng);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng);
        (competition, floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        format!("FIXED_PRICE ({} CPM)", self.fixed_cost_cpm)
    }
    
    fn create_converging_variables(&self) -> Box<dyn crate::converge::ConvergingVariables> {
        self.converger.create_converging_variables()
    }
    
    fn converge_iteration(&self, current_converge: &dyn crate::converge::ConvergingVariables, next_converge: &mut dyn crate::converge::ConvergingVariables, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        self.converger.converge_iteration(current_converge, next_converge, seller_stat)
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
        
        // Create converger based on seller_converge
        let converger: Box<dyn ConvergeAny<crate::simulationrun::SellerStat>> = match seller_converge {
            SellerConvergeStrategy::NONE { default_value } => {
                Box::new(ConvergeNone { default_value })
            }
            SellerConvergeStrategy::TOTAL_COST { target_total_cost } => {
                Box::new(ConvergeTotalCost {
                    target_cost: target_total_cost,
                    controller: ControllerProportional::new(),
                })
            }
        };
        
        // Create seller based on seller_type
        match seller_type {
            SellerType::FIRST_PRICE => {
                self.sellers.push(Box::new(SellerFirstPrice {
                    seller_id,
                    seller_name,
                    impressions_on_offer,
                    converger,
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
                    converger,
                    competition_generator,
                    floor_generator,
                }));
            }
        }
    }
}
