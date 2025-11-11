use crate::impressions::DistributionF64;
use rand::rngs::StdRng;

/// Seller type for different pricing models
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum SellerType {
    FIXED_COST { fixed_cost_cpm: f64 },
    FIRST_PRICE,
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
    
    /// Generate impression parameters (best_other_bid_cpm, floor_cpm) using the provided distributions
    /// 
    /// # Arguments
    /// * `best_other_bid_dist` - Distribution for generating best_other_bid_cpm (used for FIRST_PRICE)
    /// * `floor_cpm_dist` - Distribution for generating floor_cpm (used for FIRST_PRICE)
    /// * `fixed_cost_floor_cpm` - Fixed floor CPM value (used for FIXED_COST)
    /// * `rng` - Random number generator
    /// 
    /// # Returns
    /// Tuple of (best_other_bid_cpm, floor_cpm)
    fn generate_impression(&self, best_other_bid_dist: &dyn DistributionF64, floor_cpm_dist: &dyn DistributionF64, fixed_cost_floor_cpm: f64, rng: &mut StdRng) -> (f64, f64);
    
    /// Get a string representation of the charge type for logging
    fn charge_type_string(&self) -> String;
}

/// Seller with fixed cost pricing
pub struct SellerFixedCost {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub num_impressions: usize,
}

impl SellerTrait for SellerFixedCost {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn num_impressions(&self) -> usize { self.num_impressions }
    
    fn get_supply_cost_cpm(&self, _buyer_win_cpm: f64) -> f64 {
        self.fixed_cost_cpm
    }
    
    fn generate_impression(&self, _best_other_bid_dist: &dyn DistributionF64, _floor_cpm_dist: &dyn DistributionF64, fixed_cost_floor_cpm: f64, _rng: &mut StdRng) -> (f64, f64) {
        (0.0, fixed_cost_floor_cpm)
    }
    
    fn charge_type_string(&self) -> String {
        format!("FIXED_COST ({} CPM)", self.fixed_cost_cpm)
    }
}

/// Seller with first price auction
pub struct SellerFirstPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub num_impressions: usize,
}

impl SellerTrait for SellerFirstPrice {
    fn seller_id(&self) -> usize { self.seller_id }
    fn seller_name(&self) -> &str { &self.seller_name }
    fn num_impressions(&self) -> usize { self.num_impressions }
    
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64 {
        buyer_win_cpm
    }
    
    fn generate_impression(&self, best_other_bid_dist: &dyn DistributionF64, floor_cpm_dist: &dyn DistributionF64, _fixed_cost_floor_cpm: f64, rng: &mut StdRng) -> (f64, f64) {
        (
            best_other_bid_dist.sample(rng),
            floor_cpm_dist.sample(rng),
        )
    }
    
    fn charge_type_string(&self) -> String {
        "FIRST_PRICE".to_string()
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
    /// * `seller_type` - Seller type (FIXED_COST with fixed_cost_cpm, or FIRST_PRICE)
    /// * `num_impressions` - Number of impressions this seller will offer
    pub fn add(&mut self, seller_name: String, seller_type: SellerType, num_impressions: usize) {
        let seller_id = self.sellers.len();
        match seller_type {
            SellerType::FIXED_COST { fixed_cost_cpm } => {
                self.sellers.push(Box::new(SellerFixedCost {
                    seller_id,
                    seller_name,
                    fixed_cost_cpm,
                    num_impressions,
                }));
            }
            SellerType::FIRST_PRICE => {
                self.sellers.push(Box::new(SellerFirstPrice {
                    seller_id,
                    seller_name,
                    num_impressions,
                }));
            }
        }
    }
}
