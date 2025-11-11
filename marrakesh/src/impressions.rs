use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::types::{ChargeType, Winner, AuctionResult};
use crate::sellers::{Sellers, SellerTrait};
use crate::campaigns::{Campaigns, MAX_CAMPAIGNS};
use crate::simulationrun::{CampaignConvergeParams, SellerParam};

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
    pub best_other_bid_dist: Box<dyn DistributionF64>,
    pub floor_cpm_dist: Box<dyn DistributionF64>,
    pub base_impression_value_dist: Box<dyn DistributionF64>,
    pub value_to_campaign_multiplier_dist: Box<dyn DistributionF64>,
    pub fixed_cost_floor_cpm: f64,
}

impl ImpressionsParam {
    /// Create a new ImpressionsParam with Distribution<f64> types
    /// The distributions will be boxed internally
    pub fn new<D1, D2, D3, D4>(
        best_other_bid_dist: D1,
        floor_cpm_dist: D2,
        base_impression_value_dist: D3,
        value_to_campaign_multiplier_dist: D4,
        fixed_cost_floor_cpm: f64,
    ) -> Self
    where
        D1: Distribution<f64> + 'static,
        D2: Distribution<f64> + 'static,
        D3: Distribution<f64> + 'static,
        D4: Distribution<f64> + 'static,
    {
        Self {
            best_other_bid_dist: Box::new(best_other_bid_dist),
            floor_cpm_dist: Box::new(floor_cpm_dist),
            base_impression_value_dist: Box::new(base_impression_value_dist),
            value_to_campaign_multiplier_dist: Box::new(value_to_campaign_multiplier_dist),
            fixed_cost_floor_cpm,
        }
    }
}

/// Represents an impression on offer
#[derive(Debug, Clone)]
pub struct Impression {
    pub seller_id: usize,
    pub charge_type: ChargeType,
    pub best_other_bid_cpm: f64,
    pub floor_cpm: f64,
    pub value_to_campaign_id: [f64; MAX_CAMPAIGNS],
}

impl Impression {
    /// Run an auction for this impression with the given campaigns, campaign parameters, seller, and seller parameters
    /// Returns the auction result
    pub fn run_auction(&self, campaigns: &Campaigns, campaign_params: &CampaignConvergeParams, seller: &dyn SellerTrait, seller_param: &SellerParam) -> AuctionResult {
        // Get bids from all campaigns
        let mut winning_bid_cpm = 0.0;
        let mut winning_campaign_id: Option<usize> = None;

        for campaign in &campaigns.campaigns {
            if let Some(campaign_param) = campaign_params.params.get(campaign.campaign_id()) {
                // Use the trait method for get_bid
                let bid = campaign.get_bid(self, campaign_param.as_ref());
                if bid > winning_bid_cpm {
                    winning_bid_cpm = bid;
                    winning_campaign_id = Some(campaign.campaign_id());
                }
            }
        }

        // Apply boost_factor to winning_bid_cpm
        winning_bid_cpm *= seller_param.boost_factor;

        // Determine the result based on winning bid
        let (winner, supply_cost) = if let Some(campaign_id) = winning_campaign_id {
            if winning_bid_cpm < self.best_other_bid_cpm {
                // Winning bid is below best other bid - other demand wins
                (Winner::OTHER_DEMAND, seller.get_supply_cost_cpm(0.0) / 1000.0)
            } else if winning_bid_cpm < self.floor_cpm {
                // Winning bid is below floor - no winner
                (Winner::BELOW_FLOOR, seller.get_supply_cost_cpm(0.0) / 1000.0)
            } else {
                // Valid winner - set cost values
                // virtual_cost and buyer_charge are always the winning bid
                let supply_cost = seller.get_supply_cost_cpm(winning_bid_cpm) / 1000.0;
                let virtual_cost = winning_bid_cpm / 1000.0;
                let buyer_charge = winning_bid_cpm / 1000.0;
                
                // Convert from CPM to actual cost by dividing by 1000
                (Winner::Campaign {
                    campaign_id,
                    virtual_cost: virtual_cost,
                    buyer_charge: buyer_charge,
                }, supply_cost )
            }
        } else {
            // No campaigns participated
            (Winner::NO_DEMAND, seller.get_supply_cost_cpm(0.0) / 1000.0)
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
            for _ in 0..seller.num_impressions() {
                let (best_other_bid_cpm, floor_cpm) = match seller.charge_type() {
                    ChargeType::FIRST_PRICE => {
                        (
                            params.best_other_bid_dist.sample(&mut rng),
                            params.floor_cpm_dist.sample(&mut rng),
                        )
                    }
                    ChargeType::FIXED_COST { .. } => (0.0, params.fixed_cost_floor_cpm),
                };

                // First calculate base impression value
                let base_impression_value = params.base_impression_value_dist.sample(&mut rng);
                //println!("Base impression value: {:.4}", base_impression_value);
                // Then generate values for each campaign by multiplying base value with campaign-specific multiplier
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                for i in 0..MAX_CAMPAIGNS {
                    let multiplier = params.value_to_campaign_multiplier_dist.sample(&mut rng);
                //    println!("Campaign {} multiplier: {:.4}", i, multiplier);
                    value_to_campaign_id[i] = base_impression_value * multiplier;
                }

                impressions.push(Impression {
                    seller_id: seller.seller_id(),
                    charge_type: seller.charge_type(),
                    best_other_bid_cpm,
                    floor_cpm,
                    value_to_campaign_id,
                });
            }
        }

        Self { impressions }
    }
}

