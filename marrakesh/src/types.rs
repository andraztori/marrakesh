/// Maximum number of campaigns supported (determines size of value_to_campaign_id array)
pub const MAX_CAMPAIGNS: usize = 10;

/// Represents the winner of an auction
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Winner {
    Campaign { 
        campaign_id: usize, 
        virtual_cost_cpm: f64,
        buyer_charge_cpm: f64,
    },
    OTHER_DEMAND,
    BELOW_FLOOR,
    NO_DEMAND,
}

/// Represents the result of an auction, subsuming the winner with cost information
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionResult {
    pub winner: Winner,
    pub true_cost_cpm: f64,
}

/// Charge type for impressions and sellers
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ChargeType {
    FIXED_COST { fixed_cost_cpm: f64 },
    FIRST_PRICE,
}

/// Campaign type determining the constraint model
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignType {
    FIXED_IMPRESSIONS { total_impressions_target: i32 },
    FIXED_BUDGET { total_budget_target: f64 },
}

/// Represents an impression on offer
#[derive(Debug, Clone)]
pub struct Impression {
    pub seller_id: usize,
    pub charge_type: ChargeType,
    pub best_other_bid_cpm: f64,
    pub floor_cpm: f64,
    pub result: AuctionResult,
    pub value_to_campaign_id: [f64; MAX_CAMPAIGNS],
}

impl Impression {
    /// Run an auction for this impression with the given campaigns
    /// Determines the winning campaign based on bids
    pub fn run_auction(&mut self, campaigns: &Campaigns) {
        // Get bids from all campaigns
        let mut winning_bid = 0.0;
        let mut winning_campaign_id: Option<usize> = None;

        for campaign in &campaigns.campaigns {
            let bid = campaign.get_bid(self);
            if bid > winning_bid {
                winning_bid = bid;
                winning_campaign_id = Some(campaign.campaign_id);
            }
        }

        // Helper function to get true_cost_cpm based on charge type
        // For fixed cost, always use fixed_cost_cpm; for first price, use provided value (or 0.0 if no winner)
        let get_true_cost = |value: f64| -> f64 {
            match self.charge_type {
                ChargeType::FIXED_COST { fixed_cost_cpm } => fixed_cost_cpm,
                ChargeType::FIRST_PRICE => value,
            }
        };

        // Determine the result based on winning bid
        let (winner, true_cost_cpm) = if let Some(campaign_id) = winning_campaign_id {
            if winning_bid < self.best_other_bid_cpm {
                // Winning bid is below best other bid - other demand wins
                (Winner::OTHER_DEMAND, get_true_cost(0.0))
            } else if winning_bid < self.floor_cpm {
                // Winning bid is below floor - no winner
                (Winner::BELOW_FLOOR, get_true_cost(0.0))
            } else {
                // Valid winner - set cost values
                // virtual_cost_cpm and buyer_charge_cpm are always the winning bid
                let true_cost = get_true_cost(winning_bid);
                let virtual_cost = winning_bid;
                let buyer_charge = winning_bid;
                
                (Winner::Campaign {
                    campaign_id,
                    virtual_cost_cpm: virtual_cost,
                    buyer_charge_cpm: buyer_charge,
                }, true_cost)
            }
        } else {
            // No campaigns participated
            (Winner::NO_DEMAND, get_true_cost(0.0))
        };

        self.result = AuctionResult {
            winner,
            true_cost_cpm,
        };
    }
}

/// Represents a campaign participating in auctions
#[derive(Debug, Clone)]
pub struct Campaign {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub campaign_rnd: u64,
    pub total_cost: f64,
    pub pacing: f64,
    pub campaign_type: CampaignType,
}

impl Campaign {
    /// Calculate the bid for this campaign given an impression
    /// Bid = campaign.pacing * impression.value_to_campaign_id[campaign.campaign_id]
    pub fn get_bid(&self, impression: &Impression) -> f64 {
        self.pacing * impression.value_to_campaign_id[self.campaign_id]
    }
}

/// Represents a seller offering impressions
#[derive(Debug, Clone)]
pub struct Seller {
    pub seller_id: usize,
    pub seller_name: String,
    pub charge_type: ChargeType,
    pub num_impressions: usize,
}

/// Parameters for adding a campaign
pub struct AddCampaignParams {
    pub campaign_name: String,
    pub campaign_rnd: u64,
    pub total_cost: f64,
    pub pacing: f64,
    pub campaign_type: CampaignType,
}

/// Container for campaigns with methods to add campaigns
pub struct Campaigns {
    pub campaigns: Vec<Campaign>,
}

impl Campaigns {
    pub fn new() -> Self {
        Self {
            campaigns: Vec::new(),
        }
    }

    pub fn add(&mut self, params: AddCampaignParams) -> Result<(), String> {
        if self.campaigns.len() >= MAX_CAMPAIGNS {
            return Err(format!(
                "Cannot add campaign: maximum number of campaigns ({}) exceeded. Current count: {}",
                MAX_CAMPAIGNS,
                self.campaigns.len()
            ));
        }
        let campaign_id = self.campaigns.len();
        self.campaigns.push(Campaign {
            campaign_id,
            campaign_name: params.campaign_name,
            campaign_rnd: params.campaign_rnd,
            total_cost: params.total_cost,
            pacing: params.pacing,
            campaign_type: params.campaign_type,
        });
        Ok(())
    }
}

/// Parameters for adding a seller
pub struct AddSellerParams {
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

    pub fn add(&mut self, params: AddSellerParams) {
        let seller_id = self.sellers.len();
        self.sellers.push(Seller {
            seller_id,
            seller_name: params.seller_name,
            charge_type: params.charge_type,
            num_impressions: params.num_impressions,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bid() {
        // Create a campaign with pacing = 0.5 and campaign_id = 2
        let campaign = Campaign {
            campaign_id: 2,
            campaign_name: "Test Campaign".to_string(),
            campaign_rnd: 12345,
            total_cost: 0.0,
            pacing: 0.5,
            campaign_type: CampaignType::FIXED_IMPRESSIONS {
                total_impressions_target: 1000,
            },
        };

        // Create an impression with value_to_campaign_id[2] = 20.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[2] = 20.0;

        let impression = Impression {
            seller_id: 0,
            charge_type: ChargeType::FIRST_PRICE,
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            result: AuctionResult {
                winner: Winner::BELOW_FLOOR,
                true_cost_cpm: 0.0,
            },
            value_to_campaign_id,
        };

        // Expected bid = 0.5 * 20.0 = 10.0
        let bid = campaign.get_bid(&impression);
        assert_eq!(bid, 10.0);
    }

    #[test]
    fn test_get_bid_with_different_campaign_id() {
        // Create a campaign with pacing = 1.0 and campaign_id = 0
        let campaign = Campaign {
            campaign_id: 0,
            campaign_name: "Test Campaign".to_string(),
            campaign_rnd: 67890,
            total_cost: 0.0,
            pacing: 1.0,
            campaign_type: CampaignType::FIXED_BUDGET {
                total_budget_target: 5000.0,
            },
        };

        // Create an impression with value_to_campaign_id[0] = 15.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[0] = 15.0;

        let impression = Impression {
            seller_id: 1,
            charge_type: ChargeType::FIXED_COST {
                fixed_cost_cpm: 10.0,
            },
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            result: AuctionResult {
                winner: Winner::BELOW_FLOOR,
                true_cost_cpm: 0.0,
            },
            value_to_campaign_id,
        };

        // Expected bid = 1.0 * 15.0 = 15.0
        let bid = campaign.get_bid(&impression);
        assert_eq!(bid, 15.0);
    }

    #[test]
    fn test_get_bid_with_zero_pacing() {
        // Create a campaign with pacing = 0.0
        let campaign = Campaign {
            campaign_id: 1,
            campaign_name: "Test Campaign".to_string(),
            campaign_rnd: 11111,
            total_cost: 0.0,
            pacing: 0.0,
            campaign_type: CampaignType::FIXED_IMPRESSIONS {
                total_impressions_target: 1000,
            },
        };

        // Create an impression with value_to_campaign_id[1] = 100.0
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        value_to_campaign_id[1] = 100.0;

        let impression = Impression {
            seller_id: 0,
            charge_type: ChargeType::FIRST_PRICE,
            best_other_bid_cpm: 0.0,
            floor_cpm: 0.0,
            result: AuctionResult {
                winner: Winner::BELOW_FLOOR,
                true_cost_cpm: 0.0,
            },
            value_to_campaign_id,
        };

        // Expected bid = 0.0 * 100.0 = 0.0
        let bid = campaign.get_bid(&impression);
        assert_eq!(bid, 0.0);
    }
}

