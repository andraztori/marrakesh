use crate::competition::{ImpressionCompetition, CompetitionGeneratorTrait};
use crate::floors::FloorGeneratorTrait;
use crate::utils::ControllerProportional;
use rand::rngs::StdRng;

/// Seller type for different pricing models
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum SellerType {
    FIXED_COST_FIXED_BOOST { fixed_cost_cpm: f64 },
    FIXED_COST_DYNAMIC_BOOST { fixed_cost_cpm: f64 },
    FIRST_PRICE,
}

/// Trait for seller convergence parameters
/// Each seller type has its own associated convergence parameter type
pub trait SellerConverge: std::any::Any {
    /// Clone the convergence parameter
    fn clone_box(&self) -> Box<dyn SellerConverge>;
    
    /// Get a reference to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get a mutable reference to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Convergence parameter for seller boost factor
#[derive(Clone)]
pub struct SellerConvergeBoost {
    pub boost_factor: f64,
}

impl SellerConverge for SellerConvergeBoost {
    fn clone_box(&self) -> Box<dyn SellerConverge> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Trait for sellers participating in auctions
pub trait SellerTrait {
    /// Get the seller ID
    fn seller_id(&self) -> usize;
    
    /// Get the seller name
    fn seller_name(&self) -> &str;
    
    /// Get the number of impressions
    fn num_impressions(&self) -> usize;
    
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
    fn create_converge(&self) -> Box<dyn SellerConverge>;
    
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
    fn converge_iteration(&self, current_converge: &dyn SellerConverge, next_converge: &mut dyn SellerConverge, seller_stat: &crate::simulationrun::SellerStat) -> bool;
    
    /// Get a formatted string representation of the convergence parameters
    fn converge_string(&self, converge: &dyn SellerConverge) -> String;
}

/// Seller with fixed cost pricing and fixed boost factor
pub struct SellerFixedCostFixedBoost {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub num_impressions: usize,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
}

impl SellerTrait for SellerFixedCostFixedBoost {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn num_impressions(&self) -> usize { self.num_impressions }
    
    fn get_supply_cost_cpm(&self, _buyer_win_cpm: f64) -> f64 {
        self.fixed_cost_cpm
    }
    
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(rng);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng);
        (competition, floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        format!("FIXED_COST ({} CPM)", self.fixed_cost_cpm)
    }
    
    fn create_converge(&self) -> Box<dyn SellerConverge> {
        Box::new(SellerConvergeBoost { boost_factor: 1.0 })
    }
    
    fn converge_iteration(&self, _current_converge: &dyn SellerConverge, _next_converge: &mut dyn SellerConverge, _seller_stat: &crate::simulationrun::SellerStat) -> bool {
        // Fixed boost - no convergence
        false
    }
    
    fn converge_string(&self, converge: &dyn SellerConverge) -> String {
        let converge = converge.as_any().downcast_ref::<SellerConvergeBoost>().unwrap();
        format!("Fixed Boost: {:.2}", converge.boost_factor)
    }
}

/// Seller with fixed cost pricing and dynamic boost factor
pub struct SellerFixedCostDynamicBoost {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub num_impressions: usize,
    pub boost_converger: ControllerProportional,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
}

impl SellerTrait for SellerFixedCostDynamicBoost {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn num_impressions(&self) -> usize { self.num_impressions }
    
    fn get_supply_cost_cpm(&self, _buyer_win_cpm: f64) -> f64 {
        self.fixed_cost_cpm
    }
    
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(rng);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng);
        (competition, floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        format!("FIXED_COST ({} CPM)", self.fixed_cost_cpm)
    }
    
    fn create_converge(&self) -> Box<dyn SellerConverge> {
        Box::new(SellerConvergeBoost { boost_factor: 1.0 })
    }
    
    fn converge_iteration(&self, current_converge: &dyn SellerConverge, next_converge: &mut dyn SellerConverge, seller_stat: &crate::simulationrun::SellerStat) -> bool {
        // Downcast to concrete types at the beginning
        let current_converge = current_converge.as_any().downcast_ref::<SellerConvergeBoost>().unwrap();
        let next_converge = next_converge.as_any_mut().downcast_mut::<SellerConvergeBoost>().unwrap();
        
        // Converge when cost of impressions (num_impressions * fixed_cost_cpm) matches virtual price
        // fixed_cost_cpm is in CPM (cost per 1000 impressions), so divide by 1000 to get cost per impression
        let target = (self.num_impressions as f64) * self.fixed_cost_cpm / 1000.0;
        let actual = seller_stat.total_virtual_cost;
        let current_boost = current_converge.boost_factor;
        
        // Use the same controller logic as campaigns, but for boost_factor
        let (new_boost, changed) = self.boost_converger.pacing_in_next_iteration(target, actual, current_boost);
        next_converge.boost_factor = new_boost;
        
        changed
    }
    
    fn converge_string(&self, converge: &dyn SellerConverge) -> String {
        let converge = converge.as_any().downcast_ref::<SellerConvergeBoost>().unwrap();
        format!("Dynamic Boost: {:.2}", converge.boost_factor)
    }
}

/// Seller with first price auction
pub struct SellerFirstPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub num_impressions: usize,
    pub competition_generator: Box<dyn CompetitionGeneratorTrait>,
    pub floor_generator: Box<dyn FloorGeneratorTrait>,
}

impl SellerTrait for SellerFirstPrice {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn num_impressions(&self) -> usize { self.num_impressions }
    
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64 {
        buyer_win_cpm
    }
    
    fn generate_impression(&self, base_value: f64, rng: &mut StdRng) -> (Option<ImpressionCompetition>, f64) {
        let competition = self.competition_generator.generate_competition(rng);
        let floor_cpm = self.floor_generator.generate_floor(base_value, rng);
        (competition, floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        "FIRST_PRICE".to_string()
    }
    
    fn create_converge(&self) -> Box<dyn SellerConverge> {
        Box::new(SellerConvergeBoost { boost_factor: 1.0 })
    }
    
    fn converge_iteration(&self, _current_converge: &dyn SellerConverge, _next_converge: &mut dyn SellerConverge, _seller_stat: &crate::simulationrun::SellerStat) -> bool {
        // First price sellers don't converge boost
        false
    }
    
    fn converge_string(&self, converge: &dyn SellerConverge) -> String {
        let converge = converge.as_any().downcast_ref::<SellerConvergeBoost>().unwrap();
        format!("Fixed Boost: {:.2}", converge.boost_factor)
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
    /// * `seller_type` - Seller type (FIXED_COST_FIXED_BOOST, FIXED_COST_DYNAMIC_BOOST, or FIRST_PRICE)
    /// * `num_impressions` - Number of impressions this seller will offer
    /// * `competition_generator` - Generator for impression competition data
    /// * `floor_generator` - Generator for floor CPM values
    pub fn add(&mut self, seller_name: String, seller_type: SellerType, num_impressions: usize, competition_generator: Box<dyn CompetitionGeneratorTrait>, floor_generator: Box<dyn FloorGeneratorTrait>) {
        let seller_id = self.sellers.len();
        match seller_type {
            SellerType::FIXED_COST_FIXED_BOOST { fixed_cost_cpm } => {
                self.sellers.push(Box::new(SellerFixedCostFixedBoost {
                    seller_id,
                    seller_name,
                    fixed_cost_cpm,
                    num_impressions,
                    competition_generator,
                    floor_generator,
                }));
            }
            SellerType::FIXED_COST_DYNAMIC_BOOST { fixed_cost_cpm } => {
                self.sellers.push(Box::new(SellerFixedCostDynamicBoost {
                    seller_id,
                    seller_name,
                    fixed_cost_cpm,
                    num_impressions,
                    boost_converger: ControllerProportional::new(),
                    competition_generator,
                    floor_generator,
                }));
            }
            SellerType::FIRST_PRICE => {
                self.sellers.push(Box::new(SellerFirstPrice {
                    seller_id,
                    seller_name,
                    num_impressions,
                    competition_generator,
                    floor_generator,
                }));
            }
        }
    }
}
