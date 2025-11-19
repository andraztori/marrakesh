/// Sigmoid function implementation for win probability and marginal utility calculations
/// 
/// This struct represents a sigmoid function with scale, and offset paramters and having additional value
/// parameter that is used to calculate the marginal utility of spend.
/// It provides methods for calculating win probabilities, marginal utilities, and their inverses.

const EPSILON: f64 = 0.0001;

pub struct Sigmoid {
    pub offset: f64,
    pub scale: f64,
    pub value: f64,
}

impl Sigmoid {
    /// Create a new Sigmoid with the given parameters
    pub fn new(offset: f64, scale: f64, value: f64) -> Self {
        Self {
            offset,
            scale,
            value,
        }
    }

    /// Get the probability (sigmoid value) at x
    /// Returns: 1.0 / (1 + exp(-(x - offset) * scale))
    pub fn get_probability(&self, x: f64) -> f64 {
        1.0 / (1.0 + (-(x - self.offset) * self.scale).exp())
    }
    /*
    /// Binary search to find x such that get_probability(x) * x = y
    /// This is used to find the CPM that results in a specific spend amount
    pub fn bisect_spend_inverse(&self, y: f64) -> f64 {
        let mut min_x = 0.0;
        let mut max_x = 100.0;
        let mut a = -100.0;
        let mut steps = 0;

        while (a - y).abs() > 0.000001 {
            steps += 1;
            let x = (min_x + max_x) / 2.0;
            a = self.get_probability(x) * x;
            if a > y {
                max_x = x;
            } else {
                min_x = x;
            }

            if steps > 50 {
                panic!("Didn't find the inverse of {}", y);
            }
        }
        (min_x + max_x) / 2.0
    }*/
    /// Inverse of the sigmoid function
    /// Returns x such that get_probability(x) = y
    #[allow(dead_code)]
    pub fn inverse(&self, y: f64) -> f64 {
        let mut y_clamped = y;
        if y < EPSILON / 10.0 {
            y_clamped = EPSILON / 10.0;
        }
        if 1.0 - y <= EPSILON / 10.0 {
            y_clamped = 1.0 - EPSILON / 10.0;
        }
        (y_clamped.ln() - (1.0 - y_clamped).ln()) / self.scale + self.offset
    }
/*
    /// Numerical derivative of get_probability at x
    pub fn numeric_derivative(&self, x: f64) -> f64 {
        let e = 0.00001;
        let a1 = self.get_probability(x - e);
        let a2 = self.get_probability(x + e);
        (a2 - a1) / (2.0 * e)
    }

    /// Numerical derivative of get_probability(x) * x at x
    pub fn numeric_derivative_mul_x(&self, x: f64) -> f64 {
        let e = 0.00001;
        let x1 = x - e;
        let x2 = x + e;
        let a1 = self.get_probability(x1) * x1;
        let a2 = self.get_probability(x2) * x2;
        (a2 - a1) / (2.0 * e)
    }

    /// Marginal utility of spend using numerical calculation
    pub fn marginal_utility_of_spend_numeric(&self, x: f64) -> f64 {
        if x > 0.0001 {
            let wx = self.get_probability(x);
            let wdx = self.numeric_derivative(x);
            self.value * wdx / (wdx * x + wx)
        } else {
            0.0
        }
    }
    */
    /// Analytical formula for marginal utility M(x)
    /// M(x) = (value * scale * (1 - s(x))) / (scale * x * (1 - s(x)) + 1)
    /// where s(x) = get_probability(x)
    pub fn m(&self, x: f64) -> f64 {
        let s_val = self.get_probability(x);
        if (1.0 - s_val).abs() < 1e-15 {
            return 0.0;
        }

        let numerator = self.value * self.scale * (1.0 - s_val);
        let denominator = self.scale * x * (1.0 - s_val) + 1.0;

        if denominator.abs() < 1e-15 {
            // This case is unlikely under normal parameters but is good practice
            if numerator > 0.0 {
                return f64::INFINITY;
            } else {
                return f64::NEG_INFINITY;
            }
        }

        numerator / denominator
    }

    /// The derivative of M(x)
    /// M'(x) = -value * scale^2 * (1 - s(x)) / (scale * x * (1 - s(x)) + 1)^2
    pub fn m_prime(&self, x: f64) -> f64 {
        let s_val = self.get_probability(x);
        if (1.0 - s_val).abs() < 1e-15 {
            return 0.0;
        }

        let numerator = -self.value * (self.scale * self.scale) * (1.0 - s_val);
        let denominator = (self.scale * x * (1.0 - s_val) + 1.0).powi(2);

        if denominator.abs() < 1e-15 {
            return f64::NEG_INFINITY; // Derivative approaches -inf
        }

        numerator / denominator
    }


    /// Inverse of marginal_utility_of_spend using bisection method
    /// Finds x such that M(x) = y_target
    /// 
    /// # Arguments
    /// * `y_target` - Target value for m(x)
    /// * `min_x` - Minimum x value (will not be moved during bisection)
    /// 
    /// # Returns
    /// The x value where m(x) is closest to y_target, or None if no solution found
    pub fn marginal_utility_of_spend_inverse_numerical_2(&self, y_target: f64, mut min_x: f64) -> Option<f64> {
        let max_iterations = 100;
        let tolerance = 1e-6;
        
        // Store the original min_x (will not be modified)
        let original_min_x = min_x;
        
        // Initial search bounds (use separate variable for bisection that can be modified)
        let mut max_x = 1000.0;
        
        if min_x > 1.0 / y_target * self.value {
            return Some(1.0 / y_target * self.value); // this is below floor
        }
        

        // First, try to find an interval where f(min_x) and f(max_x) have opposite signs
        // f(x) = m(x) - y_target, we want f(x) = 0
        // Evaluate at bounds
        let f_min = self.m(min_x) - y_target;
        let f_max = self.m(max_x) - y_target;
        let r = 1.0 / y_target * self.value;

        // If both are negative, the maximum is at min_bid
        if f_min < 0.0 && f_max < 0.0 {
            return Some(min_x);
        }


        // If both are positive, we need to expand the search
        if f_min > 0.0 && f_max > 0.0 {
            // Try expanding max_x
            let mut expanded_max = max_x * 2.0;
            let mut f_expanded = self.m(expanded_max) - y_target;
            let mut attempts = 0;
            while f_min * f_expanded > 0.0 && attempts < 10 {
                expanded_max *= 2.0;
                f_expanded = self.m(expanded_max) - y_target;
                attempts += 1;
            }
            if f_min * f_expanded <= 0.0 {
                max_x = expanded_max;
            } else {
                // No sign change found, return None or use grid search fallback
                /*println!("No sign change found, using grid search fallback");
                println!("y_target: {:.4}", y_target);
                println!("min_x: {:.4}", original_min_x);
                println!("max_x: {:.4}", max_x);
                println!("f_min: {:.4}", f_min);
                println!("sigmoid: offset={:.4}, scale={:.4}, value={:.4}", self.offset, self.scale, self.value);
                */
                //return Some(min_x);
                return None;
                //return self.marginal_utility_of_spend_inverse_numerical(y_target);
            }
        }
        
        // Perform bisection (ensuring we never go below original_min_x)
        for _i in 0..max_iterations {
            let mid_x = (min_x + max_x) / 2.0;
            let f_mid = self.m(mid_x) - y_target;
            
            // Check for convergence
            if f_mid.abs() < tolerance || (max_x - min_x) < tolerance {
                return Some(mid_x);
            }
            
            // Determine which half contains the root
            let f_min = self.m(min_x) - y_target;
            if f_min * f_mid < 0.0 {
                // Root is in [min_x, mid_x]
                max_x = mid_x;
            } else {
                // Root is in [mid_x, max_x]
                min_x = mid_x;
            }
        }
        
        // Return the midpoint of the final interval, ensuring it's not below original_min_x
        Some(((min_x + max_x) / 2.0))
    }

    /// Find the bid that maximizes margin = P(win) * (full_price - bid)
    /// Uses bisection to find where the derivative of margin is zero
    /// 
    /// # Arguments
    /// * `full_price` - Maximum price we're willing to pay (also used as max_bid)
    /// * `min_bid` - Minimum bid (typically floor price)
    /// 
    /// # Returns
    /// The bid that maximizes margin, or None if max_bid <= min_bid
    pub fn max_margin_bid_bisection(&self, full_price: f64, min_bid: f64) -> Option<f64> {
        const MAX_ITERATIONS: usize = 100;
        const TOLERANCE: f64 = 1e-6;
        
        let max_bid = full_price;
        
        if max_bid <= min_bid {
            // Can't afford even the floor
            return None;
        }
        
        // The margin function is: margin(bid) = prob(bid) * (full_price - bid)
        // To find the maximum, we need to find where the derivative is zero:
        // margin'(bid) = prob'(bid) * (full_price - bid) - prob(bid) = 0
        // 
        // The derivative of sigmoid probability is:
        // prob'(bid) = scale * prob(bid) * (1 - prob(bid))
        //
        // So: margin'(bid) = scale * prob(bid) * (1 - prob(bid)) * (full_price - bid) - prob(bid)
        //                 = prob(bid) * [scale * (1 - prob(bid)) * (full_price - bid) - 1]
        //
        // We'll find the root of: f(bid) = scale * (1 - prob(bid)) * (full_price - bid) - 1
        
        // Helper function to compute the derivative of margin
        let margin_derivative = |bid: f64| -> f64 {
            let prob = self.get_probability(bid);
            self.scale * (1.0 - prob) * (full_price - bid) - 1.0
        };
        
        // Check if we have opposite signs at the bounds
        let f_min = margin_derivative(min_bid);
        let f_max = margin_derivative(max_bid);
        // If both are positive, the maximum is at max_bid
        if f_min > 0.0 && f_max > 0.0 {
            return Some(max_bid);
        }
        
        // If both are negative, the maximum is at min_bid
        if f_min < 0.0 && f_max < 0.0 {
            return Some(min_bid);
        }
        
        // If signs are opposite, use bisection to find the root
        let mut low = min_bid;
        let mut high = max_bid;
        
        for _i in 0..MAX_ITERATIONS {
            let mid = (low + high) / 2.0;
            let f_mid = margin_derivative(mid);
            
            // Check for convergence
            if f_mid.abs() < TOLERANCE || (high - low) < TOLERANCE {
                return Some(mid);
            }
            
            // Determine which half contains the root
            let f_low = margin_derivative(low);
            if f_low * f_mid < 0.0 {
                // Root is in [low, mid]
                high = mid;
            } else {
                // Root is in [mid, high]
                low = mid;
            }
        }
        
        // Return the midpoint of the final interval
        Some((low + high) / 2.0)
    }

    // The problem with this function is that it is not robust at  - better to use bisection
    #[allow(dead_code)]
    pub fn marginal_utility_of_spend_inverse(&self, y_target: f64) -> Option<f64> {
        let max_iterations = 100;
        let tolerance = 1e-6;
//        let initial_guess = 10.0;
        let initial_guess = self.offset;  // This is a good starting point making things really stable
        let mut x = initial_guess;
        //println!("y_target {}", y_target);
        for _i in 0..max_iterations {
            // Calculate the value of M(x) and its derivative at the current x
            let m_val = self.m(x);
            let m_prime_val = self.m_prime(x);
            //println!("x {}, m_val {}, m_prime_val {}", x,  m_val, m_prime_val);
            // The function whose root we are finding is f(x) = M(x) - y_target
            let f_x = m_val - y_target;

            // Avoid division by zero if the derivative is flat
            if m_prime_val.abs() < 1e-15 {
                /*eprintln!(
                    "Warning: Derivative is close to zero at x={:.2}. Method cannot proceed. \
                    y_target={:.2}, m_val={:.2}, m_prime_val={:.2}, Sigmoid parameters: scale={:.2}, offset={:.2}, value={:.2}",
                    x, y_target, m_val, m_prime_val, self.scale, self.offset, self.value
                );*/

                // We handle the extreme case by simply assuming this happen with large scales &  values on the extreme sides
                // What we do is we check what our marginal utility is at offset - if values are on extreme, we 
                // generally want to either bid to win or bid to lose
                //return None;
                return self.marginal_utility_of_spend_inverse_numerical_2(y_target, 0.0);
                /*
                if self.m(self.offset) < y_target {
                    return Some(self.inverse(0.001));
                } else {
                    return Some(self.inverse(0.999));
                }*/
            }

            // Newton-Raphson update step
            let mut x_new = x - f_x / m_prime_val;

            // Ensure x remains positive as per the problem constraint
            if x_new <= 0.0 {
                // If we get a non-positive x, we can try halving the step
                // This is a simple modification to improve robustness
                x_new = x / 2.0;
            }

            // Check for convergence
            if (x_new - x).abs() < tolerance {
                return Some(x_new);
            }

            x = x_new;
        }

        // If we didn't converge, return the last value anyway
        Some(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_probability() {
        let sigmoid = Sigmoid::new(8.0, 0.5, 1.0);
        let prob = sigmoid.get_probability(8.0);
        // At offset, probability should be 0.5
        assert!((prob - 0.5).abs() < 0.01);
    }

    // #[test]
    // fn test_inverse() {
    //     let sigmoid = Sigmoid::new(8.0, 0.5, 1.0);
    //     let prob = sigmoid.get_probability(8.0);
    //     let x = sigmoid.inverse(prob);
    //     // Should recover the original x value approximately
    //     assert!((x - 8.0).abs() < 0.1);
    // }

    // #[test]
    // fn test_marginal_utility() {
    //     let sigmoid = Sigmoid::new(8.0, 0.5, 1.0);
    //     let mu = sigmoid.marginal_utility_of_spend(10.0);
    //     // Should return a positive value
    //     assert!(mu >= 0.0);
    // }
}

