mod types;
mod simulationrun;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};
use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Impression, Campaigns, Sellers, MAX_CAMPAIGNS};
use simulationrun::{CampaignParams, SimulationRun, SimulationStat};

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
        let value_to_campaign_dist = Normal::new(10.0, 3.0).unwrap();

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

                // Generate random values for value_to_campaign_id array
                let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
                for i in 0..MAX_CAMPAIGNS {
                    value_to_campaign_id[i] = value_to_campaign_dist.sample(&mut rng);
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
    
    // Run simulation loop with pacing adjustments (maximum 10 iterations)
    for iteration in 0..100 {
        println!("\n=== Iteration {} ===", iteration + 1);
        
        // Run auctions for all impressions
        let simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params);

        // Generate statistics
        let stats = SimulationStat::new(&campaigns, &sellers, &impressions, &simulation_run);
        
        // Adjust pacing for each campaign based on targets
        // Use adaptive adjustment that reduces as we get closer to target
        let mut pacing_changed = false;
        for (index, campaign) in campaigns.campaigns.iter().enumerate() {
            let campaign_stat = &stats.campaign_stats[index];
            let pacing = &mut campaign_params.params[index].pacing;
            
            // Get target and actual values based on campaign type
            let (target, actual) = match &campaign.campaign_type {
                CampaignType::FIXED_IMPRESSIONS { total_impressions_target } => {
                    (*total_impressions_target as f64, campaign_stat.impressions_obtained as f64)
                }
                CampaignType::FIXED_BUDGET { total_budget_target } => {
                    (*total_budget_target, campaign_stat.total_buyer_charge)
                }
            };
            
            let tolerance = target * 0.01; // 1% tolerance
            
            if actual < target - tolerance {
                // Below target - increase pacing
                // Calculate error percentage and use proportional adjustment
                let error_ratio = (target - actual) / target;
                // Use smaller adjustment when closer to target (max 10%)
                let adjustment_factor = (error_ratio * 0.1).min(0.1);
                *pacing *= 1.0 + adjustment_factor;
                pacing_changed = true;
            } else if actual > target + tolerance {
                // Above target - decrease pacing
                // Calculate error percentage and use proportional adjustment
                let error_ratio = (actual - target) / target;
                // Use smaller adjustment when closer to target (max 10%)
                let adjustment_factor = (error_ratio * 0.1).min(0.1);
                *pacing *= 1.0 - adjustment_factor;
                pacing_changed = true;
            }
            // If practically on goal (within 1%), keep constant
        }
        
        // Output campaign statistics only during iterations
        stats.printout_campaigns(&campaigns, &campaign_params);
        
        // Break early if no pacing changes were made (converged)
        if !pacing_changed {
            println!("\nConverged after {} iterations", iteration + 1);
            break;
        }
    }
    
    // Run final simulation and output complete statistics
    let final_simulation_run = SimulationRun::new(&impressions, &campaigns, &campaign_params);
    let final_stats = SimulationStat::new(&campaigns, &sellers, &impressions, &final_simulation_run);
    println!("\n=== Final Results ===");
    final_stats.printout(&campaigns, &sellers, &campaign_params);
}
