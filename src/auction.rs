use crate::auction_mechanism::AuctionMechanismTrait;
use crate::impressions::Impression;
use crate::campaigns::Campaigns;
use crate::seller::SellerTrait;
use crate::logger::{Logger, LogEvent};
use crate::errln;
use crate::logln;
use crate::utils::VERBOSE_AUCTION;
use std::sync::atomic::Ordering;

/// Represents the result of an auction
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum AuctionResult {
    Campaign { 
        campaign_id: usize, 
        supply_cost: f64,
        net_supply_cost: f64,
        gross_buyer_charge: f64,
    },
    LOST {
        supply_cost: f64,
    },
    NO_DEMAND {
        supply_cost: f64,
    },
}

/// Represents a fractional winner in a fractional auction
/// Note: net_supply_cost, gross_buyer_charge, and supply_cost are here not yet multiplied by win_fraction
#[derive(Debug, Clone, PartialEq)]
pub struct FractionalWinner {
    pub campaign_id: usize,
    pub supply_cost: f64,
    pub net_supply_cost: f64,
    pub gross_buyer_charge: f64,
    pub win_fraction: f64,
}

/// Represents the result of a fractional auction (can have multiple campaigns winning fractions)
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum FractionalAuctionResult {
    Campaigns {
        winners: Vec<FractionalWinner>,
        // Supply cost is under each individual winner, fractionally
    },
    LOST {
        supply_cost: f64,
    },
    NO_DEMAND {
        supply_cost: f64,
    },
}

/// Auction structure that holds the auction mechanism
pub struct Auction {
    pub auction_mechanism: Box<dyn AuctionMechanismTrait>,
}

impl Auction {
    /// Run an auction for the given impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the auction result
    pub fn run_auction(&self, impression: &Impression, campaigns: &Campaigns, campaign_converges: &[Vec<&dyn crate::controllers::ControllerStateTrait>], seller: &dyn SellerTrait, seller_converge: &dyn crate::controllers::ControllerStateTrait, logger: &mut Logger) -> AuctionResult {
        // Get bids from all campaigns
        // highest_scoring_campaign is a tuple of (campaign_id, score, CompleteBid)
        let mut highest_scoring_campaign: Option<(usize, f64, crate::campaign::CompleteBid)> = None;
        let mut all_bids = if VERBOSE_AUCTION.load(Ordering::Relaxed) {
            Some(Vec::new())
        } else {
            None
        };

        // Get seller_control_factor from seller using get_control_variable
        let seller_control_factor = seller.get_control_variable(seller_converge);

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_converges[campaign_id];
            // Resolve value_to_campaign at call site using campaign's group ID
            let group_id = campaigns.campaign_to_value_group_mapping[campaign_id];
            let value_to_campaign = impression.value_to_campaign_group[group_id];
            // Use the trait method for get_bid
            if let Some(campaign_bid) = campaign.get_bid(impression, &campaign_converge, seller_control_factor, value_to_campaign, logger) {
                // Check if bid is below zero - skip negative bids
                if campaign_bid.gross_bid < 0.0 {
                    errln!(logger, LogEvent::Simulation, "Bid below zero: {:.4} from campaign_id: {}, skipping", campaign_bid.gross_bid, campaign_id);
                    continue;
                }
                if let Some(bids) = &mut all_bids {
                    bids.push((campaign_id, campaign_bid.gross_bid));
                }
                // Compute internal score using the auction mechanism
                let score = self.auction_mechanism.compute_internal_score(
                    &campaign_bid,
                    impression.competition.as_ref(),
                );
                // Compare scores to find the highest scoring campaign
                let current_highest_score = highest_scoring_campaign.as_ref().map(|(_, s, _)| *s).unwrap_or(f64::NEG_INFINITY);
                if score > current_highest_score {
                    highest_scoring_campaign = Some((campaign_id, score, campaign_bid));
                    //println!("Winning bid: {:.4}, campaign_id: {}, score: {:.4}", campaign_bid.gross_bid, campaign_id, score);
                }
            }
            // If get_bid returns None, skip this campaign (warning already logged)
        }

        // Extract winning bid CPM for logging before moving highest_scoring_campaign
        let winning_bid_cpm_for_logging = highest_scoring_campaign.as_ref().map(|(_, _, b)| b.gross_bid).unwrap_or(0.0);

        // Determine the result based on winning bid
        // Check all failure conditions first, then create winner in one place
        let winner = 'result: {
            // No campaigns participated
            let (campaign_id, _score, winning_bid) = match highest_scoring_campaign {
                Some((id, score, bid)) => (id, score, bid),
                None => {
                    let supply_cost = seller.get_supply_cost_cpm(0.0) / 1000.0;
                    break 'result AuctionResult::NO_DEMAND { supply_cost };
                },
            };
            
            // Winning bid is below z or below competition - no winner (LOST)
            let competition_bid = impression.competition.as_ref().map(|c| c.bid_cpm).unwrap_or(0.0);
            let minimum_cpm_to_win = impression.floor_cpm.max(competition_bid);
            
            if winning_bid.net_bid < minimum_cpm_to_win {
                let supply_cost = seller.get_supply_cost_cpm(0.0) / 1000.0;
                break 'result AuctionResult::LOST { supply_cost };
            }
            
            // Valid winner - bid passes all checks (floor and competition if present)
            // Use net_bid and gross_bid from the CompleteBid object
            // Convert from CPM to actual cost by dividing by 1000
            AuctionResult::Campaign {
                campaign_id,
                supply_cost: seller.get_supply_cost_cpm(winning_bid.net_bid) / 1000.0,
                net_supply_cost: winning_bid.net_bid / 1000.0,
                gross_buyer_charge: winning_bid.gross_bid / 1000.0,
            }
        };

        // Log auction data in CSV format
        if VERBOSE_AUCTION.load(Ordering::Relaxed) {
            let all_bids = all_bids.as_ref().unwrap();
            
            // Build CSV row
            let mut csv_fields = Vec::new();
            
            // seller_id
            csv_fields.push(format!("{}", impression.seller_id));
            
            // demand_id (winner identifier)
            let demand_id = match &winner {
                AuctionResult::Campaign { campaign_id, .. } => format!("{}", campaign_id),
                AuctionResult::LOST { .. } => "LOST".to_string(),
                AuctionResult::NO_DEMAND { .. } => "NO_DEMAND".to_string(),
            };
            csv_fields.push(demand_id);
            
            // winning_bid
            csv_fields.push(format!("{:.4}", winning_bid_cpm_for_logging));
            
            // floor_cpm
            csv_fields.push(format!("{:.4}", impression.floor_cpm));
            
            // impression_base_value
            csv_fields.push(format!("{:.4}", impression.base_impression_value));
            
            // competing_bid, competing_offset, competing_scale
            if let Some(comp) = &impression.competition {
                csv_fields.push(format!("{:.4}", comp.bid_cpm));
                csv_fields.push(format!("{:.4}", comp.win_rate_actual_sigmoid_offset));
                csv_fields.push(format!("{:.4}", comp.win_rate_actual_sigmoid_scale));
            } else {
                csv_fields.push("".to_string());
                csv_fields.push("".to_string());
                csv_fields.push("".to_string());
            }
            
            // For each campaign: value and bid
            // Create a map of campaign_id to bid for quick lookup
            let bid_map: std::collections::HashMap<usize, f64> = all_bids.iter().cloned().collect();
            
            for campaign_id in 0..campaigns.campaigns.len() {
                // campaign value - get from campaign's group
                let group_index = campaigns.campaign_to_value_group_mapping[campaign_id];
                csv_fields.push(format!("{:.4}", impression.value_to_campaign_group[group_index]));
                
                // campaign bid (empty if no bid)
                if let Some(bid) = bid_map.get(&campaign_id) {
                    csv_fields.push(format!("{:.4}", bid));
                } else {
                    csv_fields.push("".to_string());
                }
            }
            
            logln!(logger, LogEvent::Auction, "{}", csv_fields.join(","));
        }

        winner
    }

    /// Run a fractional auction for the given impression with the given campaigns, campaign converges, seller, and seller convergence parameters
    /// Returns the fractional auction result
    /// 
    /// `softmax_temperature`: Temperature parameter for softmax calculation
    /// - Lower values (< 1.0) make the distribution sharper (more concentrated on highest bid)
    /// - Higher values (> 1.0) make the distribution smoother (more uniform)
    /// - Default: 1.0 (standard softmax)
    pub fn run_fractional_auction(&self, impression: &Impression, campaigns: &Campaigns, campaign_converges: &[Vec<&dyn crate::controllers::ControllerStateTrait>], seller: &dyn SellerTrait, seller_converge: &dyn crate::controllers::ControllerStateTrait, softmax_temperature: f64, logger: &mut Logger) -> FractionalAuctionResult {
        // Calculate minimum CPM needed to win this impression
        // Must be at least the floor, and if competition exists, must beat the competing bid
        let competition_bid = impression.competition.as_ref().map(|c| c.bid_cpm).unwrap_or(0.0);
        let minimum_cpm_to_win = impression.floor_cpm.max(competition_bid);

        // Collect all campaigns with bids above minimum_cpm_to_win along with their scores
        // Store (FractionalWinner, score) tuples temporarily
        let mut fractional_winners_with_scores: Vec<(FractionalWinner, f64)> = Vec::new();
        let mut any_bids_made = false;

        // Get seller_control_factor from seller using get_control_variable
        let seller_control_factor = seller.get_control_variable(seller_converge);

        for campaign in &campaigns.campaigns {
            let campaign_id = campaign.campaign_id();
            let campaign_converge = &campaign_converges[campaign_id];
            // Resolve value_to_campaign at call site using campaign's group ID
            let group_id = campaigns.campaign_to_value_group_mapping[campaign_id];
            let value_to_campaign = impression.value_to_campaign_group[group_id];
            // Use the trait method for get_bid
            if let Some(campaign_bid) = campaign.get_bid(impression, &campaign_converge, seller_control_factor, value_to_campaign, logger) {
                any_bids_made = true;
                // Check if bid is below zero - skip negative bids
                if campaign_bid.net_bid < 0.0 {
                    errln!(logger, LogEvent::Simulation, "Bid below zero: {:.4} from campaign_id: {}, skipping", campaign_bid.gross_bid, campaign_id);
                    continue;
                }                
                // If bid is above minimum_cpm_to_win, compute score and add to winners list
                if campaign_bid.net_bid >= minimum_cpm_to_win {
                    // Compute internal score using the auction mechanism
                    let score = self.auction_mechanism.compute_internal_score(
                        &campaign_bid,
                        impression.competition.as_ref(),
                    );
                    
                    fractional_winners_with_scores.push((
                        FractionalWinner {
                            campaign_id,
                            supply_cost: seller.get_supply_cost_cpm(campaign_bid.net_bid) / 1000.0,
                            net_supply_cost: campaign_bid.net_bid / 1000.0,
                            gross_buyer_charge: campaign_bid.gross_bid / 1000.0,
                            win_fraction: 1.0,
                        },
                        score,
                    ));
                }
            }
            // If get_bid returns None, skip this campaign (warning already logged)
        }

        // Calculate win_fraction using softmax based on scores with temperature
        // Temperature controls the sharpness: lower = sharper (more concentrated on highest score), higher = smoother (more uniform)
        let mut fractional_winners: Vec<FractionalWinner> = Vec::new();
        if !fractional_winners_with_scores.is_empty() {
            // Find maximum score for numerical stability (log-sum-exp trick)
            let max_score = fractional_winners_with_scores.iter()
                .map(|(_, score)| *score)
                .fold(f64::NEG_INFINITY, f64::max);
            
            // Calculate exp((score - max_score) / temperature) for each winner
            let exp_values: Vec<f64> = fractional_winners_with_scores.iter()
                .map(|(_, score)| ((score - max_score) / softmax_temperature).exp())
                .collect();
            
            // Calculate sum of exp values
            let sum_exp: f64 = exp_values.iter().sum();
            
            // Create FractionalWinner with calculated win_fraction
            for ((winner, _), exp_val) in fractional_winners_with_scores.iter().zip(exp_values.iter()) {
                let mut final_winner = winner.clone();
                final_winner.win_fraction = exp_val / sum_exp;
                fractional_winners.push(final_winner);
            }
        }

        // Determine the result based on collected winners
        // Check all failure conditions first, then create winners in one place
        let winners = if fractional_winners.is_empty() {
            // Distinguish between no bids (NO_DEMAND) and bids below threshold (LOST)
            // Even when impression is not sold, calculate supply cost (0.0 for first price, fixed_cost_cpm for fixed price)
            let supply_cost = seller.get_supply_cost_cpm(0.0) / 1000.0;
            if any_bids_made {
                FractionalAuctionResult::LOST { supply_cost }
            } else {
                FractionalAuctionResult::NO_DEMAND { supply_cost }
            }
        } else {
            // Valid winners - all passed the minimum_cpm_to_win threshold
            // For fractional auctions with winners, supply cost is calculated per winner
            // The actual supply cost will be aggregated from individual winners in statistics
            FractionalAuctionResult::Campaigns {
                winners: fractional_winners,
            }
        };
        winners
    }
}

