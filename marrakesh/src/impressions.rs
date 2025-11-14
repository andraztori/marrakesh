use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::sellers::{Sellers, SellerTrait};
use crate::campaigns::{Campaigns, MAX_CAMPAIGNS};
use crate::simulationrun::CampaignConverges;
use crate::competition::ImpressionCompetition;

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
}

impl Impression {

    /// Run an auction for this impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the auction result
    pub fn run_auction(&self, campaigns: &Campaigns, campaign_converges: &CampaignConverges, seller: &dyn SellerTrait, seller_converge: &dyn crate::converge::ConvergingVariables, logger: &mut crate::logger::Logger) -> AuctionResult {
        // Get bids from all campaigns
        let mut winning_bid_cpm = 0.0;
        let mut winning_campaign_id: Option<usize> = None;

        // Get seller_boost_factor from seller convergence parameter
        let seller_converge_boost = seller_converge.as_any().downcast_ref::<crate::converge::ConvergingSingleVariable>().unwrap();
        let seller_boost_factor = seller_converge_boost.converging_variable;

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_converges.campaign_converges[campaign_id];
            // Use the trait method for get_bid
            if let Some(bid) = campaign.get_bid(self, campaign_converge.as_ref(), seller_boost_factor, logger) {
                if bid > winning_bid_cpm {
                    winning_bid_cpm = bid;
                    winning_campaign_id = Some(campaign_id);
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
            
            // Winning bid is below floor - no winner
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
        let mut rng = StdRng::seed_from_u64(999);

        let mut impressions = Vec::new();

        for seller in &sellers.sellers {
            for _ in 0..seller.get_impressions_on_offer() {
                // First calculate base impression value (needed for floor generation)
                let base_impression_value = params.base_impression_value_dist.sample(&mut rng);
                
                let (competition, floor_cpm) = seller.generate_impression(
                    base_impression_value,
                    &mut rng,
                );
                //println!("Base impression value: {:.4}", base_impression_value);
                // Then generate values for each campaign by multiplying base value with campaign-specific multiplier
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                //println!("Base impression value: {}", base_impression_value);
                for i in 0..MAX_CAMPAIGNS {
                    let multiplier = params.value_to_campaign_multiplier_dist.sample(&mut rng);
                //    println!("Campaign {} multiplier: {:.4}", i, multiplier);
                    value_to_campaign_id[i] = base_impression_value * multiplier;
                //    println!("     {}", value_to_campaign_id[i])
                }

                impressions.push(Impression {
                    seller_id: seller.seller_id(),
                    competition,
                    floor_cpm,
                    value_to_campaign_id,
                });
            }
        }

        Self { impressions }
    }
}

