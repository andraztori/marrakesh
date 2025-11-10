use crate::logger::{Logger, LogEvent};
use crate::logln;

/// Represents the winner of an auction
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Winner {
    Campaign { 
        campaign_id: usize, 
        virtual_cost: f64,
        buyer_charge: f64,
    },
    OTHER_DEMAND,
    BELOW_FLOOR,
    NO_DEMAND,
}

/// Represents the result of an auction, subsuming the winner with cost information
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionResult {
    pub winner: Winner,
    pub supply_cost: f64,
}

/// Charge type for impressions and sellers
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ChargeType {
    FIXED_COST { fixed_cost_cpm: f64 },
    FIRST_PRICE,
}

/// Represents a seller offering impressions
#[derive(Debug, Clone)]
pub struct Seller {
    pub seller_id: usize,
    pub seller_name: String,
    pub charge_type: ChargeType,
    pub num_impressions: usize,
}

/// Container for sellers with methods to add sellers
pub struct Sellers {
    pub sellers: Vec<Seller>,
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
        self.sellers.push(Seller {
            seller_id,
            seller_name,
            charge_type,
            num_impressions,
        });
    }
}

/// Marketplace containing campaigns, sellers, and impressions
/// This groups together the three main components of the marketplace simulation
pub struct Marketplace {
    pub campaigns: crate::campaigns::Campaigns,
    pub sellers: Sellers,
    pub impressions: crate::impressions::Impressions,
}

impl Marketplace {
    /// Print initialization information about the marketplace
    pub fn printout(&self, logger: &mut Logger) {
        
        logln!(logger, LogEvent::Simulation, "Initialized {} sellers", self.sellers.sellers.len());
        logln!(logger, LogEvent::Simulation, "Initialized {} campaigns", self.campaigns.campaigns.len());
        logln!(logger, LogEvent::Simulation, "Initialized {} impressions", self.impressions.impressions.len());
    }
}


