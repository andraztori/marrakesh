/// This scenario tests value behavior with different campaign configurations.
///
/// It has three variants:
///
/// - Variant A: A single campaign with $20 budget
/// - Variant B: Two campaigns with $10 budget each
/// - Variant C: Two campaigns with $10 budget each, in the same value group
///
/// Expected behavior:
/// - Two $10 campaigns should obtain higher value than one $20 campaign
/// - But when they are in the same value group, the value will be the same as one $20 campaign

use crate::simulationrun::{Marketplace, SimulationType};
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{CampaignType, ConvergeTarget, Campaigns};
use crate::converge::SimulationConverge;
use crate::impressions::ImpressionsParam;
use crate::competition::CompetitionGeneratorLogNormal;
use crate::floors;
use crate::utils;
use crate::logger::{Logger, LogEvent};
use crate::logln;
use crate::errln;

// Register this scenario in the catalog
inventory::submit!(crate::scenarios::ScenarioEntry {
    short_name: "value_groups",
    run,
});

/// Variant configuration for the value behavior scenario
enum VariantConfig {
    /// Single campaign with $20 budget
    Single20,
    /// Two campaigns with $10 budget each
    Two10,
    /// Two campaigns with $10 budget each in the same value group
    Two10ValueGroup,
}

/// Prepare simulation converge instance with parametrized campaign configuration
fn prepare_simulationconverge(variant: VariantConfig) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaigns based on variant
    match variant {
        VariantConfig::Single20 => {
            // Add a single campaign with $20 budget
            campaigns.add(
                "Campaign 0".to_string(),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 20.0 }],
            );
        }
        VariantConfig::Two10 => {
            // Add two campaigns with $10 budget each
            campaigns.add(
                "Campaign 0".to_string(),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 }],
            );
            campaigns.add(
                "Campaign 1".to_string(),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 }],
            );
        }
        VariantConfig::Two10ValueGroup => {
            // Add two campaigns with $10 budget each
            campaigns.add(
                "Campaign 0".to_string(),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 }],
            );
            campaigns.add(
                "Campaign 1".to_string(),
                CampaignType::MULTIPLICATIVE_PACING,
                vec![ConvergeTarget::TOTAL_BUDGET { target_total_budget: 10.0 }],
            );
            // Create a value group containing both campaigns
            campaigns.create_value_group(vec![0, 1]);
        }
    }

    // Add a single seller
    sellers.add(
        "HB".to_string(),
        SellerType::FIRST_PRICE,
        SellerConvergeStrategy::NONE { default_value: 1.0 },
        10000,
        CompetitionGeneratorLogNormal::new(10.0),
        floors::FloorGeneratorLogNormal::new(0.2, 3.0),
        
    );

    // Create impressions parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),
        utils::lognormal_dist(1.0, 0.2),
    );

    // Create marketplace containing campaigns, sellers, and impressions
    // Note: Marketplace::new() automatically calls finalize_groups()
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::FractionalInternalAuction { softmax_temperature: 0.5 });

    // Create simulation converge instance
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A: Single campaign with $20 budget
    let simulation_converge_a = prepare_simulationconverge(VariantConfig::Single20);
    let stats_a = simulation_converge_a.run_variant("Running with single $20 campaign", scenario_name, "single_20", 100, logger)?;
    
    // Run variant B: Two campaigns with $10 budget each
    let simulation_converge_b = prepare_simulationconverge(VariantConfig::Two10);
    let stats_b = simulation_converge_b.run_variant("Running with two $10 campaigns", scenario_name, "two_10", 100, logger)?;
    
    // Run variant C: Two campaigns with $10 budget each in the same value group
    let simulation_converge_c = prepare_simulationconverge(VariantConfig::Two10ValueGroup);
    let stats_c = simulation_converge_c.run_variant("Running with two $10 campaigns in value group", scenario_name, "two_10_value_group", 100, logger)?;
    
    // Compare the three variants to verify expected marketplace behavior
    // Variant B (two $10 campaigns) should have:
    // - Higher total value than variant A (single $20 campaign)
    //
    // Variant C (two $10 campaigns in value group) should have:
    // - Same total value as variant A (single $20 campaign)
    
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors = Vec::new();
    
    // Check: Variant B has higher total value than variant A
    let msg = format!(
        "Variant B (two $10 campaigns) has higher total value than variant A (single $20 campaign): {:.2} > {:.2}",
        stats_b.overall_stat.total_value,
        stats_a.overall_stat.total_value
    );
    if stats_b.overall_stat.total_value > stats_a.overall_stat.total_value {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Check: Variant C has the same total value as variant A (within tolerance)
    let value_diff = (stats_c.overall_stat.total_value - stats_a.overall_stat.total_value).abs();
    let value_avg = (stats_c.overall_stat.total_value + stats_a.overall_stat.total_value) / 2.0;
    let value_diff_pct = if value_avg > 0.0 { value_diff / value_avg * 100.0 } else { 0.0 };
    let tolerance_pct = 1.0; // 1% tolerance
    let msg = format!(
        "Variant C (two $10 campaigns in value group) has same total value as variant A (single $20 campaign): {:.2} ≈ {:.2} (diff: {:.2}%, tolerance: {:.0}%)",
        stats_c.overall_stat.total_value,
        stats_a.overall_stat.total_value,
        value_diff_pct,
        tolerance_pct
    );
    if value_diff_pct < tolerance_pct {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}

