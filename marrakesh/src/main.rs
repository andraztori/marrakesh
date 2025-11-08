mod types;
mod simulationrun;
mod converge;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal, LogNormal};
use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Impression, Campaigns, Sellers, MAX_CAMPAIGNS};
use simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use converge::SimulationConverge;

/// Container for impressions with methods to create impressions
pub struct Impressions {
    pub impressions: Vec<Impression>,
}

impl Impressions {
    /// Create a new Impressions container and populate it from sellers
    pub fn new(sellers: &Sellers) -> Self {
        // Use deterministic seed for reproducible results
        let mut rng = StdRng::seed_from_u64(999);
        let best_other_bid_dist = Normal::new(10.0, 3.0).unwrap();
        let floor_cpm_dist = Normal::new(10.0, 3.0).unwrap();
        let base_impression_value_dist = Normal::new(5.0, 3.0).unwrap();
        // Log-normal distribution with mean=1.0: if X ~ LogNormal(μ, σ), then E[X] = exp(μ + σ²/2)
        // To have E[X] = 1, we need μ = -σ²/2. Using σ=0.2 gives μ = -0.02
        let value_to_campaign_multiplier_dist = LogNormal::new(-0.02, 0.2).unwrap();

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
