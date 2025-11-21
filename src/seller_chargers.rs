// These are chargers that can be used by SellerGeneral. In theory we could give them more flexibility, but 
// vast majority of strategies require just one parameter (buyer_win_cpm or fixed_cost_cpm), so if one needs more complex state
// one can implement a full SellerTrait.
//
// Note: If we want to have second price auctions at some point, we would rename this to SellerAuction 
// and move the Impression::run_auction code under seller, so that the auction mechanism (first price, second price, etc.)
// would be determined by the seller's auction type rather than being hardcoded in the impression.

/// Trait for seller charging strategies
pub trait SellerCharger {
    /// Get the supply cost in CPM for a given buyer winning bid CPM
    /// For fixed cost sellers, returns the fixed_cost_cpm
    /// For first price sellers, returns the buyer_win_cpm
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64;
    
    /// Get a string representation of the charging type
    fn get_charging_type(&self) -> String;
}

/// Charger for first price auction
pub struct SellerChargerFirstPrice;

impl SellerCharger for SellerChargerFirstPrice {
    fn get_supply_cost_cpm(&self, buyer_win_cpm: f64) -> f64 {
        buyer_win_cpm
    }
    
    fn get_charging_type(&self) -> String {
        "First price".to_string()
    }
}

/// Charger for fixed price (cost per mille)
pub struct SellerChargerFixedPrice {
    pub fixed_cost_cpm: f64,
}

impl SellerCharger for SellerChargerFixedPrice {
    fn get_supply_cost_cpm(&self, _buyer_win_cpm: f64) -> f64 {
        self.fixed_cost_cpm
    }
    
    fn get_charging_type(&self) -> String {
        format!("Fixed price CPM: {:.2}", self.fixed_cost_cpm)
    }
}

