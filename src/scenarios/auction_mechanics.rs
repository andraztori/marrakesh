/// This scenario tests auction mechanics with optimal margin application.
///
/// It uses BidDeterminationBidFixedMargin which optimally applies margin by adjusting
/// gross_bid = optimized_bid / (1 - margin) while keeping net_bid = optimized_bid.

#[allow(unused_imports)]
use crate::simulationrun::{Marketplace, SimulationType};
use crate::auction_mechanism::{AuctionMechanismPlainNet, AuctionMechanismPlainGross, AuctionMechanismExpectedNetSpend, AuctionMechanismExpectedGrossRevenue, AuctionMechanismExpectedMargin};
use crate::sellers::{SellerType, SellerConvergeStrategy, Sellers};
use crate::campaigns::{Campaigns, CampaignGeneral};
use crate::campaign::CampaignTrait;
use crate::campaign_targets::CampaignTargetTotalBudget;
use crate::bid_valuers_single::BidValuerMultiplicative;
use crate::bid_optimizers::{BidOptimizerTrait, BidOptimizerOptimal};
use crate::bid_determination::BidDeterminationBidFixedMargin;
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
    short_name: "auction_mechanics",
    run,
});

// Margin constants
const MARGIN_LOW: f64 = 0.1;  // 5%
const MARGIN_HIGH: f64 = 0.5; // 15%

/// Prepare simulation converge instance with campaign and seller setup
fn prepare_simulationconverge(auction_mechanism: Box<dyn crate::auction_mechanism::AuctionMechanismTrait>) -> SimulationConverge {
    // Initialize containers for campaigns and sellers
    let mut campaigns = Campaigns::new();
    let mut sellers = Sellers::new();

    // Add campaign with low margin using add_advanced (ID is automatically set to match Vec index)
    let campaign_low: Box<dyn CampaignTrait> = Box::new(CampaignGeneral {
        campaign_id: 0, // Will be set by add_advanced
        campaign_name: "Campaign Low Margin".to_string(),
        converge_targets: vec![Box::new(CampaignTargetTotalBudget {
            total_budget_target: 100.0,
        })],
        converge_controllers: vec![Box::new(crate::controllers::ControllerProportionalDerivative::new())],
        bid_valuer: Box::new(BidValuerMultiplicative),
        bid_optimizer: Box::new(BidOptimizerOptimal) as Box<dyn BidOptimizerTrait>,
        net_and_gross: Box::new(BidDeterminationBidFixedMargin::new(MARGIN_LOW)),
    });
    campaigns.add_advanced(campaign_low);

    // Add campaign with high margin using add_advanced (ID is automatically set to match Vec index)
    let campaign_high: Box<dyn CampaignTrait> = Box::new(CampaignGeneral {
        campaign_id: 1, // Will be set by add_advanced
        campaign_name: "Campaign High Margin".to_string(),
        converge_targets: vec![Box::new(CampaignTargetTotalBudget {
            total_budget_target: 100.0,
        })],
        converge_controllers: vec![Box::new(crate::controllers::ControllerProportionalDerivative::new())],
        bid_valuer: Box::new(BidValuerMultiplicative),
        bid_optimizer: Box::new(BidOptimizerOptimal) as Box<dyn BidOptimizerTrait>,
        net_and_gross: Box::new(BidDeterminationBidFixedMargin::new(MARGIN_HIGH)),
    });
    campaigns.add_advanced(campaign_high);

    // Add seller (ID is automatically set to match Vec index)
    sellers.add(
        "HB".to_string(),  // seller_name
        SellerType::FIRST_PRICE,  // seller_type
        SellerConvergeStrategy::NONE { default_value: 1.0 },  // seller_converge
        100000,  // impressions_on_offer
        CompetitionGeneratorLogNormal::new(10.0),  // competition_generator
        floors::FloorGeneratorLogNormal::new(1.0, 3.0),  // floor_generator
    );

    // Create impressions parameters
    let impressions_params = ImpressionsParam::new(
        utils::lognormal_dist(10.0, 3.0),  // base_impression_value_dist
        utils::lognormal_dist(1.0, 2.0),   // value_to_campaign_multiplier_dist
    );

    // Create marketplace containing campaigns, sellers, and impressions
    let marketplace = Marketplace::new(campaigns, sellers, &impressions_params, SimulationType::Standard, auction_mechanism);

    // Create simulation converge instance (initializes campaign and seller converges internally)
    SimulationConverge::new(marketplace)
}

pub fn run(scenario_name: &str, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    // Run variant A with PlainNet auction mechanism
    let simulation_converge_a = prepare_simulationconverge(AuctionMechanismPlainNet::new());
    let stats_a = simulation_converge_a.run_variant("Running auction mechanics with PlainNet mechanism", scenario_name, "plain-net", 100, logger)?;
    
    // Run variant B with PlainGross auction mechanism
    let simulation_converge_b = prepare_simulationconverge(AuctionMechanismPlainGross::new());
    let stats_b = simulation_converge_b.run_variant("Running auction mechanics with PlainGross mechanism", scenario_name, "plain-gross", 100, logger)?;
    
    // Run variant C with ExpectedNetSpend auction mechanism
    let simulation_converge_c = prepare_simulationconverge(AuctionMechanismExpectedNetSpend::new());
    let stats_c = simulation_converge_c.run_variant("Running auction mechanics with ExpectedNetSpend mechanism", scenario_name, "expected-net-spend", 100, logger)?;
    
    // Run variant D with ExpectedGrossRevenue auction mechanism
    let simulation_converge_d = prepare_simulationconverge(AuctionMechanismExpectedGrossRevenue::new());
    let stats_d = simulation_converge_d.run_variant("Running auction mechanics with ExpectedGrossRevenue mechanism", scenario_name, "expected-gross-revenue", 100, logger)?;
    
    // Run variant E with ExpectedMargin auction mechanism
    let simulation_converge_e = prepare_simulationconverge(AuctionMechanismExpectedMargin::new());
    let stats_e = simulation_converge_e.run_variant("Running auction mechanics with ExpectedMargin mechanism", scenario_name, "expected-margin", 100, logger)?;
    
    // Validate expected marketplace behavior
    logln!(logger, LogEvent::Scenario, "");
    
    let mut errors: Vec<String> = Vec::new();
    
    // Validate margins for all variants
    let variants = vec![
        ("A (PlainNet)", &stats_a),
        ("B (PlainGross)", &stats_b),
        ("C (ExpectedNetSpend)", &stats_c),
        ("D (ExpectedGrossRevenue)", &stats_d),
        ("E (ExpectedMargin)", &stats_e),
    ];
    
    // Verify that PlainNet and ExpectedNetSpend lead to the same net values for each campaign
    // (They should be effectively the same since ExpectedNetSpend multiplies net_bid by win probability,
    // but if win probability is the same for all bids at the same net_bid level, the ranking should be identical)
    if stats_a.campaign_stats.len() == stats_c.campaign_stats.len() {
        for (campaign_idx, (stat_a, stat_c)) in stats_a.campaign_stats.iter().zip(stats_c.campaign_stats.iter()).enumerate() {
            let net_diff = (stat_a.total_net_supply_cost - stat_c.total_net_supply_cost).abs();
            let net_tolerance = 0.01; // Small tolerance for floating point differences
            let msg = format!(
                "Campaign {}: PlainNet and ExpectedNetSpend have same net value: A={:.2}, C={:.2}, diff={:.4} (tolerance={:.4})",
                campaign_idx, stat_a.total_net_supply_cost, stat_c.total_net_supply_cost, net_diff, net_tolerance
            );
            if net_diff <= net_tolerance {
                logln!(logger, LogEvent::Scenario, "✓ {}", msg);
            } else {
                errors.push(msg.clone());
                errln!(logger, LogEvent::Scenario, "✗ {}", msg);
            }
        }
    }
    
    for (variant_name, stats) in variants {
        // Verify that low margin campaign obtains more value than high margin campaign
        if stats.campaign_stats.len() >= 2 {
            let campaign_stat_low = &stats.campaign_stats[0];
            let campaign_stat_high = &stats.campaign_stats[1];
            let value_low = campaign_stat_low.total_value;
            let value_high = campaign_stat_high.total_value;
            let value_diff = value_low - value_high;
            let min_value_diff = 0.0; // Low margin campaign should have at least as much value as high margin
            
            let msg = format!(
                "Variant {} - Low margin campaign has higher value than high margin: low={:.2}, high={:.2}, diff={:.2} (expected >= {:.2})",
                variant_name, value_low, value_high, value_diff, min_value_diff
            );
            if value_diff >= min_value_diff {
                logln!(logger, LogEvent::Scenario, "✓ {}", msg);
            } else {
                errors.push(msg.clone());
                errln!(logger, LogEvent::Scenario, "✗ {}", msg);
            }
        }
        
        // Check: Campaign 0 (low margin) should have MARGIN_LOW margin
        if stats.campaign_stats.len() >= 1 {
            let campaign_stat_low = &stats.campaign_stats[0];
            let gross_buyer_charge_low = campaign_stat_low.total_gross_buyer_charge;
            let net_supply_cost_low = campaign_stat_low.total_net_supply_cost;
            let margin_low = if gross_buyer_charge_low > 0.0 {
                (gross_buyer_charge_low - net_supply_cost_low) / gross_buyer_charge_low
            } else {
                0.0
            };
            let margin_tolerance_pct = 0.01; // 1% tolerance
            let msg = format!(
                "Variant {} - Campaign 0 (low margin) has {:.0}% margin: gross={:.2}, net={:.2}, margin={:.4} (expected {:.4} ± {:.4})",
                variant_name, MARGIN_LOW * 100.0, gross_buyer_charge_low, net_supply_cost_low, margin_low, MARGIN_LOW, margin_tolerance_pct
            );
            if (margin_low - MARGIN_LOW).abs() <= margin_tolerance_pct {
                logln!(logger, LogEvent::Scenario, "✓ {}", msg);
            } else {
                errors.push(msg.clone());
                errln!(logger, LogEvent::Scenario, "✗ {}", msg);
            }
        }
        
        // Check: Campaign 1 (high margin) should have MARGIN_HIGH margin
        if stats.campaign_stats.len() >= 2 {
            let campaign_stat_high = &stats.campaign_stats[1];
            let gross_buyer_charge_high = campaign_stat_high.total_gross_buyer_charge;
            let net_supply_cost_high = campaign_stat_high.total_net_supply_cost;
            let margin_high = if gross_buyer_charge_high > 0.0 {
                (gross_buyer_charge_high - net_supply_cost_high) / gross_buyer_charge_high
            } else {
                0.0
            };
            let margin_tolerance_pct = 0.01; // 1% tolerance
            let msg = format!(
                "Variant {} - Campaign 1 (high margin) has {:.0}% margin: gross={:.2}, net={:.2}, margin={:.4} (expected {:.4} ± {:.4})",
                variant_name, MARGIN_HIGH * 100.0, gross_buyer_charge_high, net_supply_cost_high, margin_high, MARGIN_HIGH, margin_tolerance_pct
            );
            if (margin_high - MARGIN_HIGH).abs() <= margin_tolerance_pct {
                logln!(logger, LogEvent::Scenario, "✓ {}", msg);
            } else {
                errors.push(msg.clone());
                errln!(logger, LogEvent::Scenario, "✗ {}", msg);
            }
        }
    }
    
    // Verify that when comparing PlainNet and ExpectedGrossRevenue:
    // - Low margin campaign decreases obtained value in ExpectedGrossRevenue
    // - High margin campaign increases obtained value in ExpectedGrossRevenue
    // - Both changes are < 5%
    if stats_a.campaign_stats.len() >= 2 && stats_d.campaign_stats.len() >= 2 {
        // Check low margin campaign (campaign 0)
        let value_low_a = stats_a.campaign_stats[0].total_value;
        let value_low_d = stats_d.campaign_stats[0].total_value;
        let value_diff_low = value_low_d - value_low_a;
        let pct_change_low = if value_low_a > 0.0 {
            (value_diff_low / value_low_a) * 100.0
        } else {
            0.0
        };
        let max_pct_change = 5.0; // 5% maximum change
        let msg_low = format!(
            "Low margin campaign: PlainNet={:.2}, ExpectedGrossRevenue={:.2}, diff={:.2}, pct_change={:.2}% (expected < 0 and |pct| < {:.0}%)",
            value_low_a, value_low_d, value_diff_low, pct_change_low, max_pct_change
        );
        if value_diff_low < 0.0 && pct_change_low.abs() < max_pct_change {
            logln!(logger, LogEvent::Scenario, "✓ {}", msg_low);
        } else {
            errors.push(msg_low.clone());
            errln!(logger, LogEvent::Scenario, "✗ {}", msg_low);
        }
        
        // Check high margin campaign (campaign 1)
        let value_high_a = stats_a.campaign_stats[1].total_value;
        let value_high_d = stats_d.campaign_stats[1].total_value;
        let value_diff_high = value_high_d - value_high_a;
        let pct_change_high = if value_high_a > 0.0 {
            (value_diff_high / value_high_a) * 100.0
        } else {
            0.0
        };
        let msg_high = format!(
            "High margin campaign: PlainNet={:.2}, ExpectedGrossRevenue={:.2}, diff={:.2}, pct_change={:.2}% (expected > 0 and |pct| < {:.0}%)",
            value_high_a, value_high_d, value_diff_high, pct_change_high, max_pct_change
        );
        if value_diff_high > 0.0 && pct_change_high.abs() < max_pct_change {
            logln!(logger, LogEvent::Scenario, "✓ {}", msg_high);
        } else {
            errors.push(msg_high.clone());
            errln!(logger, LogEvent::Scenario, "✗ {}", msg_high);
        }
    }
    
    // Verify that PlainNet provides the highest total obtained value
    let total_value_a = stats_a.overall_stat.total_value;
    let total_value_b = stats_b.overall_stat.total_value;
    let total_value_c = stats_c.overall_stat.total_value;
    let total_value_d = stats_d.overall_stat.total_value;
    let total_value_e = stats_e.overall_stat.total_value;
    
    let max_other_value = total_value_b.max(total_value_c).max(total_value_d).max(total_value_e);
    let value_diff = total_value_a - max_other_value;
    let min_value_diff = 0.0; // PlainNet should have at least as much value as others
    
    let msg = format!(
        "PlainNet (variant A) has highest total value: A={:.2}, max(other)={:.2}, diff={:.2} (expected >= {:.2})",
        total_value_a, max_other_value, value_diff, min_value_diff
    );
    if value_diff >= min_value_diff {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Verify that PlainGross provides the lowest total obtained value
    let min_other_value = total_value_a.min(total_value_c).min(total_value_d).min(total_value_e);
    let value_diff_lowest = min_other_value - total_value_b;
    let min_value_diff_lowest = 0.0; // PlainGross should have at most as much value as others
    
    let msg = format!(
        "PlainGross (variant B) has lowest total value: B={:.2}, min(other)={:.2}, diff={:.2} (expected <= {:.2})",
        total_value_b, min_other_value, value_diff_lowest, min_value_diff_lowest
    );
    if value_diff_lowest >= min_value_diff_lowest {
        logln!(logger, LogEvent::Scenario, "✓ {}", msg);
    } else {
        errors.push(msg.clone());
        errln!(logger, LogEvent::Scenario, "✗ {}", msg);
    }
    
    // Log all variant values for comparison
    logln!(logger, LogEvent::Scenario, "Total values by variant: A (PlainNet)={:.2}, B (PlainGross)={:.2}, C (ExpectedNetSpend)={:.2}, D (ExpectedGrossRevenue)={:.2}, E (ExpectedMargin)={:.2}",
        total_value_a, total_value_b, total_value_c, total_value_d, total_value_e);
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Scenario '{}' validation failed:\n{}", scenario_name, errors.join("\n")).into())
    }
}