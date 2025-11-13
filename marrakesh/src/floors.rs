use rand::rngs::StdRng;
use rand_distr::Distribution;
use crate::utils::lognormal_dist;

/// Trait for generating floor CPM values
pub trait FloorGeneratorTrait {
    /// Generate a floor CPM value based on base_impression_ value
    /// 
    /// # Arguments
    /// * `base_impression_value` - Base impression value parameter
    /// * `rng` - Random number generator
    /// 
    /// # Returns
    /// Generated floor CPM value
    fn generate_floor(&self, base_impression_value: f64, rng: &mut StdRng) -> f64;
}

/// Floor generator that always returns a fixed value
pub struct FloorGeneratorFixed {
    pub value: f64,
}

impl FloorGeneratorFixed {
    /// Create a new FloorGeneratorFixed with the given value
    pub fn new(value: f64) -> Box<Self> {
        Box::new(Self { value })
    }
}

impl FloorGeneratorTrait for FloorGeneratorFixed {
    fn generate_floor(&self, _base_impression_value: f64, _rng: &mut StdRng) -> f64 {
        self.value
    }
}

/// Floor generator that uses a lognormal distribution centered around base_value
pub struct FloorGeneratorLogNormal {
    relative_to_impression_value: f64,
    stddev: f64,
}

impl FloorGeneratorLogNormal {
    /// Create a new FloorGeneratorLogNormal with the given standard deviation
    /// The distribution will be centered around base_value when generating floors
    pub fn new(relative_to_impression_value: f64, stddev: f64) -> Box<Self> {
        // Validate stddev by creating a distribution (will be recreated in generate_floor with actual base_value)
        Box::new(Self { relative_to_impression_value, stddev })
    }
}

impl FloorGeneratorTrait for FloorGeneratorLogNormal {
    fn generate_floor(&self, base_impression_value: f64, rng: &mut StdRng) -> f64 {
        // We get the base of the floor as scaling of base_impression_value
        // Create a lognormal distribution centered around base_value using the utility function
        let dist = lognormal_dist(base_impression_value * self.relative_to_impression_value, self.stddev);
        Distribution::sample(&dist, rng).max(0.0) // Ensure floor is non-negative
    }
}

