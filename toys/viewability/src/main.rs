use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Grid, Label, Scale, Orientation, Picture, Adjustment, GestureDrag};
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

const MAX_CPM: f64 = 20.0;
const STEP: f64 = 0.05;
const STEP_3D: f64 = 0.2; // Larger step for 3D rendering to reduce point density
const EPSILON: f64 = 0.0001;

struct Sigmoid {
    scale: f64,
    offset: f64,
    value: f64,
}

impl Sigmoid {
    fn new(scale: f64, offset: f64, value: f64) -> Self {
        Self {
            scale,
            offset,
            value: value.max(0.0).min(1.0),
        }
    }

    fn get_probability(&self, x: f64) -> f64 {
        1.0 / (1.0 + (-(x - self.offset) * self.scale).exp())
    }

    fn inverse(&self, y: f64) -> f64 {
        let y_clamped = if y < EPSILON / 10.0 {
            EPSILON / 10.0
        } else if 1.0 - y <= EPSILON / 10.0 {
            1.0 - EPSILON / 10.0
        } else {
            y
        };
        (y_clamped.ln() - (1.0 - y_clamped).ln()) / self.scale + self.offset
    }
}

struct Parameters {
    sigmoid_a_value: f64,
    sigmoid_a_scale: f64,
    sigmoid_a_offset: f64,
    sigmoid_b_value: f64,
    sigmoid_b_scale: f64,
    sigmoid_b_offset: f64,
    sigmoid_c_value: f64,
    sigmoid_c_scale: f64,
    sigmoid_c_offset: f64,
    isohypsis_value: f64,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            sigmoid_a_value: 0.8,
            sigmoid_a_scale: 0.4,
            sigmoid_a_offset: 8.0,
            sigmoid_b_value: 0.6,
            sigmoid_b_scale: 0.9,
            sigmoid_b_offset: 9.0,
            sigmoid_c_value: 0.7,
            sigmoid_c_scale: 0.6,
            sigmoid_c_offset: 7.0,
            isohypsis_value: 0.66,
        }
    }
}

struct ComputedData {
    s_a: Sigmoid,
    s_b: Sigmoid,
    s_c: Sigmoid,
    cpm_a: Vec<Vec<f64>>,
    cpm_b: Vec<Vec<f64>>,
    cpm_c: Vec<Vec<f64>>,
    weighted_sum: Vec<Vec<f64>>,
    valid_mask: Vec<Vec<bool>>,
}

fn setup_data(p: &Parameters) -> ComputedData {
    let s_a = Sigmoid::new(p.sigmoid_a_scale, p.sigmoid_a_offset, p.sigmoid_a_value);
    let s_b = Sigmoid::new(p.sigmoid_b_scale, p.sigmoid_b_offset, p.sigmoid_b_value);
    let s_c = Sigmoid::new(p.sigmoid_c_scale, p.sigmoid_c_offset, p.sigmoid_c_value);

    let cpm_a_range: Vec<f64> = (0..((MAX_CPM / STEP) as usize))
        .map(|i| i as f64 * STEP)
        .collect();
    let cpm_b_range: Vec<f64> = (0..((MAX_CPM / STEP) as usize))
        .map(|i| i as f64 * STEP)
        .collect();

    let size = cpm_b_range.len();
    let mut cpm_a = vec![vec![0.0; size]; size];
    let mut cpm_b = vec![vec![0.0; size]; size];
    let mut cpm_c = vec![vec![f64::NAN; size]; size];
    let mut weighted_sum = vec![vec![f64::NAN; size]; size];
    let mut valid_mask = vec![vec![false; size]; size];

    for (i, &cpm_a_val) in cpm_a_range.iter().enumerate() {
        let prob_a = s_a.get_probability(cpm_a_val);

        for (j, &cpm_b_val) in cpm_b_range.iter().enumerate() {
            cpm_a[j][i] = cpm_a_val;
            cpm_b[j][i] = cpm_b_val;

            let prob_b = s_b.get_probability(cpm_b_val);
            let prob_c_required = 1.0 - prob_a - prob_b;

            if prob_c_required < 0.0 || prob_c_required > 1.0 {
                continue;
            }

            let cpm_c_val = s_c.inverse(prob_c_required);

            if cpm_c_val < 0.0 || cpm_c_val > MAX_CPM * 2.0 {
                continue;
            }

            let prob_c = s_c.get_probability(cpm_c_val);
            let sum_probs = prob_a + prob_b + prob_c;

            if (sum_probs - 1.0).abs() > 0.01 {
                continue;
            }

            let weighted = s_a.value * prob_a + s_b.value * prob_b + s_c.value * prob_c;

            weighted_sum[j][i] = weighted;
            cpm_c[j][i] = cpm_c_val;
            valid_mask[j][i] = true;
        }
    }

    ComputedData {
        s_a,
        s_b,
        s_c,
        cpm_a,
        cpm_b,
        cpm_c,
        weighted_sum,
        valid_mask,
    }
}

fn render_chart1(data: &ComputedData) -> Vec<u8> {
    let width = 600;
    let height = 420;
    let mut buffer = vec![0u8; width * height * 3];
    {
        let backend = BitMapBackend::with_buffer(&mut buffer, (width as u32, height as u32));
        let root = backend.into_drawing_area();
        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .caption("Win Probability", ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(0.0..MAX_CPM, 0.0..1.0)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        // Generate probability curves
        let prob_range: Vec<f64> = (0..((MAX_CPM / STEP) as usize))
            .map(|i| i as f64 * STEP)
            .collect();
        let prob_a_full: Vec<f64> = prob_range.iter().map(|&cpm| data.s_a.get_probability(cpm)).collect();
        let prob_b_full: Vec<f64> = prob_range.iter().map(|&cpm| data.s_b.get_probability(cpm)).collect();
        let prob_c_full: Vec<f64> = prob_range.iter().map(|&cpm| data.s_c.get_probability(cpm)).collect();

        chart
            .draw_series(LineSeries::new(
                prob_range.iter().zip(prob_a_full.iter()).map(|(x, y)| (*x, *y)),
                &BLUE,
            ))
            .unwrap()
            .label("Win probability A")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

        chart
            .draw_series(LineSeries::new(
                prob_range.iter().zip(prob_b_full.iter()).map(|(x, y)| (*x, *y)),
                &GREEN,
            ))
            .unwrap()
            .label("Win probability B")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));

        chart
            .draw_series(LineSeries::new(
                prob_range.iter().zip(prob_c_full.iter()).map(|(x, y)| (*x, *y)),
                &RED,
            ))
            .unwrap()
            .label("Win probability C")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        // Draw value dots
        chart
            .draw_series(PointSeries::of_element(
                vec![(0.0, data.s_a.value)],
                5,
                &BLUE,
                &|c, s, st| {
                    return EmptyElement::at(c)
                        + Circle::new((0, 0), s, st.filled());
                },
            ))
            .unwrap()
            .label("Value A");

        chart
            .draw_series(PointSeries::of_element(
                vec![(0.0, data.s_b.value)],
                5,
                &GREEN,
                &|c, s, st| {
                    return EmptyElement::at(c)
                        + Circle::new((0, 0), s, st.filled());
                },
            ))
            .unwrap()
            .label("Value B");

        chart
            .draw_series(PointSeries::of_element(
                vec![(0.0, data.s_c.value)],
                5,
                &RED,
                &|c, s, st| {
                    return EmptyElement::at(c)
                        + Circle::new((0, 0), s, st.filled());
                },
            ))
            .unwrap()
            .label("Value C");

        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .unwrap();

        root.present().unwrap();
    }
    buffer
}

fn render_chart2(data: &ComputedData, isohypsis_value: f64) -> Vec<u8> {
    let width = 600;
    let height = 420;
    let mut buffer = vec![0u8; width * height * 3];
    {
        let backend = BitMapBackend::with_buffer(&mut buffer, (width as u32, height as u32));
        let root = backend.into_drawing_area();
        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .caption("Total Spend vs CPM A (for Isohypsis)", ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(0.0..MAX_CPM, 0.0..MAX_CPM * 3.0) // Y-axis: total spend can be up to 3*MAX_CPM
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        // For each CPM A, find points on the isohypsis and calculate total spend
        let mut spend_data: Vec<(f64, f64)> = Vec::new(); // (cpm_a, total_spend)
        let rows = data.weighted_sum.len();
        let cols = if rows > 0 { data.weighted_sum[0].len() } else { 0 };
        let tolerance = 0.01;
        
        // Collect all points on the isohypsis
        for j in 0..rows {
            for i in 0..cols {
                if data.valid_mask[j][i] {
                    let ws_val = data.weighted_sum[j][i];
                    if ws_val.is_finite() && (ws_val - isohypsis_value).abs() < tolerance {
                        let cpm_a = data.cpm_a[j][i];
                        let cpm_b = data.cpm_b[j][i];
                        let cpm_c = data.cpm_c[j][i];
                        
                        // Calculate probabilities
                        let prob_a = data.s_a.get_probability(cpm_a);
                        let prob_b = data.s_b.get_probability(cpm_b);
                        let prob_c = data.s_c.get_probability(cpm_c);
                        
                        // Calculate total spend
                        let total_spend = cpm_a * prob_a + cpm_b * prob_b + cpm_c * prob_c;
                        
                        spend_data.push((cpm_a, total_spend));
                    }
                }
            }
        }
        
        // Sort by CPM A and group/average for same CPM A values (in case of multiple solutions)
        spend_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Group by CPM A and take average spend (or min/max - let's use average)
        let mut grouped_data = Vec::new();
        if !spend_data.is_empty() {
            let mut current_cpm_a = spend_data[0].0;
            let mut spends = vec![spend_data[0].1];
            
            for &(cpm_a, spend) in spend_data.iter().skip(1) {
                if (cpm_a - current_cpm_a).abs() < STEP / 2.0 {
                    // Same CPM A (within tolerance)
                    spends.push(spend);
                } else {
                    // New CPM A - save previous group
                    let avg_spend = spends.iter().sum::<f64>() / spends.len() as f64;
                    grouped_data.push((current_cpm_a, avg_spend));
                    
                    // Start new group
                    current_cpm_a = cpm_a;
                    spends = vec![spend];
                }
            }
            // Don't forget the last group
            let avg_spend = spends.iter().sum::<f64>() / spends.len() as f64;
            grouped_data.push((current_cpm_a, avg_spend));
        }
        
        // Draw the line (thin line, no points)
        if !grouped_data.is_empty() {
            chart
                .draw_series(LineSeries::new(
                    grouped_data.iter().map(|(x, y)| (*x, *y)),
                    &BLUE,
                ))
                .unwrap()
                .label("Total Spend")
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
            
            // Find the point with minimum Y (lowest total spend)
            if let Some(min_point) = grouped_data.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
                let (min_x, min_y) = *min_point;
                
                // Find Y range for vertical line
                let y_min = grouped_data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
                let y_max = grouped_data.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);
                
                // Draw vertical line at minimum point
                chart
                    .draw_series(LineSeries::new(
                        vec![(min_x, y_min), (min_x, y_max)],
                        &RED,
                    ))
                    .unwrap();
                
                // Mark the minimum point with a circle
                chart
                    .draw_series(PointSeries::of_element(
                        vec![(min_x, min_y)],
                        5,
                        &RED,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled());
                        },
                    ))
                    .unwrap();
            }
        }

        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .unwrap();

        root.present().unwrap();
    }
    buffer
}

fn render_chart3(data: &ComputedData, isohypsis_value: f64, yaw: f64, pitch: f64) -> Vec<u8> {
    let width = 600;
    let height = 420;
    let mut buffer = vec![0u8; width * height * 3];
    {
        let backend = BitMapBackend::with_buffer(&mut buffer, (width as u32, height as u32));

        let root = backend.into_drawing_area();
        root.fill(&WHITE).unwrap();

        // Find min/max for CPM_C
        let mut min_c = f64::INFINITY;
        let mut max_c = f64::NEG_INFINITY;
        for row in &data.cpm_c {
            for &val in row {
                if val.is_finite() {
                    min_c = min_c.min(val);
                    max_c = max_c.max(val);
                }
            }
        }

        if min_c.is_infinite() {
            min_c = 0.0;
            max_c = MAX_CPM;
        }

        let mut chart = ChartBuilder::on(&root)
            .caption("CPM C Surface", ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_3d(0.0..MAX_CPM, 0.0..MAX_CPM, min_c..max_c)
            .unwrap();

        // Set 3D view with scale to prevent overflow
        chart.with_projection(|mut pb| {
            pb.pitch = pitch;
            pb.yaw = yaw;
            pb.scale = 0.7; // Scale down to keep content within bounds
            pb.into_matrix()
        });

        chart.configure_axes().draw().unwrap();

        // Create surface data points with reduced density for faster rendering
        let mut surface_points = Vec::new();
        let step_ratio = (STEP_3D / STEP) as usize;
        for (j, row) in data.cpm_c.iter().enumerate().step_by(step_ratio.max(1)) {
            for (i, &cpm_c_val) in row.iter().enumerate().step_by(step_ratio.max(1)) {
                if data.valid_mask[j][i] && cpm_c_val.is_finite() {
                    surface_points.push((
                        data.cpm_a[j][i],
                        data.cpm_b[j][i],
                        cpm_c_val,
                    ));
                }
            }
        }

        // Draw surface as scatter plot with color mapping
        if !surface_points.is_empty() {
            let color_map = |z: f64| {
                let t = (z - min_c) / (max_c - min_c);
                RGBColor(
                    (t * 255.0) as u8,
                    ((1.0 - t) * 100.0) as u8,
                    ((1.0 - t) * 255.0) as u8,
                )
            };

            for (x, y, z) in surface_points {
                let color = color_map(z);
                chart.draw_series(
                    PointSeries::of_element(
                        vec![(x, y, z)],
                        2,
                        &color,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled());
                        },
                    )
                ).unwrap();
            }
        }

        // Draw isohypsis contour in CPM A/B/C space
        // Use the same contour detection method as chart4 - find crossing points along grid edges
        let mut isohypsis_points = Vec::new();
        let rows = data.weighted_sum.len();
        let cols = if rows > 0 { data.weighted_sum[0].len() } else { 0 };
        let contour_step = 2; // Sample every 2nd row/col for contour finding (balance between accuracy and performance)
        let max_points = 2000; // Limit total points to prevent performance issues
        
        // Find contour crossings along grid edges (same method as chart4)
        for j in (0..rows.saturating_sub(1)).step_by(contour_step) {
            for i in (0..cols.saturating_sub(1)).step_by(contour_step) {
                if isohypsis_points.len() >= max_points {
                    break;
                }
                if !data.valid_mask[j][i] || !data.valid_mask[j][i+1] || 
                   !data.valid_mask[j+1][i] || !data.valid_mask[j+1][i+1] {
                    continue;
                }
                
                let v00 = data.weighted_sum[j][i];
                let v01 = data.weighted_sum[j][i+1];
                let v10 = data.weighted_sum[j+1][i];
                let v11 = data.weighted_sum[j+1][i+1];
                
                if !v00.is_finite() || !v01.is_finite() || !v10.is_finite() || !v11.is_finite() {
                    continue;
                }
                
                // Get CPM coordinates for interpolation
                let x0 = data.cpm_a[j][i];
                let x1 = data.cpm_a[j][i+1];
                let y0 = data.cpm_b[j][i];
                let y1 = data.cpm_b[j+1][i];
                let z00 = data.cpm_c[j][i];
                let z01 = data.cpm_c[j][i+1];
                let z10 = data.cpm_c[j+1][i];
                let z11 = data.cpm_c[j+1][i+1];
                
                // Check each edge for crossing using linear interpolation
                // Top edge (v00 to v01)
                if (v00 < isohypsis_value && v01 >= isohypsis_value) || 
                   (v00 >= isohypsis_value && v01 < isohypsis_value) {
                    let diff = v01 - v00;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v00) / diff;
                        let x = x0 + t * (x1 - x0);
                        let z = z00 + t * (z01 - z00);
                        isohypsis_points.push((x, y0, z));
                    }
                }
                
                // Bottom edge (v10 to v11)
                if (v10 < isohypsis_value && v11 >= isohypsis_value) || 
                   (v10 >= isohypsis_value && v11 < isohypsis_value) {
                    let diff = v11 - v10;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v10) / diff;
                        let x = x0 + t * (x1 - x0);
                        let z = z10 + t * (z11 - z10);
                        isohypsis_points.push((x, y1, z));
                    }
                }
                
                // Left edge (v00 to v10)
                if (v00 < isohypsis_value && v10 >= isohypsis_value) || 
                   (v00 >= isohypsis_value && v10 < isohypsis_value) {
                    let diff = v10 - v00;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v00) / diff;
                        let y = y0 + t * (y1 - y0);
                        let z = z00 + t * (z10 - z00);
                        isohypsis_points.push((x0, y, z));
                    }
                }
                
                // Right edge (v01 to v11)
                if (v01 < isohypsis_value && v11 >= isohypsis_value) || 
                   (v01 >= isohypsis_value && v11 < isohypsis_value) {
                    let diff = v11 - v01;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v01) / diff;
                        let y = y0 + t * (y1 - y0);
                        let z = z01 + t * (z11 - z01);
                        isohypsis_points.push((x1, y, z));
                    }
                }
                
                if isohypsis_points.len() >= max_points {
                    break;
                }
            }
            if isohypsis_points.len() >= max_points {
                break;
            }
        }
        
        // Draw isohypsis points - optimized for performance
        if !isohypsis_points.is_empty() {
            // For large point sets, use simpler grouping to avoid O(n²) complexity
            if isohypsis_points.len() > 500 {
                // Draw as simple point cloud for large sets
                chart.draw_series(
                    PointSeries::of_element(
                        isohypsis_points,
                        2,
                        &RED,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled());
                        },
                    )
                ).unwrap();
            } else {
                // For smaller sets, group into paths (but limit search to avoid O(n²))
                let mut used = vec![false; isohypsis_points.len()];
                let mut paths = Vec::new();
                let max_path_length = 100; // Limit path length
                
                for start_idx in 0..isohypsis_points.len() {
                    if used[start_idx] {
                        continue;
                    }
                    
                    let mut path = vec![isohypsis_points[start_idx]];
                    used[start_idx] = true;
                    
                    // Try to extend the path (with limited iterations)
                    for _iter in 0..max_path_length {
                        let mut found = false;
                        let current = path[path.len() - 1];
                        let search_radius_sq = (STEP * 1.5).powi(2);
                        
                        // Only search nearby points (within grid distance)
                        for (idx, &point) in isohypsis_points.iter().enumerate() {
                            if used[idx] {
                                continue;
                            }
                            
                            let dist_sq = (current.0 - point.0).powi(2) + (current.1 - point.1).powi(2);
                            if dist_sq < search_radius_sq {
                                path.push(point);
                                used[idx] = true;
                                found = true;
                                break;
                            }
                        }
                        
                        if !found {
                            break;
                        }
                    }
                    
                    if path.len() > 1 {
                        paths.push(path);
                    }
                }
                
                // Draw paths
                for path in paths {
                    chart.draw_series(
                        PointSeries::of_element(
                            path,
                            3,
                            &RED,
                            &|c, s, st| {
                                return EmptyElement::at(c)
                                    + Circle::new((0, 0), s, st.filled());
                            },
                        )
                    ).unwrap();
                }
                
                // Draw remaining individual points
                let remaining: Vec<_> = isohypsis_points.iter()
                    .enumerate()
                    .filter(|(idx, _)| !used[*idx])
                    .map(|(_, &p)| p)
                    .collect();
                
                if !remaining.is_empty() {
                    chart.draw_series(
                        PointSeries::of_element(
                            remaining,
                            2,
                            &RED,
                            &|c, s, st| {
                                return EmptyElement::at(c)
                                    + Circle::new((0, 0), s, st.filled());
                            },
                        )
                    ).unwrap();
                }
            }
        }

        root.present().unwrap();
    }
    buffer
}

fn render_chart4(data: &ComputedData, isohypsis_value: f64, yaw: f64, pitch: f64) -> Vec<u8> {
    let width = 600;
    let height = 420;
    let mut buffer = vec![0u8; width * height * 3];
    {
        let backend = BitMapBackend::with_buffer(&mut buffer, (width as u32, height as u32));
        let root = backend.into_drawing_area();
        root.fill(&WHITE).unwrap();

        // Find min/max for weighted sum
        let mut min_ws = f64::INFINITY;
        let mut max_ws = f64::NEG_INFINITY;
        for row in &data.weighted_sum {
            for &val in row {
                if val.is_finite() {
                    min_ws = min_ws.min(val);
                    max_ws = max_ws.max(val);
                }
            }
        }

        if min_ws.is_infinite() {
            min_ws = 0.0;
            max_ws = 1.0;
        }

        let mut chart = ChartBuilder::on(&root)
            .caption(&format!("Weighted Sum Surface (isohypsis at {:.2})", isohypsis_value), ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_3d(0.0..MAX_CPM, 0.0..MAX_CPM, min_ws..max_ws)
            .unwrap();

        // Set 3D view with scale to prevent overflow
        chart.with_projection(|mut pb| {
            pb.pitch = pitch;
            pb.yaw = yaw;
            pb.scale = 0.7; // Scale down to keep content within bounds
            pb.into_matrix()
        });

        chart.configure_axes().draw().unwrap();

        // Create surface data points with reduced density for faster rendering
        let mut surface_points = Vec::new();
        let step_ratio = (STEP_3D / STEP) as usize;
        for (j, row) in data.weighted_sum.iter().enumerate().step_by(step_ratio.max(1)) {
            for (i, &ws_val) in row.iter().enumerate().step_by(step_ratio.max(1)) {
                if data.valid_mask[j][i] && ws_val.is_finite() {
                    let x = data.cpm_a[j][i];
                    let y = data.cpm_b[j][i];
                    surface_points.push((x, y, ws_val));
                }
            }
        }

        // Draw surface
        if !surface_points.is_empty() {
            let color_map = |z: f64| {
                let t = (z - min_ws) / (max_ws - min_ws);
                RGBColor(
                    (t * 100.0) as u8,
                    (t * 200.0) as u8,
                    ((1.0 - t) * 255.0) as u8,
                )
            };

            for (x, y, z) in surface_points {
                let color = color_map(z);
                chart.draw_series(
                    PointSeries::of_element(
                        vec![(x, y, z)],
                        2,
                        &color,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled());
                        },
                    )
                ).unwrap();
            }
        }

        // Draw isohypsis contour - find crossing points and connect them
        // Use reduced resolution for performance, but still accurate enough
        let mut contour_segments = Vec::new();
        let rows = data.weighted_sum.len();
        let cols = if rows > 0 { data.weighted_sum[0].len() } else { 0 };
        let contour_step = 2; // Sample every 2nd row/col for contour finding (balance between accuracy and performance)
        let max_contour_points = 1000; // Limit contour points to prevent performance issues
        
        // Find contour crossings along grid edges (reduced resolution for performance)
        for j in (0..rows.saturating_sub(1)).step_by(contour_step) {
            for i in (0..cols.saturating_sub(1)).step_by(contour_step) {
                if contour_segments.len() >= max_contour_points {
                    break;
                }
                if !data.valid_mask[j][i] || !data.valid_mask[j][i+1] || 
                   !data.valid_mask[j+1][i] || !data.valid_mask[j+1][i+1] {
                    continue;
                }
                
                let v00 = data.weighted_sum[j][i];
                let v01 = data.weighted_sum[j][i+1];
                let v10 = data.weighted_sum[j+1][i];
                let v11 = data.weighted_sum[j+1][i+1];
                
                if !v00.is_finite() || !v01.is_finite() || !v10.is_finite() || !v11.is_finite() {
                    continue;
                }
                
                let x0 = data.cpm_a[j][i];
                let x1 = data.cpm_a[j][i+1];
                let y0 = data.cpm_b[j][i];
                let y1 = data.cpm_b[j+1][i];
                
                // Check each edge for crossing using linear interpolation
                // Top edge (v00 to v01)
                if (v00 < isohypsis_value && v01 >= isohypsis_value) || 
                   (v00 >= isohypsis_value && v01 < isohypsis_value) {
                    let diff = v01 - v00;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v00) / diff;
                        let x = x0 + t * (x1 - x0);
                        contour_segments.push((x, y0, isohypsis_value));
                    }
                }
                
                // Bottom edge (v10 to v11)
                if (v10 < isohypsis_value && v11 >= isohypsis_value) || 
                   (v10 >= isohypsis_value && v11 < isohypsis_value) {
                    let diff = v11 - v10;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v10) / diff;
                        let x = x0 + t * (x1 - x0);
                        contour_segments.push((x, y1, isohypsis_value));
                    }
                }
                
                // Left edge (v00 to v10)
                if (v00 < isohypsis_value && v10 >= isohypsis_value) || 
                   (v00 >= isohypsis_value && v10 < isohypsis_value) {
                    let diff = v10 - v00;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v00) / diff;
                        let y = y0 + t * (y1 - y0);
                        contour_segments.push((x0, y, isohypsis_value));
                    }
                }
                
                // Right edge (v01 to v11)
                if (v01 < isohypsis_value && v11 >= isohypsis_value) || 
                   (v01 >= isohypsis_value && v11 < isohypsis_value) {
                    let diff = v11 - v01;
                    if diff.abs() > 1e-10 {
                        let t = (isohypsis_value - v01) / diff;
                        let y = y0 + t * (y1 - y0);
                        contour_segments.push((x1, y, isohypsis_value));
                    }
                }
                if contour_segments.len() >= max_contour_points {
                    break;
                }
            }
            if contour_segments.len() >= max_contour_points {
                break;
            }
        }
        
        // Draw isohypsis contour - optimized for performance
        if !contour_segments.is_empty() {
            // For large point sets, use simpler rendering
            if contour_segments.len() > 300 {
                // Draw as simple point cloud for large sets
                chart.draw_series(
                    PointSeries::of_element(
                        contour_segments,
                        2,
                        &RED,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled());
                        },
                    )
                ).unwrap();
            } else {
                // For smaller sets, group into paths (with limited iterations)
                let mut used = vec![false; contour_segments.len()];
                let mut paths = Vec::new();
                let max_path_length = 50; // Limit path length
                
                for start_idx in 0..contour_segments.len() {
                    if used[start_idx] {
                        continue;
                    }
                    
                    let mut path = vec![contour_segments[start_idx]];
                    used[start_idx] = true;
                    
                    // Try to extend the path (with limited iterations)
                    for _iter in 0..max_path_length {
                        let mut found = false;
                        let current = path[path.len() - 1];
                        let search_radius_sq = (STEP * 2.0).powi(2);
                        
                        for (idx, &point) in contour_segments.iter().enumerate() {
                            if used[idx] {
                                continue;
                            }
                            
                            let dist_sq = (current.0 - point.0).powi(2) + (current.1 - point.1).powi(2);
                            if dist_sq < search_radius_sq {
                                path.push(point);
                                used[idx] = true;
                                found = true;
                                break;
                            }
                        }
                        
                        if !found {
                            break;
                        }
                    }
                    
                    if path.len() > 1 {
                        paths.push(path);
                    }
                }
                
                // Draw paths
                for path in paths {
                    chart.draw_series(
                        PointSeries::of_element(
                            path,
                            2,
                            &RED,
                            &|c, s, st| {
                                return EmptyElement::at(c)
                                    + Circle::new((0, 0), s, st.filled());
                            },
                        )
                    ).unwrap();
                }
                
                // Draw remaining individual points
                let remaining: Vec<_> = contour_segments.iter()
                    .enumerate()
                    .filter(|(idx, _)| !used[*idx])
                    .map(|(_, &p)| p)
                    .collect();
                
                if !remaining.is_empty() {
                    chart.draw_series(
                        PointSeries::of_element(
                            remaining,
                            2,
                            &RED,
                            &|c, s, st| {
                                return EmptyElement::at(c)
                                    + Circle::new((0, 0), s, st.filled());
                            },
                        )
                    ).unwrap();
                }
            }
        }

        root.present().unwrap();
    }
    buffer
}

fn update_picture_from_buffer(picture: &Picture, buffer: &[u8], width: u32, height: u32) {
    let pixbuf = gtk4::gdk_pixbuf::Pixbuf::from_bytes(
        &gtk4::glib::Bytes::from(buffer),
        gtk4::gdk_pixbuf::Colorspace::Rgb,
        false,
        8,
        width as i32,
        height as i32,
        (width * 3) as i32,
    );
    picture.set_pixbuf(Some(&pixbuf));
}

struct MainWindow {
    window: ApplicationWindow,
    parameters: Arc<Mutex<Parameters>>,
    picture1: Picture,
    picture2: Picture,
    picture3: Picture,
    picture4: Picture,
    rotation3: Arc<Mutex<(f64, f64)>>, // (yaw, pitch) for chart3
    rotation4: Arc<Mutex<(f64, f64)>>, // (yaw, pitch) for chart4
}

impl MainWindow {
    fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1400)
            .default_height(850)
            .title("Viewability Simulation")
            .build();

        let parameters = Arc::new(Mutex::new(Parameters::default()));

        let box1 = Box::new(Orientation::Horizontal, 10);
        let box2 = Box::new(Orientation::Vertical, 10);
        let box3 = Box::new(Orientation::Vertical, 10);

        box2.set_size_request(350, -1);
        box2.set_hexpand(false);

        window.set_child(Some(&box1));
        box1.append(&box2);
        box1.append(&box3);

        let label = Label::new(Some("Parameters"));
        box2.append(&label);

        let picture1 = Picture::new();
        let picture2 = Picture::new();
        let picture3 = Picture::new();
        let picture4 = Picture::new();

        picture1.set_size_request(600, 420);
        picture2.set_size_request(600, 420);
        picture3.set_size_request(600, 420);
        picture4.set_size_request(600, 420);

        let charts_grid = Grid::new();
        charts_grid.set_row_spacing(10);
        charts_grid.set_column_spacing(10);

        charts_grid.attach(&picture1, 0, 0, 1, 1);  // Top left
        charts_grid.attach(&picture2, 1, 0, 1, 1);  // Top right
        charts_grid.attach(&picture3, 0, 1, 1, 1);  // Bottom left
        charts_grid.attach(&picture4, 1, 1, 1, 1);  // Bottom right

        box3.append(&charts_grid);

        // Initialize rotation state with sensible 3D perspective
        // pitch ~0.5 and yaw ~0.5 gives a nice angled view
        let rotation3 = Arc::new(Mutex::new((0.5, 0.5))); // (yaw, pitch)
        let rotation4 = Arc::new(Mutex::new((0.5, 0.5)));

        // Create update function
        let picture1_clone = picture1.clone();
        let picture2_clone = picture2.clone();
        let picture3_clone = picture3.clone();
        let picture4_clone = picture4.clone();
        let params_clone = Arc::clone(&parameters);
        let rot3_clone = Arc::clone(&rotation3);
        let rot4_clone = Arc::clone(&rotation4);
        
        let update_charts = move || {
            let params = params_clone.lock().unwrap();
            let data = setup_data(&params);
            let isohypsis = params.isohypsis_value;
            drop(params);

            let rot3 = rot3_clone.lock().unwrap();
            let rot4 = rot4_clone.lock().unwrap();

            let buffer1 = render_chart1(&data);
            let buffer2 = render_chart2(&data, isohypsis);
            let buffer3 = render_chart3(&data, isohypsis, rot3.0, rot3.1);
            let buffer4 = render_chart4(&data, isohypsis, rot4.0, rot4.1);
            drop(rot3);
            drop(rot4);

            update_picture_from_buffer(&picture1_clone, &buffer1, 600, 420);
            update_picture_from_buffer(&picture2_clone, &buffer2, 600, 420);
            update_picture_from_buffer(&picture3_clone, &buffer3, 600, 420);
            update_picture_from_buffer(&picture4_clone, &buffer4, 600, 420);
        };

        // Initial render
        update_charts();
        
        // Add mouse drag handlers for natural 3D rotation
        let update_charts_3 = {
            let picture1 = picture1.clone();
            let picture2 = picture2.clone();
            let picture3 = picture3.clone();
            let picture4 = picture4.clone();
            let params_clone2 = Arc::clone(&parameters);
            let rot3_clone2 = Arc::clone(&rotation3);
            let rot4_clone2 = Arc::clone(&rotation4);
            move || {
                let params = params_clone2.lock().unwrap();
                let data = setup_data(&params);
                let isohypsis = params.isohypsis_value;
                drop(params);
                
                let rot3 = rot3_clone2.lock().unwrap();
                let rot4 = rot4_clone2.lock().unwrap();

                let buffer1 = render_chart1(&data);
                let buffer2 = render_chart2(&data, isohypsis);
                let buffer3 = render_chart3(&data, isohypsis, rot3.0, rot3.1);
                let buffer4 = render_chart4(&data, isohypsis, rot4.0, rot4.1);
                drop(rot3);
                drop(rot4);

                update_picture_from_buffer(&picture1, &buffer1, 600, 420);
                update_picture_from_buffer(&picture2, &buffer2, 600, 420);
                update_picture_from_buffer(&picture3, &buffer3, 600, 420);
                update_picture_from_buffer(&picture4, &buffer4, 600, 420);
            }
        };
        
        let update_charts_4 = {
            let picture1 = picture1.clone();
            let picture2 = picture2.clone();
            let picture3 = picture3.clone();
            let picture4 = picture4.clone();
            let params_clone2 = Arc::clone(&parameters);
            let rot3_clone2 = Arc::clone(&rotation3);
            let rot4_clone2 = Arc::clone(&rotation4);
            move || {
                let params = params_clone2.lock().unwrap();
                let data = setup_data(&params);
                let isohypsis = params.isohypsis_value;
                drop(params);
                
                let rot3 = rot3_clone2.lock().unwrap();
                let rot4 = rot4_clone2.lock().unwrap();

                let buffer1 = render_chart1(&data);
                let buffer2 = render_chart2(&data, isohypsis);
                let buffer3 = render_chart3(&data, isohypsis, rot3.0, rot3.1);
                let buffer4 = render_chart4(&data, isohypsis, rot4.0, rot4.1);
                drop(rot3);
                drop(rot4);

                update_picture_from_buffer(&picture1, &buffer1, 600, 420);
                update_picture_from_buffer(&picture2, &buffer2, 600, 420);
                update_picture_from_buffer(&picture3, &buffer3, 600, 420);
                update_picture_from_buffer(&picture4, &buffer4, 600, 420);
            }
        };
        
        // Add gesture drag for chart3 - natural rotation
        let gesture3 = GestureDrag::new();
        let start_rot3 = Arc::new(Mutex::new((0.0, 0.0)));
        let rot3_gesture = Arc::clone(&rotation3);
        let update_fn_3 = Rc::new(RefCell::new(update_charts_3));
        
        let rot3_start = Arc::clone(&rotation3);
        let start_rot3_clone = Arc::clone(&start_rot3);
        gesture3.connect_drag_begin(move |_gesture, _start_x, _start_y| {
            let rot = rot3_start.lock().unwrap();
            *start_rot3_clone.lock().unwrap() = (rot.0, rot.1);
            drop(rot);
        });
        
        let rot3_update = Arc::clone(&rotation3);
        let start_rot3_update = Arc::clone(&start_rot3);
        gesture3.connect_drag_update(move |_gesture, offset_x, offset_y| {
            let start_rot = start_rot3_update.lock().unwrap();
            let mut rot = rot3_update.lock().unwrap();
            
            // Natural rotation: horizontal drag rotates yaw, vertical drag rotates pitch
            // Inverted Y for intuitive control (drag up = tilt up)
            rot.0 = start_rot.0 + offset_x * 0.005; // yaw
            rot.1 = (start_rot.1 - offset_y * 0.005).max(-1.0).min(1.0); // pitch, clamped
            
            drop(rot);
            drop(start_rot);
            update_fn_3.borrow_mut()();
        });
        picture3.add_controller(gesture3);
        
        // Add gesture drag for chart4 - natural rotation
        let gesture4 = GestureDrag::new();
        let start_rot4 = Arc::new(Mutex::new((0.0, 0.0)));
        let update_fn_4 = Rc::new(RefCell::new(update_charts_4));
        
        let rot4_start = Arc::clone(&rotation4);
        let start_rot4_clone = Arc::clone(&start_rot4);
        gesture4.connect_drag_begin(move |_gesture, _start_x, _start_y| {
            let rot = rot4_start.lock().unwrap();
            *start_rot4_clone.lock().unwrap() = (rot.0, rot.1);
            drop(rot);
        });
        
        let rot4_update = Arc::clone(&rotation4);
        let start_rot4_update = Arc::clone(&start_rot4);
        gesture4.connect_drag_update(move |_gesture, offset_x, offset_y| {
            let start_rot = start_rot4_update.lock().unwrap();
            let mut rot = rot4_update.lock().unwrap();
            
            // Natural rotation: horizontal drag rotates yaw, vertical drag rotates pitch
            rot.0 = start_rot.0 + offset_x * 0.005; // yaw
            rot.1 = (start_rot.1 - offset_y * 0.005).max(-1.0).min(1.0); // pitch, clamped
            
            drop(rot);
            drop(start_rot);
            update_fn_4.borrow_mut()();
        });
        picture4.add_controller(gesture4);

        // Create sliders with proper callbacks
        let params_for_sliders = Arc::clone(&parameters);
        let rot3_for_sliders = Arc::clone(&rotation3);
        let rot4_for_sliders = Arc::clone(&rotation4);
        let update_fn: Rc<RefCell<dyn FnMut()>> = {
            let picture1 = picture1.clone();
            let picture2 = picture2.clone();
            let picture3 = picture3.clone();
            let picture4 = picture4.clone();
            Rc::new(RefCell::new(move || {
                let params = params_for_sliders.lock().unwrap();
                let data = setup_data(&params);
                let isohypsis = params.isohypsis_value;
                drop(params);
                
                let rot3 = rot3_for_sliders.lock().unwrap();
                let rot4 = rot4_for_sliders.lock().unwrap();

                let buffer1 = render_chart1(&data);
                let buffer2 = render_chart2(&data, isohypsis);
                let buffer3 = render_chart3(&data, isohypsis, rot3.0, rot3.1);
                let buffer4 = render_chart4(&data, isohypsis, rot4.0, rot4.1);
                drop(rot3);
                drop(rot4);

                update_picture_from_buffer(&picture1, &buffer1, 600, 420);
                update_picture_from_buffer(&picture2, &buffer2, 600, 420);
                update_picture_from_buffer(&picture3, &buffer3, 600, 420);
                update_picture_from_buffer(&picture4, &buffer4, 600, 420);
            }))
        };

        MainWindow::add_slider(&box2, "Sigma A value", 0.0, 1.0, 0.8, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_a_value = v.max(0.0).min(1.0));
        MainWindow::add_slider(&box2, "Sigma A scale", 0.001, 2.0, 0.4, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_a_scale = v);
        MainWindow::add_slider(&box2, "Sigma A offset", 0.0, 20.0, 8.0, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_a_offset = v);
        MainWindow::add_slider(&box2, "Sigma B value", 0.0, 1.0, 0.6, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_b_value = v.max(0.0).min(1.0));
        MainWindow::add_slider(&box2, "Sigma B scale", 0.001, 2.0, 0.9, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_b_scale = v);
        MainWindow::add_slider(&box2, "Sigma B offset", 0.0, 20.0, 9.0, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_b_offset = v);
        MainWindow::add_slider(&box2, "Sigma C value", 0.0, 1.0, 0.7, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_c_value = v.max(0.0).min(1.0));
        MainWindow::add_slider(&box2, "Sigma C scale", 0.001, 2.0, 0.6, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_c_scale = v);
        MainWindow::add_slider(&box2, "Sigma C offset", 0.0, 20.0, 7.0, Arc::clone(&parameters), Rc::clone(&update_fn), |p, v| p.sigmoid_c_offset = v);
        MainWindow::add_slider(&box2, "Isohypsis value", 0.0, 1.0, 0.66, Arc::clone(&parameters), update_fn, |p, v| p.isohypsis_value = v);

        Self {
            window,
            parameters,
            picture1,
            picture2,
            picture3,
            picture4,
            rotation3,
            rotation4,
        }
    }

    fn add_slider<F>(
        box2: &Box,
        label_text: &str,
        min: f64,
        max: f64,
        initial: f64,
        parameters: Arc<Mutex<Parameters>>,
        update_fn: Rc<RefCell<dyn FnMut()>>,
        setter: F,
    ) where
        F: Fn(&mut Parameters, f64) + 'static + Clone,
    {
        let adjustment = Adjustment::new(initial, min, max, 0.01, 0.1, 0.0);
        let slider = Scale::builder()
            .digits(2)
            .adjustment(&adjustment)
            .draw_value(true)
            .orientation(Orientation::Horizontal)
            .build();

        // Make slider expand to fill available space
        slider.set_hexpand(true);
        slider.set_vexpand(false);
        slider.set_size_request(200, -1); // Set minimum width

        let params_clone = Arc::clone(&parameters);
        let setter_clone = setter.clone();
        let update_fn_clone = Rc::clone(&update_fn);
        slider.connect_value_changed(move |slider| {
            let new_value = slider.value();
            let mut params = params_clone.lock().unwrap();
            setter_clone(&mut params, new_value);
            drop(params);
            update_fn_clone.borrow_mut()();
        });

        let label = Label::new(Some(label_text));
        label.set_hexpand(false);
        label.set_vexpand(false);
        
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_hexpand(true);
        hbox.append(&label);
        hbox.append(&slider);

        box2.append(&hbox);
    }

    fn present(&self) {
        self.window.present();
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.ViewabilitySimulation")
        .build();

    app.connect_activate(|app| {
        let win = MainWindow::new(app);
        win.present();
    });

    app.run();
}
