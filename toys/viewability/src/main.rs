use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Grid, Label, Scale, Orientation, Picture, Adjustment};
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

fn render_chart3(data: &ComputedData) -> Vec<u8> {
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

        root.present().unwrap();
    }
    buffer
}

fn render_chart4(data: &ComputedData, isohypsis_value: f64) -> Vec<u8> {
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
        // Use full resolution for contour to maintain accuracy
        let mut contour_segments = Vec::new();
        let rows = data.weighted_sum.len();
        let cols = if rows > 0 { data.weighted_sum[0].len() } else { 0 };
        
        // Find contour crossings along grid edges (use full resolution for accuracy)
        for j in 0..rows.saturating_sub(1) {
            for i in 0..cols.saturating_sub(1) {
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
            }
        }
        
        // Draw isohypsis contour - connect crossing points into continuous lines
        if !contour_segments.is_empty() {
            // Group points into connected paths
            // Simple approach: connect points that are close together
            let mut used = vec![false; contour_segments.len()];
            let mut paths = Vec::new();
            
            for start_idx in 0..contour_segments.len() {
                if used[start_idx] {
                    continue;
                }
                
                let mut path = vec![contour_segments[start_idx]];
                used[start_idx] = true;
                
                // Try to extend the path by finding nearby unused points
                loop {
                    let mut found = false;
                    let current = path[path.len() - 1];
                    
                    for (idx, &point) in contour_segments.iter().enumerate() {
                        if used[idx] {
                            continue;
                        }
                        
                        let dist = ((current.0 - point.0).powi(2) + (current.1 - point.1).powi(2)).sqrt();
                        if dist < STEP * 1.5 {
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
            
            // Draw each path as a connected line by drawing points close together
            // For 3D plots, we'll draw the contour as a series of closely spaced points
            // that visually form a line
            for path in paths {
                // Draw the path as a series of points
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
            
            // Also draw individual crossing points for better visibility
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
    picture3: Picture,
    picture4: Picture,
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
        let picture3 = Picture::new();
        let picture4 = Picture::new();

        picture1.set_size_request(600, 420);
        picture3.set_size_request(600, 420);
        picture4.set_size_request(600, 420);

        let charts_grid = Grid::new();
        charts_grid.set_row_spacing(10);
        charts_grid.set_column_spacing(10);

        charts_grid.attach(&picture1, 0, 0, 2, 1);
        charts_grid.attach(&picture3, 0, 1, 1, 1);
        charts_grid.attach(&picture4, 1, 1, 1, 1);

        box3.append(&charts_grid);

        // Create update function
        let picture1_clone = picture1.clone();
        let picture3_clone = picture3.clone();
        let picture4_clone = picture4.clone();
        let params_clone = Arc::clone(&parameters);
        
        let update_charts = move || {
            let params = params_clone.lock().unwrap();
            let data = setup_data(&params);
            let isohypsis = params.isohypsis_value;
            drop(params);

            let buffer1 = render_chart1(&data);
            let buffer3 = render_chart3(&data);
            let buffer4 = render_chart4(&data, isohypsis);

            update_picture_from_buffer(&picture1_clone, &buffer1, 600, 420);
            update_picture_from_buffer(&picture3_clone, &buffer3, 600, 420);
            update_picture_from_buffer(&picture4_clone, &buffer4, 600, 420);
        };

        // Initial render
        update_charts();

        // Create sliders with proper callbacks
        let params_for_sliders = Arc::clone(&parameters);
        let update_fn: Rc<RefCell<dyn FnMut()>> = {
            let picture1 = picture1.clone();
            let picture3 = picture3.clone();
            let picture4 = picture4.clone();
            Rc::new(RefCell::new(move || {
                let params = params_for_sliders.lock().unwrap();
                let data = setup_data(&params);
                let isohypsis = params.isohypsis_value;
                drop(params);

                let buffer1 = render_chart1(&data);
                let buffer3 = render_chart3(&data);
                let buffer4 = render_chart4(&data, isohypsis);

                update_picture_from_buffer(&picture1, &buffer1, 600, 420);
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
            picture3,
            picture4,
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
