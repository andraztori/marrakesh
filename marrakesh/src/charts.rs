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
        let competition = competition_generator.generate_competition(base_impression_value, &mut rng);
        
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

/// Main function to generate all histograms
pub fn generate_all_histograms() -> Result<(), Box<dyn std::error::Error>> {
    // Create charts directory if it doesn't exist
    fs::create_dir_all("charts")?;
    
    // Generate all impression data once
    let impressions = generate_all_impressions();
    
    // Generate all histograms from the same data
    generate_bid_histogram(&impressions)?;
    generate_floor_histogram(&impressions)?;
    generate_base_impression_value_histogram(&impressions)?;
    generate_win_rate_probability_histograms(&impressions)?;
    generate_floors_and_competing_bid_histogram(&impressions)?;
    
    Ok(())
}

/// Generate histogram for competing bids
fn generate_bid_histogram(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let mut bids = Vec::new();
    for impression in impressions {
        if let Some(ref competition) = impression.competition {
            bids.push(competition.bid_cpm);
        }
    }
    
    if bids.is_empty() {
        return Err("No competing bids found in impressions".into());
    }
    
    create_single_histogram(
        &bids,
        "Competing Bid Distribution",
        "charts/competing_bid_histogram.png",
        "Competing Bid (CPM)",
        &BLUE,
    )?;
    
    Ok(())
}

/// Generate histogram for floor CPM values
fn generate_floor_histogram(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let floors: Vec<f64> = impressions.iter().map(|imp| imp.floor_cpm).collect();
    
    create_single_histogram(
        &floors,
        "Floor CPM Distribution",
        "charts/floor_cpm_histogram.png",
        "Floor CPM",
        &RED,
    )?;
    
    Ok(())
}

/// Generate histogram for base impression values
fn generate_base_impression_value_histogram(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = impressions.iter()
        .map(|imp| imp.value_to_campaign_id[0])
        .collect();
    
    create_single_histogram(
        &values,
        "Base Impression Value Distribution",
        "charts/base_impression_value_histogram.png",
        "Base Impression Value",
        &GREEN,
    )?;
    
    Ok(())
}

/// Generate side-by-side histograms for win rate probability offset and scale
fn generate_win_rate_probability_histograms(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let mut prediction_offsets = Vec::new();
    let mut actual_offsets = Vec::new();
    let mut prediction_scales = Vec::new();
    let mut actual_scales = Vec::new();
    
    for impression in impressions {
        if let Some(ref competition) = impression.competition {
            prediction_offsets.push(competition.win_rate_prediction_sigmoid_offset);
            actual_offsets.push(competition.win_rate_actual_sigmoid_offset);
            prediction_scales.push(competition.win_rate_prediction_sigmoid_scale);
            actual_scales.push(competition.win_rate_actual_sigmoid_scale);
        }
    }
    
    // Create side-by-side histograms for offset
    create_side_by_side_histogram(
        &actual_offsets,
        &prediction_offsets,
        "Actual",
        "Prediction",
        "Win Rate Probability Offset",
        "charts/win_rate_offset_histogram.png",
        "Sigmoid Offset",
        DrawingStyle::Bars,
        DrawingStyle::Line,
        &BLUE,
        &RED,
    )?;
    
    // Create side-by-side histograms for scale
    create_side_by_side_histogram(
        &actual_scales,
        &prediction_scales,
        "Actual",
        "Prediction",
        "Win Rate Probability Scale",
        "charts/win_rate_scale_histogram.png",
        "Sigmoid Scale",
        DrawingStyle::Bars,
        DrawingStyle::Line,
        &BLUE,
        &RED,
    )?;
    
    Ok(())
}

/// Generate combined histogram for floors and competing bids
fn generate_floors_and_competing_bid_histogram(impressions: &[Impression]) -> Result<(), Box<dyn std::error::Error>> {
    let mut floors = Vec::new();
    let mut bids = Vec::new();
    
    for impression in impressions {
        floors.push(impression.floor_cpm);
        if let Some(ref competition) = impression.competition {
            bids.push(competition.bid_cpm);
        }
    }
    
    create_side_by_side_histogram(
        &floors,
        &bids,
        "Floor",
        "Competing Bid",
        "Floor and Competing Bid Distribution",
        "charts/floor_and_competing_bid_histogram.png",
        "CPM",
        DrawingStyle::Bars,
        DrawingStyle::Line,
        &GREEN,
        &BLUE,
    )?;
    
    Ok(())
}

#[derive(Clone, Copy)]
enum DrawingStyle {
    Bars,
    Line,
}

/// Helper function to create a side-by-side histogram with two datasets
/// Can render each dataset as either bars or lines
fn create_side_by_side_histogram(
    values1: &[f64],
    values2: &[f64],
    label1: &str,
    label2: &str,
    title: &str,
    filename: &str,
    x_label: &str,
    style1: DrawingStyle,
    style2: DrawingStyle,
    color1: &RGBColor,
    color2: &RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    if values1.is_empty() || values2.is_empty() {
        return Err(format!("Cannot create histogram: one or both datasets are empty").into());
    }
    
    // Calculate statistics for both datasets
    let min1 = values1.iter().cloned().fold(f64::INFINITY, f64::min);
    let max1 = values1.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean1 = values1.iter().sum::<f64>() / values1.len() as f64;
    
    let min2 = values2.iter().cloned().fold(f64::INFINITY, f64::min);
    let max2 = values2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean2 = values2.iter().sum::<f64>() / values2.len() as f64;
    
    // Use the overall min and max for bin range
    let overall_min = min1.min(min2);
    let overall_max = max1.max(max2);
    
    // Create bins
    const NUM_BINS: usize = 50;
    let bin_width = (overall_max - overall_min) / NUM_BINS as f64;
    
    let mut bins1 = vec![0u32; NUM_BINS];
    let mut bins2 = vec![0u32; NUM_BINS];
    
    // Fill bins for dataset 1
    for &value in values1 {
        let bin_idx = ((value - overall_min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(NUM_BINS - 1);
        bins1[bin_idx] += 1;
    }
    
    // Fill bins for dataset 2
    for &value in values2 {
        let bin_idx = ((value - overall_min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(NUM_BINS - 1);
        bins2[bin_idx] += 1;
    }
    
    // Find max count for y-axis
    let max_count1 = *bins1.iter().max().unwrap_or(&0);
    let max_count2 = *bins2.iter().max().unwrap_or(&0);
    let max_count = max_count1.max(max_count2);
    
    // Create the drawing area
    let root = BitMapBackend::new(filename, (1200, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    
    // Split into two charts
    let (left, right) = root.split_horizontally(600);
    
    // Draw first chart (left)
    {
        let mut chart = ChartBuilder::on(&left)
            .caption(format!("{} - {}", title, label1), ("sans-serif", 25))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(overall_min..overall_max, 0u32..max_count + max_count / 10)?;
        
        chart.configure_mesh()
            .x_desc(x_label)
            .y_desc("Count")
            .draw()?;
        
        match style1 {
            DrawingStyle::Bars => {
                // Draw as bars
                chart.draw_series(
                    bins1.iter().enumerate().map(|(i, &count)| {
                        let x0 = overall_min + i as f64 * bin_width;
                        let x1 = x0 + bin_width;
                        Rectangle::new([(x0, 0), (x1, count)], color1.filled())
                    })
                )?
                .label(label1)
                .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color1.filled()));
            },
            DrawingStyle::Line => {
                // Draw as line
                let line_data: Vec<(f64, u32)> = bins1.iter().enumerate()
                    .map(|(i, &count)| {
                        let x = overall_min + (i as f64 + 0.5) * bin_width;
                        (x, count)
                    })
                    .collect();
                
                chart.draw_series(LineSeries::new(line_data, color1))?
                    .label(label1)
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color1));
            }
        }
        
        // Draw mean line
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(mean1, 0), (mean1, max_count)],
            &BLACK,
        )))?
        .label(format!("Mean: {:.2}", mean1))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));
        
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
    }
    
    // Draw second chart (right)
    {
        let mut chart = ChartBuilder::on(&right)
            .caption(format!("{} - {}", title, label2), ("sans-serif", 25))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(overall_min..overall_max, 0u32..max_count + max_count / 10)?;
        
        chart.configure_mesh()
            .x_desc(x_label)
            .y_desc("Count")
            .draw()?;
        
        match style2 {
            DrawingStyle::Bars => {
                // Draw as bars
                chart.draw_series(
                    bins2.iter().enumerate().map(|(i, &count)| {
                        let x0 = overall_min + i as f64 * bin_width;
                        let x1 = x0 + bin_width;
                        Rectangle::new([(x0, 0), (x1, count)], color2.filled())
                    })
                )?
                .label(label2)
                .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color2.filled()));
            },
            DrawingStyle::Line => {
                // Draw as line
                let line_data: Vec<(f64, u32)> = bins2.iter().enumerate()
                    .map(|(i, &count)| {
                        let x = overall_min + (i as f64 + 0.5) * bin_width;
                        (x, count)
                    })
                    .collect();
                
                chart.draw_series(LineSeries::new(line_data, color2))?
                    .label(label2)
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color2));
            }
        }
        
        // Draw mean line
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(mean2, 0), (mean2, max_count)],
            &BLACK,
        )))?
        .label(format!("Mean: {:.2}", mean2))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));
        
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
    }
    
    root.present()?;
    
    println!("Histogram saved to {}", filename);
    println!("Left ({}) - Min: {:.2}, Max: {:.2}, Mean: {:.2}", label1, min1, max1, mean1);
    println!("Right ({}) - Min: {:.2}, Max: {:.2}, Mean: {:.2}", label2, min2, max2, mean2);
    
    Ok(())
}

/// Helper function to create a single histogram
fn create_single_histogram(
    values: &[f64],
    title: &str,
    filename: &str,
    x_label: &str,
    color: &RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    if values.is_empty() {
        return Err("Cannot create histogram: dataset is empty".into());
    }
    
    // Calculate statistics
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean_val = values.iter().sum::<f64>() / values.len() as f64;
    
    // Create bins
    const NUM_BINS: usize = 50;
    let bin_width = (max_val - min_val) / NUM_BINS as f64;
    
    let mut bins = vec![0u32; NUM_BINS];
    
    // Fill bins
    for &value in values {
        let bin_idx = ((value - min_val) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(NUM_BINS - 1);
        bins[bin_idx] += 1;
    }
    
    let max_count = *bins.iter().max().unwrap_or(&0);
    
    // Create the drawing area
    let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(min_val..max_val, 0u32..max_count + max_count / 10)?;
    
    chart.configure_mesh()
        .x_desc(x_label)
        .y_desc("Count")
        .draw()?;
    
    // Draw bars
    chart.draw_series(
        bins.iter().enumerate().map(|(i, &count)| {
            let x0 = min_val + i as f64 * bin_width;
            let x1 = x0 + bin_width;
            Rectangle::new([(x0, 0), (x1, count)], color.filled())
        })
    )?
    .label(format!("Values (n={})", values.len()))
    .legend(|(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    
    // Draw mean line
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(mean_val, 0), (mean_val, max_count)],
        &BLACK,
    )))?
    .label(format!("Mean: {:.2}", mean_val))
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));
    
    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;
    
    root.present()?;
    
    println!("Histogram saved to {}", filename);
    println!("Min: {:.2}, Max: {:.2}, Mean: {:.2}", min_val, max_val, mean_val);
    
    Ok(())
}

/// Generate sigmoid function charts for debugging
pub fn generate_sigmoid_charts() -> Result<(), Box<dyn std::error::Error>> {
    // Create charts directory if it doesn't exist
    fs::create_dir_all("charts")?;
    
    // Initialize sigmoid with specific parameters that were causing issues
    let sigmoid = crate::sigmoid::Sigmoid::new(
        18.971227371311485,     // offset
        3.0,                    // scale
        71.52711771826877,      // value
    );
    
    // Define the x range for plotting
    let x_min = 0.0;
    let x_max = 40.0;
    let num_points = 1000;
    
    // Generate data points
    let mut x_values = Vec::new();
    let mut probability_values = Vec::new();
    let mut m_values = Vec::new();
    let mut m_prime_values = Vec::new();
    
    for i in 0..num_points {
        let x = x_min + (x_max - x_min) * (i as f64) / (num_points as f64 - 1.0);
        x_values.push(x);
        probability_values.push(sigmoid.get_probability(x));
        m_values.push(sigmoid.m(x));
        m_prime_values.push(sigmoid.m_prime(x));
    }
    
    // Chart 1: get_probability()
    {
        let filepath = "charts/sigmoid_probability.png";
        let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Sigmoid: get_probability(x)", ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, 0.0..1.0)?;
        
        chart.configure_mesh().draw()?;
        
        chart.draw_series(LineSeries::new(
            x_values.iter().zip(probability_values.iter()).map(|(x, y)| (*x, *y)),
            &BLUE,
        ))?;
        
        root.present()?;
        println!("Generated: {}", filepath);
    }
    
    // Chart 2: m() - Marginal utility of spend
    {
        let filepath = "charts/sigmoid_marginal_utility.png";
        let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        // Find min and max for y-axis
        let y_min = m_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = m_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_range = if y_max - y_min < 0.1 {
            y_min - 0.5..y_max + 0.5
        } else {
            y_min..y_max
        };
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Sigmoid: M(x) - Marginal Utility of Spend", ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, y_range)?;
        
        chart.configure_mesh().draw()?;
        
        chart.draw_series(LineSeries::new(
            x_values.iter().zip(m_values.iter()).map(|(x, y)| (*x, *y)),
            &RED,
        ))?;
        
        // Draw horizontal line at y=1 (the target value that was failing)
        chart.draw_series(LineSeries::new(
            vec![(x_min, 1.0), (x_max, 1.0)],
            &BLACK.mix(0.3),
        ))?
        .label("y_target = 1.0")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK.mix(0.3)));
        
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
        
        root.present()?;
        println!("Generated: {}", filepath);
    }
    
    // Chart 3: m_prime() - Derivative of marginal utility
    {
        let filepath = "charts/sigmoid_marginal_utility_derivative.png";
        let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        // Find min and max for y-axis
        let y_min = m_prime_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = m_prime_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_range = if y_max - y_min < 0.1 {
            y_min - 0.5..y_max + 0.5
        } else {
            y_min..y_max
        };
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Sigmoid: M'(x) - Derivative of Marginal Utility", ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, y_range)?;
        
        chart.configure_mesh().draw()?;
        
        chart.draw_series(LineSeries::new(
            x_values.iter().zip(m_prime_values.iter()).map(|(x, y)| (*x, *y)),
            &GREEN,
        ))?;
        
        // Draw horizontal line at y=0
        chart.draw_series(LineSeries::new(
            vec![(x_min, 0.0), (x_max, 0.0)],
            &BLACK.mix(0.3),
        ))?;
        
        root.present()?;
        println!("Generated: {}", filepath);
    }
    
    Ok(())
}
