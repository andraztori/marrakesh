use crate::impressions::Impression;
use crate::campaign_targets::CampaignTargetTrait;
use crate::controllers::ControllerTrait;
use crate::logger::Logger;

/// Maximum number of controllers supported by campaigns
const MAX_CONTROLLERS: usize = 10;

/// Trait for campaigns participating in auctions
pub trait CampaignTrait {
    /// Get the campaign ID
    fn campaign_id(&self) -> usize;
    
    /// Get the campaign name
    fn campaign_name(&self) -> &str;
    
    /// Calculate the bid for this campaign given an impression, convergence parameter, and seller control factor
    /// Bid = campaign_control_factor * value_to_campaign * seller_control_factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    fn get_bid(&self, impression: &Impression, controller_states: &[&dyn crate::controllers::ControllerStateTrait], seller_control_factor: f64, value_to_campaign: f64, logger: &mut crate::logger::Logger) -> Option<f64>;
    
    /// Create a new convergence parameter for this campaign type
    fn create_controller_state(&self) -> Vec<Box<dyn crate::controllers::ControllerStateTrait>>;

    /// Perform one iteration of convergence, updating the next convergence parameter
    /// This method encapsulates the convergence logic for each campaign type
    /// 
    /// # Arguments
    /// * `previous_states` - Previous controller states (immutable slice of Boxes)
    /// * `next_states` - Next controller states to be updated (mutable slice of Boxes)
    /// * `campaign_stat` - Statistics from the current simulation run
    /// 
    /// # Returns
    /// `true` if pacing was changed, `false` if it remained the same
    fn next_controller_state(&self, previous_states: &[Box<dyn crate::controllers::ControllerStateTrait>], next_states: &mut [Box<dyn crate::controllers::ControllerStateTrait>], campaign_stat: &crate::simulationrun::CampaignStat) -> bool;

    /// Get a string representation of the campaign type and convergence strategy
    /// 
    /// # Arguments
    /// * `controller_states` - Controller states to include pacing information
    fn type_target_and_controller_state_string(&self, controller_states: &[&dyn crate::controllers::ControllerStateTrait]) -> String;
}

/// Trait for campaign bidding strategies
pub trait CampaignBidderTrait {
    /// Calculate the bid for this campaign given an impression, control variables slice, converge targets, and seller control factor
    /// Returns None if bid cannot be calculated (logs warning via logger)
    /// Must be implemented by specific bidder types
    fn get_bid(&self, value_to_campaign: f64, impression: &Impression, control_variables: &[f64], converge_targets: &Vec<Box<dyn CampaignTargetTrait>>, seller_control_factor: f64, logger: &mut Logger) -> Option<f64>;
    
    /// Get a string representation of the bidding type
    fn get_bidding_type(&self) -> String;
}

/// While in theory one can write any kind of campaign, in practice it is possible to break it down to key elements
/// that can operate separately: 
/// - what outcomes is the campaign looking to target
/// - what is the controller taking care of convergence to target for each target
/// - what is the bidding (pricing and optimization) strategy
/// CampaignGeneral is a generalized implementation of CampaignTrait. But it is possible to implement 
/// CampaignTrait from scratch when one needs more flexibility.

pub struct CampaignGeneral {
    pub campaign_id: usize,
    pub campaign_name: String,
    pub converge_targets: Vec<Box<dyn CampaignTargetTrait>>,
    pub converge_controllers: Vec<Box<dyn ControllerTrait>>,
    pub bidder: Box<dyn CampaignBidderTrait>,
}

impl CampaignTrait for CampaignGeneral {
    fn campaign_id(&self) -> usize {
        self.campaign_id
    }
    
    fn campaign_name(&self) -> &str {
        &self.campaign_name
    }
    
    fn get_bid(&self, impression: &Impression, controller_states: &[&dyn crate::controllers::ControllerStateTrait], seller_control_factor: f64, value_to_campaign: f64, logger: &mut crate::logger::Logger) -> Option<f64> {
        // Setup control variables in a static array
        let mut control_variables = [0.0; MAX_CONTROLLERS];
        for (i, (converge_controller, controller_state)) in self.converge_controllers.iter().zip(controller_states.iter()).enumerate() {
            control_variables[i] = converge_controller.get_control_variable(*controller_state);
        }
        
        // Delegate to the bidder
        self.bidder.get_bid(value_to_campaign, impression, &control_variables[..self.converge_controllers.len()], &self.converge_targets, seller_control_factor, logger)
    }
    
    fn next_controller_state(&self, previous_states: &[Box<dyn crate::controllers::ControllerStateTrait>], next_states: &mut [Box<dyn crate::controllers::ControllerStateTrait>], campaign_stat: &crate::simulationrun::CampaignStat) -> bool {
        let mut any_changed = false;
        for (index, (converge_target, converge_controller)) in self.converge_targets.iter().zip(self.converge_controllers.iter()).enumerate() {
            let (actual, target) = converge_target.get_actual_and_target(campaign_stat);
            let changed = converge_controller.next_controller_state(previous_states[index].as_ref(), next_states[index].as_mut(), actual, target);
            any_changed = any_changed || changed;
        }
        any_changed
    }
    
    fn type_target_and_controller_state_string(&self, controller_states: &[&dyn crate::controllers::ControllerStateTrait]) -> String {
        let mut parts = Vec::new();
        for (index, (converge_target, converge_controller)) in self.converge_targets.iter().zip(self.converge_controllers.iter()).enumerate() {
            parts.push(format!("T{}: {} ({})", 
                index + 1,
                converge_target.converge_target_string(),
                converge_controller.controller_string(controller_states[index])
            ));
        }
        format!("{} ({})", self.bidder.get_bidding_type(), parts.join(", "))
    }
    
    fn create_controller_state(&self) -> Vec<Box<dyn crate::controllers::ControllerStateTrait>> {
        self.converge_controllers.iter().map(|c| c.create_controller_state()).collect()
    }
}

