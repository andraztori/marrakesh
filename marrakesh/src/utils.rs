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
pub fn create_lognormal(mean: f64, stddev: f64) -> LogNormal<f64> {
    let (mu, sigma) = lognormal_from_mean_stddev(mean, stddev);
    LogNormal::new(mu, sigma).unwrap()
}

