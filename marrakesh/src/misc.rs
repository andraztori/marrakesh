use rand::{rngs::StdRng, SeedableRng};
use crate::impressions::ImpressionCompetitionGenerator;
use plotters::prelude::*;

/// Generate 10000 impressions and create a histogram of bid_cpm values
/// 
/// Initializes ImpressionCompetitionGenerator with value_base of 10.0,
/// generates 10000 impressions, creates a histogram with 100 buckets,
/// and renders it to an image file using plotters.
pub fn generate_bid_histogram() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize generator with value_base of 10.0
    let generator = ImpressionCompetitionGenerator::new(10.0);
    
    // Create a seeded RNG for reproducibility
    let mut rng = StdRng::seed_from_u64(42);
    
    // Generate 10000 impressions and collect bid_cpm values
    let mut bid_cpms = Vec::with_capacity(10000);
    for _ in 0..10000 {
        let competition = generator.generate_competition(&mut rng);
        bid_cpms.push(competition.bid_cpm);
    }
    
    // Find min and max for histogram bounds
    let min_bid = bid_cpms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_bid = bid_cpms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Create histogram data
    let num_buckets = 100;
    let bucket_width = (max_bid - min_bid) / num_buckets as f64;
    let mut histogram = vec![0u32; num_buckets];
    
    for &bid_cpm in &bid_cpms {
        let bucket_index = ((bid_cpm - min_bid) / bucket_width) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1); // Clamp to valid range
        histogram[bucket_index] += 1;
    }
    
    // Find max count for Y-axis scaling
    let max_count = *histogram.iter().max().unwrap_or(&1) as u32;
    
    // Create the plot
    let root = BitMapBackend::new("bid_histogram.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    
    let mut chart = ChartBuilder::on(&root)
        .caption("Bid CPM Histogram", ("sans-serif", 40).into_font())
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(min_bid..max_bid, 0u32..max_count)?;
    
    chart.configure_mesh()
        .x_desc("Bid CPM")
        .y_desc("Frequency")
        .draw()?;
    
    // Draw histogram bars as rectangles
    for (i, &count) in histogram.iter().enumerate() {
        if count > 0 {
            let bucket_start = min_bid + (i as f64 * bucket_width);
            let bucket_end = min_bid + ((i + 1) as f64 * bucket_width);
            
            chart.draw_series(std::iter::once(Rectangle::new(
                [(bucket_start, 0), (bucket_end, count)],
                BLUE.filled(),
            )))?;
        }
    }
    
    root.present()?;
    
    println!("Histogram saved to bid_cpm_histogram.png");
    println!("Min bid_cpm: {:.2}, Max bid_cpm: {:.2}", min_bid, max_bid);
    println!("Total impressions: {}", bid_cpms.len());
    
    Ok(())
}

