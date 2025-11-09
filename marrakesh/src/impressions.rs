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
            for _ in 0..seller.num_impressions {
                let (best_other_bid_cpm, floor_cpm) = match seller.charge_type {
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

