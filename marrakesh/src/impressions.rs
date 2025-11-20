use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::sellers::{Sellers, SellerTrait};
use crate::campaigns::{Campaigns, MAX_CAMPAIGNS};
use crate::converge::CampaignControllerStates;
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
    pub value_to_campaign_id: [f64; MAX_CAMPAIGNS],
    pub base_impression_value: f64,  // Store base value for logging
}

impl Impression {

    /// Run an auction for this impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the auction result
    pub fn run_auction(&self, campaigns: &Campaigns, campaign_controller_states: &CampaignControllerStates, seller: &dyn SellerTrait, seller_converge: &dyn crate::controllers::ControllerState, logger: &mut crate::logger::Logger) -> AuctionResult {
        // Get bids from all campaigns
        let mut winning_bid_cpm = 0.0;
        let mut winning_campaign_id: Option<usize> = None;
        let mut all_bids = if VERBOSE_AUCTION.load(Ordering::Relaxed) {
            Some(Vec::new())
        } else {
            None
        };

        // Get seller_boost_factor from seller convergence parameter
        let seller_converge_boost = seller_converge.as_any().downcast_ref::<crate::controllers::ControllerStateSingleVariable>().unwrap();
        let seller_boost_factor = seller_converge_boost.converging_variable;

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_controller_states.campaign_controller_states[campaign_id];
            // Use the trait method for get_bid
            if let Some(bid) = campaign.get_bid(self, campaign_converge.as_ref(), seller_boost_factor, logger) {
                // Check if bid is below zero
                if bid < 0.0 {
                    errln!(logger, LogEvent::Simulation, "Bid below zero: {:.4} from campaign_id: {}", bid, campaign_id);
                    panic!("Bid below zero: {:.4} from campaign_id: {}", bid, campaign_id);
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
            
            // Winning bid is below z - no winner
            if winning_bid_cpm < self.floor_cpm {
                break 'result (Winner::BELOW_FLOOR, seller.get_supply_cost_cpm(0.0) / 1000.0);
            }
            
            // Check competition if it exists
            if let Some(competition) = &self.competition {
                // Winning bid is below best other bid - other demand wins
                if winning_bid_cpm < competition.bid_cpm {
                    break 'result (Winner::OTHER_DEMAND, seller.get_supply_cost_cpm(0.0) / 1000.0);
                }
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
                Winner::OTHER_DEMAND => "OTHER_DEMAND".to_string(),
                Winner::BELOW_FLOOR => "BELOW_FLOOR".to_string(),
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
                // campaign value
                csv_fields.push(format!("{:.4}", self.value_to_campaign_id[campaign_id]));
                
                // campaign bid (empty if no bid)
                if let Some(bid) = bid_map.get(&campaign_id) {
                    csv_fields.push(format!("{:.4}", bid));
                } else {
                    csv_fields.push("".to_string());
                }
            }
            
            logln!(logger, LogEvent::Auction, "{}", csv_fields.join(","));
        }

        // Format first campaign value
        /*if all_bids.len() > 0 {
            let values_str = format!("{:.2} {:.4}, {:.4}", self.competition.as_ref().unwrap().win_rate_actual_sigmoid_offset,self.value_to_campaign_id[0], all_bids[0].1);
            logln!(logger, LogEvent::Auction, "{}", values_str);
        }*/
        AuctionResult {
            winner,
            supply_cost,
        }
    }
}

/// Container for impressions with methods to create impressions
pub struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    /// Create a new Impressions container and populate it from sellers
    pub fn new(sellers: &Sellers, params: &ImpressionsParam) -> Self {
        // Use deterministic seed for reproducible results
        let mut rng_base_value = StdRng::seed_from_u64(get_seed(1991));
        let mut rng_competition = StdRng::seed_from_u64(get_seed(2992));
        let mut rng_floor = StdRng::seed_from_u64(get_seed(3993));
        let mut rng_campaigns_multiplier = StdRng::seed_from_u64(get_seed(4994));

        let mut impressions = Vec::new();

        for seller in &sellers.sellers {
            for _ in 0..seller.get_impressions_on_offer() {
                // First calculate base impression value (needed for floor generation)
                let base_impression_value = params.base_impression_value_dist.sample(&mut rng_base_value);
                let (competition, floor_cpm) = seller.generate_impression(
                    base_impression_value,
                    &mut rng_competition,
                    &mut rng_floor,
                );
                //println!("Base impression value: {:.4}", base_impression_value);
                // Then generate values for each campaign by multiplying base value with campaign-specific multiplier
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                //println!("Base impression value: {}", base_impression_value);
                for i in 0..MAX_CAMPAIGNS {
                    let multiplier = params.value_to_campaign_multiplier_dist.sample(&mut rng_campaigns_multiplier);
                    //multiplier = 1.0;
                //    println!("Campaign {} multiplier: {:.4}", i, multiplier);
                    value_to_campaign_id[i] = base_impression_value * multiplier;
//                    println!("Campaign {} value: {:.4}", i, value_to_campaign_id[i]);
                //    println!("     {}", value_to_campaign_id[i])
                }

                impressions.push(Impression {
                    seller_id: seller.seller_id(),
                    competition,
                    floor_cpm,
                    value_to_campaign_id,
                    base_impression_value,
                });
            }
        }

        Self { impressions }
    }
}

