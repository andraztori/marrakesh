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

/// Marketplace containing campaigns, sellers, and impressions
/// This groups together the three main components of the marketplace simulation
pub struct Marketplace {
    pub campaigns: crate::campaigns::Campaigns,
    pub sellers: crate::sellers::Sellers,
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


