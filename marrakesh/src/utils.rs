use rand_distr::LogNormal;
use rand::Rng;

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


/// Sample a bid from a logistic distribution with given sigmoid parameters
/// 
/// Uses the inverse CDF method: x = μ + s * ln(u / (1 - u))
/// where μ is the location (sigmoid_offset) and s is the scale (sigmoid_scale)
/// 
/// # Arguments
/// * `sigmoid_offset` - Location parameter (mean) of the logistic distribution
/// * `sigmoid_scale` - Scale parameter of the logistic distribution
/// * `rng` - Random number generator
/// 
/// # Returns
/// A sampled bid value from Logistic(sigmoid_offset, sigmoid_scale)
pub fn sample_logistic_bid<R: Rng>(sigmoid_offset: f64, sigmoid_scale: f64, rng: &mut R) -> f64 {
    // Sample a uniform random variable in (0, 1) to avoid edge cases
    // Matches https://github.com/numpy/numpy/blob/main/numpy/random/src/distributions/distributions.c
    // rng.gen() returns [0, 1), so we clamp to (epsilon, 1 - epsilon) for numerical stability
    let epsilon = 1e-10;
    let u: f64 = rng.gen();
    let u_clamped = u.max(epsilon).min(1.0 - epsilon);
    // Use inverse CDF: x = μ + s * ln(u / (1 - u))
    sigmoid_offset + sigmoid_scale * (u_clamped / (1.0 - u_clamped)).ln()
}

