use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::sellers::Sellers;
use crate::seller::SellerTrait;
use crate::campaigns::Campaigns;
use crate::competition::ImpressionCompetition;
use crate::logger::LogEvent;
use crate::errln;
use crate::logln;
use crate::utils::get_seed;
use crate::utils::VERBOSE_AUCTION;
use std::sync::atomic::Ordering;

/// Represents the winner of an auction
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Winner {
    Campaign { 
        campaign_id: usize, 
        virtual_cost: f64,
        buyer_charge: f64,
    },
    LOST,
    NO_DEMAND,
}

/// Represents a fractional winner in a fractional auction
#[derive(Debug, Clone, PartialEq)]
pub struct FractionalWinner {
    pub campaign_id: usize,
    pub virtual_cost: f64,
    pub buyer_charge: f64,
    pub win_fraction: f64,
    pub bid_cpm: f64,
    pub supply_cost: f64,
}

/// Represents the winners of a fractional auction (can have multiple campaigns winning fractions)
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum FractionalWinners {
    Campaigns {
        winners: Vec<FractionalWinner>,
    },
    LOST,
    NO_DEMAND,
}

/// Represents the result of an auction, subsuming the winner with cost information
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionResult {
    pub winner: Winner,
    pub supply_cost: f64,
}

/// Represents the result of a fractional auction, subsuming the winners with cost information
#[derive(Debug, Clone, PartialEq)]
pub struct FractionalAuctionResult {
    pub winner: FractionalWinners,
}

/// Object-safe wrapper for Distribution<f64> that works with StdRng
/// This is needed because Distribution<f64> cannot be made into a trait object
/// due to its generic sample method
pub trait DistributionF64 {
    fn sample(&self, rng: &mut StdRng) -> f64;
}

impl<D: Distribution<f64>> DistributionF64 for D {
    fn sample(&self, rng: &mut StdRng) -> f64 {
        Distribution::sample(self, rng)
    }
}

/// Struct for providing distribution parameters for impression generation
/// Contains pre-initialized distribution boxes
pub struct ImpressionsParam {
    pub base_impression_value_dist: Box<dyn DistributionF64>,
    pub value_to_campaign_multiplier_dist: Box<dyn DistributionF64>,
}

impl ImpressionsParam {
    /// Create a new ImpressionsParam with Distribution<f64> types
    /// The distributions will be boxed internally
    pub fn new<D1, D2>(
        base_impression_value_dist: D1,
        value_to_campaign_multiplier_dist: D2,
    ) -> Self
    where
        D1: Distribution<f64> + 'static,
        D2: Distribution<f64> + 'static,
    {
        Self {
            base_impression_value_dist: Box::new(base_impression_value_dist),
            value_to_campaign_multiplier_dist: Box::new(value_to_campaign_multiplier_dist),
        }
    }
}


/// Represents an impression on offer
#[derive(Debug, Clone)]
pub struct Impression {
    pub seller_id: usize,
    pub competition: Option<ImpressionCompetition>,
    pub floor_cpm: f64,
    pub value_to_campaign_group: Vec<f64>,
    pub base_impression_value: f64,  // Store base value for logging
}

impl Impression {

    /// Run an auction for this impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the auction result
    pub fn run_auction(&self, campaigns: &Campaigns, campaign_converges: &[Vec<&dyn crate::controllers::ControllerState>], seller: &dyn SellerTrait, seller_converge: &dyn crate::controllers::ControllerState, logger: &mut crate::logger::Logger) -> AuctionResult {
        // Get bids from all campaigns
        let mut winning_bid_cpm = 0.0;
        let mut winning_campaign_id: Option<usize> = None;
        let mut all_bids = if VERBOSE_AUCTION.load(Ordering::Relaxed) {
            Some(Vec::new())
        } else {
            None
        };

        // Get seller_control_factor from seller using get_control_variable
        let seller_control_factor = seller.get_control_variable(seller_converge);

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_converges[campaign_id];
            // Resolve value_to_campaign at call site using campaign's group ID
            let group_id = campaigns.campaign_to_value_group_mapping[campaign_id];
            let value_to_campaign = self.value_to_campaign_group[group_id];
            // Use the trait method for get_bid
            if let Some(bid) = campaign.get_bid(self, &campaign_converge, seller_control_factor, value_to_campaign, logger) {
                // Check if bid is below zero - skip negative bids
                if bid < 0.0 {
                    errln!(logger, LogEvent::Simulation, "Bid below zero: {:.4} from campaign_id: {}, skipping", bid, campaign_id);
                    continue;
                }
                if let Some(bids) = &mut all_bids {
                    bids.push((campaign_id, bid));
                }
                if bid > winning_bid_cpm {
                    winning_bid_cpm = bid;
                    winning_campaign_id = Some(campaign_id);
                    //println!("Winning bid: {:.4}, campaign_id: {}", bid, campaign_id);
                }
            }
            // If get_bid returns None, skip this campaign (warning already logged)
        }

        // Determine the result based on winning bid
        // Check all failure conditions first, then create winner in one place
        let (winner, supply_cost) = 'result: {
            // No campaigns participated
            let campaign_id = match winning_campaign_id {
                Some(id) => id,
                None => break 'result (Winner::NO_DEMAND, seller.get_supply_cost_cpm(0.0) / 1000.0),
            };
            
            // Winning bid is below z or below competition - no winner (LOST)
            let minimum_cpm_to_win = if let Some(competition) = &self.competition {
                self.floor_cpm.max(competition.bid_cpm)
            } else {
                self.floor_cpm
            };
            
            if winning_bid_cpm < minimum_cpm_to_win {
                break 'result (Winner::LOST, seller.get_supply_cost_cpm(0.0) / 1000.0);
            }
            
            // Valid winner - bid passes all checks (floor and competition if present)
            // Set cost values - virtual_cost and buyer_charge are always the winning bid
            let supply_cost = seller.get_supply_cost_cpm(winning_bid_cpm) / 1000.0;
            let virtual_cost = winning_bid_cpm / 1000.0;
            let buyer_charge = winning_bid_cpm / 1000.0;
            
            // Convert from CPM to actual cost by dividing by 1000
            (Winner::Campaign {
                campaign_id,
                virtual_cost,
                buyer_charge,
            }, supply_cost)
        };

        // Log auction data in CSV format
        if VERBOSE_AUCTION.load(Ordering::Relaxed) {
            let all_bids = all_bids.as_ref().unwrap();
            
            // Build CSV row
            let mut csv_fields = Vec::new();
            
            // seller_id
            csv_fields.push(format!("{}", self.seller_id));
            
            // demand_id (winner identifier)
            let demand_id = match &winner {
                Winner::Campaign { campaign_id, .. } => format!("{}", campaign_id),
                Winner::LOST => "LOST".to_string(),
                Winner::NO_DEMAND => "NO_DEMAND".to_string(),
            };
            csv_fields.push(demand_id);
            
            // winning_bid
            csv_fields.push(format!("{:.4}", winning_bid_cpm));
            
            // floor_cpm
            csv_fields.push(format!("{:.4}", self.floor_cpm));
            
            // impression_base_value
            csv_fields.push(format!("{:.4}", self.base_impression_value));
            
            // competing_bid, competing_offset, competing_scale
            if let Some(comp) = &self.competition {
                csv_fields.push(format!("{:.4}", comp.bid_cpm));
                csv_fields.push(format!("{:.4}", comp.win_rate_actual_sigmoid_offset));
                csv_fields.push(format!("{:.4}", comp.win_rate_actual_sigmoid_scale));
            } else {
                csv_fields.push("".to_string());
                csv_fields.push("".to_string());
                csv_fields.push("".to_string());
            }
            
            // For each campaign: value and bid
            // Create a map of campaign_id to bid for quick lookup
            let bid_map: std::collections::HashMap<usize, f64> = all_bids.iter().cloned().collect();
            
            for campaign_id in 0..campaigns.campaigns.len() {
                // campaign value - get from campaign's group
                let group_index = campaigns.campaign_to_value_group_mapping[campaign_id];
                csv_fields.push(format!("{:.4}", self.value_to_campaign_group[group_index]));
                
                // campaign bid (empty if no bid)
                if let Some(bid) = bid_map.get(&campaign_id) {
                    csv_fields.push(format!("{:.4}", bid));
                } else {
                    csv_fields.push("".to_string());
                }
            }
            
            logln!(logger, LogEvent::Auction, "{}", csv_fields.join(","));
        }

        AuctionResult {
            winner,
            supply_cost,
        }
    }

    /// Run a fractional auction for this impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the fractional auction result
    /// 
    /// `softmax_temperature`: Temperature parameter for softmax calculation
    /// - Lower values (< 1.0) make the distribution sharper (more concentrated on highest bid)
    /// - Higher values (> 1.0) make the distribution smoother (more uniform)
    /// - Default: 1.0 (standard softmax)
    pub fn run_fractional_auction(&self, campaigns: &Campaigns, campaign_converges: &[Vec<&dyn crate::controllers::ControllerState>], seller: &dyn SellerTrait, seller_converge: &dyn crate::controllers::ControllerState, softmax_temperature: f64, logger: &mut crate::logger::Logger) -> FractionalAuctionResult {
        // Calculate minimum CPM needed to win this impression
        // Must be at least the floor, and if competition exists, must beat the competing bid
        let minimum_cpm_to_win = if let Some(competition) = &self.competition {
            self.floor_cpm.max(competition.bid_cpm)
        } else {
            self.floor_cpm
        };

        // Collect all campaigns with bids above minimum_cpm_to_win
        let mut fractional_winners: Vec<FractionalWinner> = Vec::new();
        let mut any_bids_made = false;

        // Get seller_control_factor from seller using get_control_variable
        let seller_control_factor = seller.get_control_variable(seller_converge);

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_converges[campaign_id];
            // Resolve value_to_campaign at call site using campaign's group ID
            let group_id = campaigns.campaign_to_value_group_mapping[campaign_id];
            let value_to_campaign = self.value_to_campaign_group[group_id];
            // Use the trait method for get_bid
            if let Some(bid) = campaign.get_bid(self, &campaign_converge, seller_control_factor, value_to_campaign, logger) {
                any_bids_made = true;
                // Check if bid is below zero - skip negative bids
                if bid < 0.0 {
                    errln!(logger, LogEvent::Simulation, "Bid below zero: {:.4} from campaign_id: {}, skipping", bid, campaign_id);
                    continue;
                }
                
                // If bid is above minimum_cpm_to_win, add to winners list
                if bid >= minimum_cpm_to_win {
                    let virtual_cost = bid / 1000.0;
                    let buyer_charge = bid / 1000.0;
                    let supply_cost = seller.get_supply_cost_cpm(bid) / 1000.0;
                    fractional_winners.push(FractionalWinner {
                        campaign_id,
                        virtual_cost,
                        buyer_charge,
                        win_fraction: 1.0,
                        bid_cpm: bid,
                        supply_cost,
                    });
                }
            }
            // If get_bid returns None, skip this campaign (warning already logged)
        }

        // Calculate win_fraction using softmax based on bid_cpm with temperature
        // Temperature controls the sharpness: lower = sharper (more concentrated on highest bid), higher = smoother (more uniform)
        if !fractional_winners.is_empty() {
            // Find maximum bid for numerical stability (log-sum-exp trick)
            let max_bid = fractional_winners.iter()
                .map(|w| w.bid_cpm)
                .fold(f64::NEG_INFINITY, f64::max);
            
            // Calculate exp((bid_cpm - max_bid) / temperature) for each winner
            let exp_values: Vec<f64> = fractional_winners.iter()
                .map(|w| ((w.bid_cpm - max_bid) / softmax_temperature).exp())
                .collect();
            
            // Calculate sum of exp values
            let sum_exp: f64 = exp_values.iter().sum();
            
            // Update win_fraction for each winner using softmax
            for (winner, exp_val) in fractional_winners.iter_mut().zip(exp_values.iter()) {
                winner.win_fraction = exp_val / sum_exp;
            }
        }

        // Determine the result based on collected winners
        // Check all failure conditions first, then create winner in one place
        let winner = if fractional_winners.is_empty() {
            // Distinguish between no bids (NO_DEMAND) and bids below threshold (LOST)
            if any_bids_made {
                FractionalWinners::LOST
            } else {
                FractionalWinners::NO_DEMAND
            }
        } else {
            // Valid winners - all passed the minimum_cpm_to_win threshold
            FractionalWinners::Campaigns {
                winners: fractional_winners,
            }
        };

        FractionalAuctionResult {
            winner,
        }
    }
}

/// Container for impressions with methods to create impressions
pub struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    /// Create a new Impressions container and populate it from sellers
    /// Note: campaign groups must be finalized before calling this function
    pub fn new(sellers: &Sellers, params: &ImpressionsParam, campaigns: &Campaigns) -> Self {
        // Calculate total number of impressions ahead of time
        let total_impressions: usize = sellers.sellers.iter()
            .map(|seller| seller.get_impressions_on_offer())
            .sum();
        
        // Get number of campaign groups directly from value_groups length
        let num_campaign_groups = campaigns.value_groups.len();
        
        // Check that campaigns have been finalized
        if num_campaign_groups == 0 {
            panic!("Campaigns have to be finalized before calling impressions::new()");
        }
        
        // Pre-allocate impressions vector with calculated capacity
        let mut impressions = Vec::with_capacity(total_impressions);
        
        // Use deterministic seed for reproducible results
        let mut rng_base_value = StdRng::seed_from_u64(get_seed(1991));
        let mut rng_competition = StdRng::seed_from_u64(get_seed(2992));
        let mut rng_floor = StdRng::seed_from_u64(get_seed(3993));
        let mut rng_campaigns_multiplier = StdRng::seed_from_u64(get_seed(4994));
        for seller in &sellers.sellers {
            for _ in 0..seller.get_impressions_on_offer() {
                // First calculate base impression value (needed for floor generation)
                let base_impression_value = params.base_impression_value_dist.sample(&mut rng_base_value);
               // println!("base_impression_value: {:.4}", base_impression_value);
                let (competition, floor_cpm) = seller.generate_impression(
                    base_impression_value,
                    &mut rng_competition,
                    &mut rng_floor,
                );

                // Generate values for each campaign group by multiplying base value with campaign-specific multiplier
                let mut value_to_campaign_group = Vec::with_capacity(num_campaign_groups);

                for _ in 0..num_campaign_groups {
                    let multiplier = params.value_to_campaign_multiplier_dist.sample(&mut rng_campaigns_multiplier);
//                    println!("multiplier: {:.4}", multiplier);
                    // println!("base_impression_value: {:.4}", base_impression_value)
                    //let multiplier = 1.0;
                    let value = base_impression_value * multiplier;
                    value_to_campaign_group.push(value);
                }
                impressions.push(Impression {
                    seller_id: seller.seller_id(),
                    competition,
                    floor_cpm,
                    value_to_campaign_group,
                    base_impression_value,
                });
            }
        }

        Self { 
            impressions,
        }
    }
}

