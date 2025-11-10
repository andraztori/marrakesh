use rand_distr::LogNormal;

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
pub fn lognormal_dist(mean: f64, stddev: f64) -> LogNormal<f64> {
    let (mu, sigma) = lognormal_from_mean_stddev(mean, stddev);
    LogNormal::new(mu, sigma).unwrap()
}

/// Proportional controller for adjusting campaign pacing based on target vs actual performance
pub struct ControllerProportional {
    tolerance_fraction: f64,      // Tolerance as a fraction of target (e.g., 0.005 = 0.5%)
    max_adjustment_factor: f64,   // Maximum adjustment factor (e.g., 0.2 = 20%)
    proportional_gain: f64,       // Proportional gain (e.g., 0.2 = 20% of error)
}

impl ControllerProportional {
    /// Create a new proportional controller with default parameters
    pub fn new() -> Self {
        Self {
            tolerance_fraction: 0.005,  // 0.5% tolerance
            max_adjustment_factor: 0.2,  // Max 20% adjustment
            proportional_gain: 0.2,      // 20% of error
        }
    }

    /// Adjust pacing based on target and actual values
    /// 
    /// # Arguments
    /// * `target` - Target value to achieve
    /// * `actual` - Actual value achieved
    /// * `current_pacing` - Current pacing value
    /// 
    /// # Returns
    /// Tuple of (new_pacing, changed) where changed indicates if pacing was modified
    pub fn adjust_pacing(&self, target: f64, actual: f64, current_pacing: f64) -> (f64, bool) {
        let tolerance = target * self.tolerance_fraction;
        
        if actual < target - tolerance {
            // Below target - increase pacing
            let error_ratio = (target - actual) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 + adjustment_factor);
            (new_pacing, true)
        } else if actual > target + tolerance {
            // Above target - decrease pacing
            let error_ratio = (actual - target) / target;
            let adjustment_factor = (error_ratio * self.proportional_gain).min(self.max_adjustment_factor);
            let new_pacing = current_pacing * (1.0 - adjustment_factor);
            (new_pacing, true)
        } else {
            // Within tolerance - keep constant
            (current_pacing, false)
        }
    }
}

