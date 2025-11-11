use crate::types::ChargeType;

/// Trait for sellers participating in auctions
pub trait SellerTrait {
    /// Get the seller ID
    fn seller_id(&self) -> usize;
    
    /// Get the seller name
    fn seller_name(&self) -> &str;
    
    /// Get the charge type
    fn charge_type(&self) -> ChargeType;
    
    /// Get the number of impressions
    fn num_impressions(&self) -> usize;
    
    /// Get the supply cost in CPM for a given value
    /// For fixed cost sellers, returns the fixed_cost_cpm
    /// For first price sellers, returns the provided value
    fn get_supply_cost_cpm(&self, value: f64) -> f64;
}

/// Seller with fixed cost pricing
pub struct SellerFixedCost {
    pub seller_id: usize,
    pub seller_name: String,
    pub fixed_cost_cpm: f64,
    pub num_impressions: usize,
}

impl SellerTrait for SellerFixedCost {
    fn seller_id(&self) -> usize {
        self.seller_id
    }
    
    fn seller_name(&self) -> &str {
        &self.seller_name
    }
    
    fn charge_type(&self) -> ChargeType {
        ChargeType::FIXED_COST {
            fixed_cost_cpm: self.fixed_cost_cpm,
        }
    }
    
    fn num_impressions(&self) -> usize {
        self.num_impressions
    }
    
    fn get_supply_cost_cpm(&self, _value: f64) -> f64 {
        self.fixed_cost_cpm
    }
}

/// Seller with first price auction
pub struct SellerFirstPrice {
    pub seller_id: usize,
    pub seller_name: String,
    pub num_impressions: usize,
}

impl SellerTrait for SellerFirstPrice {
    fn seller_id(&self) -> usize {
        self.seller_id
    }
    
    fn seller_name(&self) -> &str {
        &self.seller_name
    }
    
    fn charge_type(&self) -> ChargeType {
        ChargeType::FIRST_PRICE
    }
    
    fn num_impressions(&self) -> usize {
        self.num_impressions
    }
    
    fn get_supply_cost_cpm(&self, value: f64) -> f64 {
        value
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
    /// * `charge_type` - Charge type (FIXED_COST with fixed_cost_cpm, or FIRST_PRICE)
    /// * `num_impressions` - Number of impressions this seller will offer
    pub fn add(&mut self, seller_name: String, charge_type: ChargeType, num_impressions: usize) {
        let seller_id = self.sellers.len();
        match charge_type {
            ChargeType::FIXED_COST { fixed_cost_cpm } => {
                self.sellers.push(Box::new(SellerFixedCost {
                    seller_id,
                    seller_name,
                    fixed_cost_cpm,
                    num_impressions,
                }));
            }
            ChargeType::FIRST_PRICE => {
                self.sellers.push(Box::new(SellerFirstPrice {
                    seller_id,
                    seller_name,
                    num_impressions,
                }));
            }
        }
    }
}
