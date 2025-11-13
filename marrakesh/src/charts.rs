use rand::{rngs::StdRng, SeedableRng};
use rand_distr::Distribution;
use crate::impressions::Impression;
use crate::competition::{CompetitionGeneratorParametrizedLogNormal, CompetitionGeneratorTrait};
use crate::floors::{FloorGeneratorLogNormal, FloorGeneratorTrait};
use crate::campaigns::MAX_CAMPAIGNS;
use crate::utils::lognormal_dist;
use plotters::prelude::*;
use std::fs;

/// Generate all impressions once
fn generate_all_impressions() -> Vec<Impression> {
    // Initialize generators
    let competition_generator = CompetitionGeneratorParametrizedLogNormal::new(10.0);
    let floor_generator = FloorGeneratorLogNormal::new(0.2, 2.0);
    let base_impression_value_dist = lognormal_dist(10.0, 3.0);
    let value_to_campaign_multiplier_dist = lognormal_dist(1.0, 0.2);
    
    // Create a seeded RNG for reproducibility
    let mut rng = StdRng::seed_from_u64(42);
    
    const NUM_SAMPLES: usize = 10000;
    
    let mut impressions = Vec::with_capacity(NUM_SAMPLES);
    
    // Generate all impressions in one pass
    for _ in 0..NUM_SAMPLES {
        // Generate base impression value
        let base_impression_value = Distribution::sample(&base_impression_value_dist, &mut rng);
        
        // Generate competition
        let competition = competition_generator.generate_competition(&mut rng);
        
        // Generate floor
        let floor_cpm = floor_generator.generate_floor(base_impression_value, &mut rng);
        
        // Generate value_to_campaign_id array
        let mut value_to_campaign_id = [0.0; MAX_CAMPAIGNS];
        for i in 0..MAX_CAMPAIGNS {
            let multiplier = Distribution::sample(&value_to_campaign_multiplier_dist, &mut rng);
            value_to_campaign_id[i] = base_impression_value * multiplier;
        }
        
        impressions.push(Impression {
            seller_id: 0,
            competition,
            floor_cpm,
            value_to_campaign_id,
        });
    }
    
    impressions
}

/// Generate all histograms from pre-generated impressions
pub fn generate_all_histograms() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating all impressions...");
    let impressions = generate_all_impressions();
    
    println!("Creating histograms...");
    
    generate_floor_and_bid_combined_histogram_from_impressions(&impressions)?;
    generate_base_impression_value_histogram_from_impressions(&impressions)?;
    generate_win_rate_sigmoid_combined_histogram_from_impressions(&impressions)?;
    
    println!("All histograms generated successfully!");
    Ok(())
}

/// Generate combined histogram of floor_cpm and competing bid_cpm on a single chart
fn generate_floor_and_bid_combined_histogram_from_impressions(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let bid_cpms: Vec<f64> = impressions.iter()
        .filter_map(|imp| imp.competition.as_ref().map(|c| c.bid_cpm))
        .collect();
    let floor_cpms: Vec<f64> = impressions.iter().map(|imp| imp.floor_cpm).collect();
    
    create_dual_histogram_combined(
        &floor_cpms,
        &bid_cpms,
        "floor_and_bid_combined_histogram.png",
        "Floor CPM and Competing Bid CPM",
        "CPM",
        "Floor",
        "Competing Bid",
        RGBColor(50, 200, 50),    // Green for floor
        RGBColor(50, 100, 200),   // Blue for competing bid
    )
}

/// Helper function to create a histogram with two series on the same chart
/// This is a special case of create_side_by_side_histogram with empty right side
fn create_dual_histogram_combined(
    values1: &[f64],
    values2: &[f64],
    filename: &str,
    caption: &str,
    x_label: &str,
    label1: &str,
    label2: &str,
    color1: RGBColor,
    color2: RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    // Call create_side_by_side_histogram with empty right side arrays
    create_side_by_side_histogram(
        values1,
        values2,
        &[],  // Empty right side
        &[],  // Empty right side
        filename,
        caption,
        caption,  // Right caption (unused when right side is empty)
        x_label,
        x_label,  // Right x_label (unused when right side is empty)
        label1,
        label2,
        color1,
        color2,
        color1,  // Right colors (unused when right side is empty)
        color2,
    )
}

/// Helper function to create and save a histogram from a vector of values
fn create_histogram(
    values: &[f64],
    filename: &str,
    caption: &str,
    x_label: &str,
    color: RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    if values.is_empty() {
        return Err("No values to create histogram".into());
    }
    
    // Find min and max for histogram bounds
    let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Create histogram data
    let num_buckets = 100;
    let bucket_width = (max_val - min_val) / num_buckets as f64;
    let mut histogram = vec![0u32; num_buckets];
    
    for &value in values {
        let bucket_index = ((value - min_val) / bucket_width) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1); // Clamp to valid range
        histogram[bucket_index] += 1;
    }
    
    // Find max count for Y-axis scaling
    let max_count = *histogram.iter().max().unwrap_or(&1) as u32;
    
    // Ensure charts directory exists
    fs::create_dir_all("charts")?;
    
    // Create the plot
    let filepath = format!("charts/{}", filename);
    let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    
    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 20).into_font())
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(min_val..max_val, 0u32..max_count)?;
    
    chart.configure_mesh()
        .x_desc(x_label)
        .y_desc("Frequency")
        .draw()?;
    
    // Draw histogram bars as rectangles with a label for the legend
    let rectangles: Vec<_> = histogram.iter().enumerate()
        .filter_map(|(i, &count)| {
            if count > 0 {
                let bucket_start = min_val + (i as f64 * bucket_width);
                let bucket_end = min_val + ((i + 1) as f64 * bucket_width);
                Some(Rectangle::new(
                    [(bucket_start, 0), (bucket_end, count)],
                    color.filled(),
                ))
            } else {
                None
            }
        })
        .collect();
    
    chart.draw_series(rectangles.into_iter())?
        .label("Data")
        .legend(move |(x, y)| Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], color.filled()));
    
    // Add legend
    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;
    
    root.present()?;
    
    println!("Histogram saved to charts/{}", filename);
    println!("Min {}: {:.2}, Max {}: {:.2}", x_label, min_val, x_label, max_val);
    println!("Total values: {}", values.len());
    
    Ok(())
}

/// Generate histogram of base_impression_value from pre-generated impressions
/// Uses the first campaign value which is base_impression_value * multiplier
/// Since multiplier has mean 1.0, this approximates the base_impression_value distribution
fn generate_base_impression_value_histogram_from_impressions(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    // value_to_campaign_id[0] = base_impression_value * multiplier
    // Since multiplier has mean 1.0, this closely approximates base_impression_value
    let base_values: Vec<f64> = impressions.iter()
        .map(|imp| imp.value_to_campaign_id[0])
        .collect();
    create_histogram(
        &base_values,
        "base_impression_value_histogram.png",
        "Base Impression Value",
        "Base Impression Value",
        RED,
    )
}

/// Generate combined histogram of win_rate sigmoid offset and scale (prediction and actual) side-by-side
fn generate_win_rate_sigmoid_combined_histogram_from_impressions(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let prediction_offset_values: Vec<f64> = impressions.iter()
        .filter_map(|imp| imp.competition.as_ref().map(|c| c.win_rate_prediction_sigmoid_offset))
        .collect();
    let actual_offset_values: Vec<f64> = impressions.iter()
        .filter_map(|imp| imp.competition.as_ref().map(|c| c.win_rate_actual_sigmoid_offset))
        .collect();
    let prediction_scale_values: Vec<f64> = impressions.iter()
        .filter_map(|imp| imp.competition.as_ref().map(|c| c.win_rate_prediction_sigmoid_scale))
        .collect();
    let actual_scale_values: Vec<f64> = impressions.iter()
        .filter_map(|imp| imp.competition.as_ref().map(|c| c.win_rate_actual_sigmoid_scale))
        .collect();
    
    create_side_by_side_histogram(
        &actual_offset_values,      // Left: first series (actual) - will be bars
        &prediction_offset_values,  // Left: second series (prediction) - will be lines
        &actual_scale_values,       // Right: first series (actual) - will be bars
        &prediction_scale_values,   // Right: second series (prediction) - will be lines
        "win_rate_sigmoid_combined_histogram.png",
        "Win Rate Probability Offset",
        "Win Rate Probability Scale",
        "Offset",
        "Scale",
        "Actual",
        "Prediction",
        RGBColor(200, 50, 50),    // Red for actual
        RGBColor(0, 100, 200),    // Blue for prediction
        RGBColor(200, 50, 50),    // Red for actual
        RGBColor(0, 100, 200),    // Blue for prediction
    )
}

/// Helper function to create side-by-side histograms
/// If right side arrays are empty, creates a single chart instead
fn create_side_by_side_histogram(
    values1_left: &[f64],
    values2_left: &[f64],
    values1_right: &[f64],
    values2_right: &[f64],
    filename: &str,
    caption_left: &str,
    caption_right: &str,
    x_label_left: &str,
    x_label_right: &str,
    label1: &str,
    label2: &str,
    color1: RGBColor,
    color2: RGBColor,
    color3: RGBColor,
    color4: RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_single_chart = values1_right.is_empty() && values2_right.is_empty();
    
    if values1_left.is_empty() && values2_left.is_empty() {
        return Err("No values to create histogram".into());
    }
    
    if !is_single_chart && values1_right.is_empty() && values2_right.is_empty() {
        return Err("No values to create histogram".into());
    }
    
    // Find min and max for left histogram (offset)
    let min_left = values1_left.iter().chain(values2_left.iter()).fold(f64::INFINITY, |a, &b| a.min(b));
    let max_left = values1_left.iter().chain(values2_left.iter()).fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Find min and max for right histogram (scale)
    let min_right = values1_right.iter().chain(values2_right.iter()).fold(f64::INFINITY, |a, &b| a.min(b));
    let max_right = values1_right.iter().chain(values2_right.iter()).fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Create histogram data for left (offset)
    let num_buckets = 100;
    let bucket_width_left = (max_left - min_left) / num_buckets as f64;
    let mut histogram1_left = vec![0u32; num_buckets];
    let mut histogram2_left = vec![0u32; num_buckets];
    
    for &value in values1_left {
        let bucket_index = ((value - min_left) / bucket_width_left) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1);
        histogram1_left[bucket_index] += 1;
    }
    
    for &value in values2_left {
        let bucket_index = ((value - min_left) / bucket_width_left) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1);
        histogram2_left[bucket_index] += 1;
    }
    
    // Create histogram data for right (scale)
    let bucket_width_right = (max_right - min_right) / num_buckets as f64;
    let mut histogram1_right = vec![0u32; num_buckets];
    let mut histogram2_right = vec![0u32; num_buckets];
    
    for &value in values1_right {
        let bucket_index = ((value - min_right) / bucket_width_right) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1);
        histogram1_right[bucket_index] += 1;
    }
    
    for &value in values2_right {
        let bucket_index = ((value - min_right) / bucket_width_right) as usize;
        let bucket_index = bucket_index.min(num_buckets - 1);
        histogram2_right[bucket_index] += 1;
    }
    
    // Calculate means for vertical lines
    let mean1_left = if !values1_left.is_empty() {
        values1_left.iter().sum::<f64>() / values1_left.len() as f64
    } else {
        0.0
    };
    let mean2_left = if !values2_left.is_empty() {
        values2_left.iter().sum::<f64>() / values2_left.len() as f64
    } else {
        0.0
    };
    let mean1_right = if !values1_right.is_empty() {
        values1_right.iter().sum::<f64>() / values1_right.len() as f64
    } else {
        0.0
    };
    let mean2_right = if !values2_right.is_empty() {
        values2_right.iter().sum::<f64>() / values2_right.len() as f64
    } else {
        0.0
    };
    
    // Find max count for Y-axis scaling
    let max_count_left = histogram1_left.iter().chain(histogram2_left.iter()).max().copied().unwrap_or(1) as u32;
    let max_count_right = histogram1_right.iter().chain(histogram2_right.iter()).max().copied().unwrap_or(1) as u32;
    let max_count = max_count_left.max(max_count_right);
    
    // Ensure charts directory exists
    fs::create_dir_all("charts")?;
    
    // Create the plot - single chart or side-by-side layout
    let filepath = format!("charts/{}", filename);
    
    if is_single_chart {
        // Single chart mode
        let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let mut chart = ChartBuilder::on(&root)
            .caption(caption_left, ("sans-serif", 20).into_font())
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(min_left..max_left, 0u32..max_count)?;
        
        chart.configure_mesh()
            .x_desc(x_label_left)
            .y_desc("Frequency")
            .draw()?;
        
        // Draw first dataset as bars
        let rectangles1: Vec<_> = histogram1_left.iter().enumerate()
            .filter_map(|(i, &count)| {
                if count > 0 {
                    let bucket_start = min_left + (i as f64 * bucket_width_left);
                    let bucket_end = min_left + ((i + 1) as f64 * bucket_width_left);
                    Some(Rectangle::new(
                        [(bucket_start, 0), (bucket_end, count)],
                        color1.filled(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        
        chart.draw_series(rectangles1.into_iter())?
            .label(label1)
            .legend(move |(x, y)| Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], color1.filled()));
        
        // Convert second dataset to line graph points
        let line_points2: Vec<(f64, u32)> = histogram2_left.iter().enumerate()
            .map(|(i, &count)| {
                let bucket_center = min_left + (i as f64 + 0.5) * bucket_width_left;
                (bucket_center, count)
            })
            .collect();
        
        chart.draw_series(LineSeries::new(
            line_points2.iter().map(|&(x, y)| (x, y)),
            color2.stroke_width(2),
        ))?
            .label(label2)
            .legend(move |(x, y)| PathElement::new(vec![(x - 5, y), (x + 5, y)], color2.stroke_width(2)));
        
        // Draw vertical mean lines in black
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(mean1_left, 0), (mean1_left, max_count)],
            BLACK.stroke_width(2),
        )))?;
        
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(mean2_left, 0), (mean2_left, max_count)],
            BLACK.stroke_width(2),
        )))?;
        
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
        
        root.present()?;
        
        println!("Histogram saved to charts/{}", filename);
        println!("Min {}: {:.2}, Max {}: {:.2}", x_label_left, min_left, x_label_left, max_left);
        println!("Total values - {}: {}, {}: {}", label1, values1_left.len(), label2, values2_left.len());
        println!("Means - {}: {:.2}, {}: {:.2}", label1, mean1_left, label2, mean2_left);
        
        return Ok(());
    }
    
    // Side-by-side mode
    let root = BitMapBackend::new(&filepath, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    let (left_area, right_area) = root.split_horizontally(800);
    
    // Draw left chart
    let mut chart_left = ChartBuilder::on(&left_area)
        .caption(caption_left, ("sans-serif", 20).into_font())
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(min_left..max_left, 0u32..max_count)?;
    
    chart_left.configure_mesh()
        .x_desc(x_label_left)
        .y_desc("Frequency")
        .draw()?;
    
    // Draw left chart: first dataset (actual) as bars, second dataset (prediction) as lines
    let rectangles1_left: Vec<_> = histogram1_left.iter().enumerate()
        .filter_map(|(i, &count)| {
            if count > 0 {
                let bucket_start = min_left + (i as f64 * bucket_width_left);
                let bucket_end = min_left + ((i + 1) as f64 * bucket_width_left);
                Some(Rectangle::new(
                    [(bucket_start, 0), (bucket_end, count)],
                    color1.filled(),
                ))
            } else {
                None
            }
        })
        .collect();
    
    chart_left.draw_series(rectangles1_left.into_iter())?
        .label(label1)  // Actual (first in legend)
        .legend(move |(x, y)| Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], color1.filled()));
    
    // Convert second dataset (prediction) to line graph points
    let line_points2_left: Vec<(f64, u32)> = histogram2_left.iter().enumerate()
        .map(|(i, &count)| {
            let bucket_center = min_left + (i as f64 + 0.5) * bucket_width_left;
            (bucket_center, count)
        })
        .collect();
    
    chart_left.draw_series(LineSeries::new(
        line_points2_left.iter().map(|&(x, y)| (x, y)),
        color2.stroke_width(2),
    ))?
        .label(label2)  // Prediction (second in legend)
        .legend(move |(x, y)| PathElement::new(vec![(x - 5, y), (x + 5, y)], color2.stroke_width(2)));
    
    // Draw vertical mean lines for left chart in black
    chart_left.draw_series(std::iter::once(PathElement::new(
        vec![(mean1_left, 0), (mean1_left, max_count)],
        BLACK.stroke_width(2),
    )))?;
    
    chart_left.draw_series(std::iter::once(PathElement::new(
        vec![(mean2_left, 0), (mean2_left, max_count)],
        BLACK.stroke_width(2),
    )))?;
    
    chart_left.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;
    
    // Draw right chart
    let mut chart_right = ChartBuilder::on(&right_area)
            .caption(caption_right, ("sans-serif", 20).into_font())
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(min_right..max_right, 0u32..max_count)?;
        
        chart_right.configure_mesh()
            .x_desc(x_label_right)
            .y_desc("Frequency")
            .draw()?;
        
        // Draw right chart: first dataset (actual) as bars, second dataset (prediction) as lines
        let rectangles1_right: Vec<_> = histogram1_right.iter().enumerate()
            .filter_map(|(i, &count)| {
                if count > 0 {
                    let bucket_start = min_right + (i as f64 * bucket_width_right);
                    let bucket_end = min_right + ((i + 1) as f64 * bucket_width_right);
                    Some(Rectangle::new(
                        [(bucket_start, 0), (bucket_end, count)],
                        color3.filled(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        
        chart_right.draw_series(rectangles1_right.into_iter())?
            .label(label1)  // Actual (first in legend)
            .legend(move |(x, y)| Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], color3.filled()));
        
        // Convert second dataset (prediction) to line graph points
        let line_points2_right: Vec<(f64, u32)> = histogram2_right.iter().enumerate()
            .map(|(i, &count)| {
                let bucket_center = min_right + (i as f64 + 0.5) * bucket_width_right;
                (bucket_center, count)
            })
            .collect();
        
        chart_right.draw_series(LineSeries::new(
            line_points2_right.iter().map(|&(x, y)| (x, y)),
            color4.stroke_width(2),
        ))?
            .label(label2)  // Prediction (second in legend)
            .legend(move |(x, y)| PathElement::new(vec![(x - 5, y), (x + 5, y)], color4.stroke_width(2)));
        
        // Draw vertical mean lines for right chart in black
        chart_right.draw_series(std::iter::once(PathElement::new(
            vec![(mean1_right, 0), (mean1_right, max_count)],
            BLACK.stroke_width(2),
        )))?;
        
        chart_right.draw_series(std::iter::once(PathElement::new(
            vec![(mean2_right, 0), (mean2_right, max_count)],
            BLACK.stroke_width(2),
        )))?;
        
    chart_right.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;
    
    root.present()?;
    
    println!("Histogram saved to charts/{}", filename);
    println!("Left - Min: {:.2}, Max: {:.2}", min_left, max_left);
    println!("Right - Min: {:.2}, Max: {:.2}", min_right, max_right);
    println!("Total values - {} (left): {}, {} (left): {}, {} (right): {}, {} (right): {}", 
             label1, values1_left.len(), label2, values2_left.len(), label1, values1_right.len(), label2, values2_right.len());
    
    Ok(())
}

