mod types;
mod simulationrun;
mod converge;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, LogNormal};
use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Impression, Campaigns, Sellers, MAX_CAMPAIGNS};
use simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use converge::SimulationConverge;

/// Container for impressions with methods to create impressions
pub struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    /// Convert mean and standard deviation to log-normal distribution parameters
    /// Returns (μ, σ) for LogNormal(μ, σ) that approximates the given mean and stddev
    /// 
    /// For LogNormal(μ, σ):
    /// - E[X] = exp(μ + σ²/2)
    /// - Var[X] = (exp(σ²) - 1) * exp(2μ + σ²)
    /// 
    /// To convert from mean (m) and stddev (s):
    /// - σ = sqrt(ln(1 + s²/m²))
    /// - μ = ln(m) - σ²/2
    fn lognormal_from_mean_stddev(mean: f64, stddev: f64) -> (f64, f64) {
        let variance = stddev * stddev;
        let sigma_squared = (1.0 + variance / (mean * mean)).ln();
        let sigma = sigma_squared.sqrt();
        let mu = mean.ln() - sigma_squared / 2.0;
        (mu, sigma)
    }

    /// Create a log-normal distribution from mean and standard deviation
    /// This is a convenience wrapper that converts mean/stddev to log-normal parameters
    fn create_lognormal(mean: f64, stddev: f64) -> LogNormal<f64> {
        let (mu, sigma) = Self::lognormal_from_mean_stddev(mean, stddev);
        LogNormal::new(mu, sigma).unwrap()
    }

    /// Create a new Impressions container and populate it from sellers
    pub fn new(sellers: &Sellers) -> Self {
        // Use deterministic seed for reproducible results
        let mut rng = StdRng::seed_from_u64(999);
        
        // Log-normal distributions for best_other_bid and floor_cpm (mean=10.0, stddev=3.0)
        let best_other_bid_dist = Self::create_lognormal(10.0, 3.0);
        let floor_cpm_dist = Self::create_lognormal(10.0, 3.0);
        
        // Log-normal distribution for base impression value (mean=5.0, stddev=3.0)
        let base_impression_value_dist = Self::create_lognormal(5.0, 3.0);
        
        // Log-normal distribution for multiplier (mean=1.0, stddev=0.2)
        let value_to_campaign_multiplier_dist = Self::create_lognormal(1.0, 0.2);

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
                    ChargeType::FIXED_COST { .. } => (0.0, 0.0),
                };

                // First calculate base impression value
                let base_impression_value = base_impression_value_dist.sample(&mut rng);
                
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

fn main() {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 0".to_string(),
        campaign_rnd: 12345,
        campaign_type: CampaignType::FIXED_IMPRESSIONS {
            total_impressions_target: 1000,
        },
    }).expect("Failed to add campaign");

    campaigns.add(AddCampaignParams {
        campaign_name: "Campaign 1".to_string(),
        campaign_rnd: 67890,
        campaign_type: CampaignType::FIXED_BUDGET {
            total_budget_target: 20.0,
        },
    }).expect("Failed to add campaign");


    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(AddSellerParams {
        seller_name: "MRG".to_string(),
        charge_type: ChargeType::FIXED_COST {
            fixed_cost_cpm: 10.0,
        },
        num_impressions: 1000,
    });

    sellers.add(AddSellerParams {
        seller_name: "HB".to_string(),
        charge_type: ChargeType::FIRST_PRICE,
        num_impressions: 10000,
    });

    // Create impressions for all sellers
    let impressions = Impressions::new(&sellers);

    println!("Initialized {} sellers", sellers.sellers.len());
    println!("Initialized {} campaigns", campaigns.campaigns.len());
    println!("Initialized {} impressions", impressions.impressions.len());

    // Create campaign parameters from campaigns (default pacing = 1.0)
    let mut campaign_params = CampaignParams::new(&campaigns);
    
    // Run simulation loop with pacing adjustments (maximum 100 iterations)
    // verbosity = false means only print convergence message and final solution
    SimulationConverge::run(&impressions, &campaigns, &sellers, &mut campaign_params, 100, false);
    
    // Run final simulation and output complete statistics
    let final_simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params);
    let final_stats = SimulationStat::new(&campaigns, &sellers, &impressions, &final_simulation_run);
    println!("\n=== Final Results ===");
    final_stats.printout(&campaigns, &sellers, &campaign_params);
}
