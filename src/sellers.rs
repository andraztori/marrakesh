use crate::competition::CompetitionGeneratorTrait;
use crate::floors::FloorGeneratorTrait;
use crate::controllers::ControllerTrait;
pub use crate::seller::SellerTrait;
pub use crate::seller::SellerGeneral;
pub use crate::seller_targets::SellerTargetTrait;

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
pub use crate::seller_targets::{SellerTargetNone, SellerTargetTotalCost};
// Re-export charger types for convenience
pub use crate::seller_chargers::{SellerCharger, SellerChargerFirstPrice, SellerChargerFixedPrice};

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
        let (converge_target, converge_controller): (Box<dyn SellerTargetTrait>, Box<dyn ControllerTrait>) = match seller_converge {
            SellerConvergeStrategy::NONE { default_value } => {
                (
                    Box::new(SellerTargetNone),
                    Box::new(crate::controllers::ControllerConstant::new(default_value))
                )
            }
            SellerConvergeStrategy::TOTAL_COST { target_total_cost } => {
                (
                    Box::new(SellerTargetTotalCost {
                        target_cost: target_total_cost,
                    }),
                    Box::new(crate::controllers::ControllerProportionalDerivative::new())
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
