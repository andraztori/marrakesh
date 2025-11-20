/// This is a simple scenario that uses simple first price bidding on two sources of supply -
/// fixed price (MRG) and regular first price (HB).
///
/// Its two variants show:
///
/// - If there is scarce HB supply prices are high enough that there is profit on MRG supply
///
/// - If there is abundant HB supply, demand flows to it and leaves prices below guaranteed
///   prices on MRG

use crate::simulationrun::Marketplace;
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::{Impressions, ImpressionsParam};
use crate::competition::{CompetitionGeneratorLogNormal, CompetitionGeneratorNone};
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "HBabundance",
    run,
});

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(hb_impressions: usize) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add two hardcoded campaigns (IDs are automatically set to match Vec index)
    campaigns.add(
        "Campaign 0".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        ConvergeTarget::TOTAL_IMPRESSIONS { target_total_impressions: 1000 },
    );

    campaigns.add(
        "Campaign 1".to_string(),  // campaign_name
        CampaignType::MULTIPLICATIVE_PACING,
        ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 },
    );

    // Add two sellers (IDs are automatically set to match Vec index)
    sellers.add(
        "MRG".to_string(),  // seller_name
        SellerType::FIXED_PRICE {
            fixed_cost_cpm: 10.0,
        },  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        1000,  // impressions_on_offer
        CompetitionGeneratorNone::new(),  // competition_generator
        floors::FloorGeneratorFixed::new(0.0),  // floor_generator
    );

    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        hb_impressions,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
        floors::FloorGeneratorLogNormal::new(0.2, 3.0),  // floor_generator
    );

    // Create impressions for all sellers using default parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 0.2),   // value_to_campaign_multiplier_dist
    );
    let impressions = Impressions::new(&sellers, &impressions_params);

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace {
        campaigns,
        sellers,
        impressions,
    };

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant with 100 HB impressions
    let simulation_converge_a = prepare_simulationconverge(1000);
    let stats_a = simulation_converge_a.run_variant("Running with Scarce HB impressions", scenario_name, "scarce", 100, logger);
    
    // Run variant with 1000 HB impressions
    let simulation_converge_b = prepare_simulationconverge(10000);
    let stats_b = simulation_converge_b.run_variant("Running with Abundant HB impressions", scenario_name, "abundant", 100, logger);
    
    // Compare the two variants to verify expected marketplace behavior
    // Variant A (100 HB) should have:
    // - Higher total cost charged to buyers (due to scarcity driving up prices)
    // - Lower total value obtained (fewer impressions available)
    // - Supply cost < buyer charge (marketplace is profitable)
    //
    // Variant B (1000 HB) should have:
    // - Lower total cost charged to buyers (abundance drives prices down)
    // - Higher total value obtained (more impressions available)
    // - Supply cost > buyer charge (marketplace becomes unprofitable)
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors = Vec::new();
    
    // Check: Variant A has higher total cost charged to buyers
    let msg = format!(
        "Variant A (Scarce HB) has higher total buyer charge than variant B (Abundant HB): {:.2} > {:.2}",
        stats_a.overall_stat.total_buyer_charge,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_buyer_charge > stats_b.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: Variant A has lower total value
    let msg = format!(
        "Variant A (Scarce HB) has lower total value than variant B (Abundant HB): {:.2} < {:.2}",
        stats_a.overall_stat.total_value,
        stats_b.overall_stat.total_value
    );
    if stats_a.overall_stat.total_value < stats_b.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: In variant A, cost of inventory is lower than cost charged to buyers
    let msg = format!(
        "Variant A (Scarce HB) is profitable (supply_cost < buyer_charge): {:.2} < {:.2}",
        stats_a.overall_stat.total_supply_cost,
        stats_a.overall_stat.total_buyer_charge
    );
    if stats_a.overall_stat.total_supply_cost < stats_a.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    // Check: In variant B, cost of inventory is higher than cost charged to buyers
    let msg = format!(
        "Variant B (Abundant HB) is unprofitable (supply_cost > buyer_charge): {:.2} > {:.2}",
        stats_b.overall_stat.total_supply_cost,
        stats_b.overall_stat.total_buyer_charge
    );
    if stats_b.overall_stat.total_supply_cost > stats_b.overall_stat.total_buyer_charge {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "{}", msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}
