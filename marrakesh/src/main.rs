mod types;
mod simulationrun;
mod converge;
mod utils;
mod impressions;

use types::{AddCampaignParams, AddSellerParams, CampaignType, ChargeType, Campaigns, Sellers};
use simulationrun::{CampaignParams, SimulationRun, SimulationStat};
use converge::SimulationConverge;
use impressions::{Impressions, ImpressionsParam};

/// Default implementation of ImpressionsParam with parametrizable distribution parameters
pub struct ImpressionsDefault {
    best_other_bid_mean: f64,
    best_other_bid_stddev: f64,
    floor_cpm_mean: f64,
    floor_cpm_stddev: f64,
    base_impression_value_mean: f64,
    base_impression_value_stddev: f64,
    value_to_campaign_multiplier_mean: f64,
    value_to_campaign_multiplier_stddev: f64,
    fixed_cost_floor_cpm: f64,
}

impl ImpressionsDefault {
    /// Create a new ImpressionsDefault with specified distribution parameters
    pub fn new(
        best_other_bid_mean: f64,
        best_other_bid_stddev: f64,
        floor_cpm_mean: f64,
        floor_cpm_stddev: f64,
        base_impression_value_mean: f64,
        base_impression_value_stddev: f64,
        value_to_campaign_multiplier_mean: f64,
        value_to_campaign_multiplier_stddev: f64,
        fixed_cost_floor_cpm: f64,
    ) -> Self {
        Self {
            best_other_bid_mean,
            best_other_bid_stddev,
            floor_cpm_mean,
            floor_cpm_stddev,
            base_impression_value_mean,
            base_impression_value_stddev,
            value_to_campaign_multiplier_mean,
            value_to_campaign_multiplier_stddev,
            fixed_cost_floor_cpm,
        }
    }
}

impl ImpressionsParam for ImpressionsDefault {
    fn best_other_bid_dist(&self) -> Box<dyn impressions::DistributionF64> {
        Box::new(utils::create_lognormal(self.best_other_bid_mean, self.best_other_bid_stddev))
    }

    fn floor_cpm_dist(&self) -> Box<dyn impressions::DistributionF64> {
        Box::new(utils::create_lognormal(self.floor_cpm_mean, self.floor_cpm_stddev))
    }

    fn base_impression_value_dist(&self) -> Box<dyn impressions::DistributionF64> {
        Box::new(utils::create_lognormal(self.base_impression_value_mean, self.base_impression_value_stddev))
    }

    fn value_to_campaign_multiplier_dist(&self) -> Box<dyn impressions::DistributionF64> {
        Box::new(utils::create_lognormal(self.value_to_campaign_multiplier_mean, self.value_to_campaign_multiplier_stddev))
    }

    fn fixed_cost_floor_cpm(&self) -> f64 {
        self.fixed_cost_floor_cpm
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

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsDefault::new(
        10.0,  // best_other_bid_mean
        3.0,   // best_other_bid_stddev
        10.0,  // floor_cpm_mean
        3.0,   // floor_cpm_stddev
        10.0,  // base_impression_value_mean
        3.0,   // base_impression_value_stddev
        1.0,   // value_to_campaign_multiplier_mean
        0.2,   // value_to_campaign_multiplier_stddev
        0.0,   // fixed_cost_floor_cpm
    );
    let impressions = Impressions::new(&sellers, &impressions_params);

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
