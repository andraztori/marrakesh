use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::types::{ChargeType, Impression, Sellers, MAX_CAMPAIGNS};

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

/// Trait for providing distribution parameters for impression generation
/// All methods return boxed types that implement Distribution<f64>
pub trait ImpressionsParam {
    fn best_other_bid_dist(&self) -> Box<dyn DistributionF64>;
    fn floor_cpm_dist(&self) -> Box<dyn DistributionF64>;
    fn base_impression_value_dist(&self) -> Box<dyn DistributionF64>;
    fn value_to_campaign_multiplier_dist(&self) -> Box<dyn DistributionF64>;
    /// Returns the floor_cpm value for FIXED_COST impressions
    fn fixed_cost_floor_cpm(&self) -> f64;
}

/// Container for impressions with methods to create impressions
pub struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    /// Create a new Impressions container and populate it from sellers
    pub fn new(sellers: &Sellers, params: &dyn ImpressionsParam) -> Self {
        // Use deterministic seed for reproducible results
        let mut rng = StdRng::seed_from_u64(999);
        
        let best_other_bid_dist = params.best_other_bid_dist();
        let floor_cpm_dist = params.floor_cpm_dist();
        let base_impression_value_dist = params.base_impression_value_dist();
        let value_to_campaign_multiplier_dist = params.value_to_campaign_multiplier_dist();

        let mut impressions = Vec::new();

        for seller in &sellers.sellers {
            for _ in 0..seller.num_impressions {
                let (best_other_bid_cpm, floor_cpm) = match seller.charge_type {
                    ChargeType::FIRST_PRICE => {
                        (
                            best_other_bid_dist.sample(&mut rng),
                            floor_cpm_dist.sample(&mut rng),
                        )
                    }
                    ChargeType::FIXED_COST { .. } => (0.0, params.fixed_cost_floor_cpm()),
                };

                // First calculate base impression value
                let base_impression_value = base_impression_value_dist.sample(&mut rng);
                //println!("Base impression value: {:.4}", base_impression_value);
                // Then generate values for each campaign by multiplying base value with campaign-specific multiplier
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                for i in 0..MAX_CAMPAIGNS {
                    let multiplier = value_to_campaign_multiplier_dist.sample(&mut rng);
                //    println!("Campaign {} multiplier: {:.4}", i, multiplier);
                    value_to_campaign_id[i] = base_impression_value * multiplier;
                }

                impressions.push(Impression {
                    seller_id: seller.seller_id,
                    charge_type: seller.charge_type.clone(),
                    best_other_bid_cpm,
                    floor_cpm,
                    value_to_campaign_id,
                });
            }
        }

        Self { impressions }
    }
}

