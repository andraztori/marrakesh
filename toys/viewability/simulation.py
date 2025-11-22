import gi, sys
gi.require_version('Adw', '1')
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, Adw, GLib

import math
import signal
import numpy as np
from matplotlib.backends.backend_gtk4agg import \
    FigureCanvasGTK4Agg as FigureCanvas
from matplotlib.figure import Figure
import matplotlib.pyplot as plt


MAX_CPM = 20
STEP = 0.05

EPSILON = 0.0001

class Sigmoid:
    def __init__(self, scale, offset, value):
        self.scale = scale
        self.offset = offset
        # Clamp value to [0, 1.0]
        self.value = max(0.0, min(1.0, value))
    
    def get_probability(self, x):
        return 1.0 / (1 + math.exp(-(x - self.offset) * self.scale))
    
    def inverse(self, y):
        """Inverse of the sigmoid function. Returns x such that get_probability(x) = y"""
        y_clamped = y
        if y < EPSILON / 10.0:
            y_clamped = EPSILON / 10.0
        if 1.0 - y <= EPSILON / 10.0:
            y_clamped = 1.0 - EPSILON / 10.0
        return (math.log(y_clamped) - math.log(1.0 - y_clamped)) / self.scale + self.offset


class Parameters:
    def __init__(self):
        self.sigmoid_A_value = 0.8
        self.sigmoid_A_scale = 0.4
        self.sigmoid_A_offset = 8.0
        self.sigmoid_B_value = 0.6
        self.sigmoid_B_scale = 0.9
        self.sigmoid_B_offset = 9.0


def setup_data(p: Parameters):
    """Setup sigmoid objects and compute data.
    For each CPM A, find CPM B such that prob_A + prob_B = 1.0"""
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, p.sigmoid_A_value)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset, p.sigmoid_B_value)

    # Initialize data lists
    l_prob_A = []
    l_prob_B = []
    l_cpm_A = []  # CPM A values
    l_cpm_B = []  # CPM B values (calculated to satisfy prob_A + prob_B = 1.0)
    l_weighted_sum_values = []  # Weighted sum of values divided by sum of probabilities
    l_total_impressions = []  # Total impressions bought (sum of probabilities)

    # For each CPM A, find CPM B such that prob_A + prob_B = 1.0
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm_A = 1.0 * cpm_input * STEP
        prob_A = s_A.get_probability(cpm_A)
        
        # Calculate required prob_B to make prob_A + prob_B = 1.0
        prob_B_required = 1.0 - prob_A
        
        # Check if prob_B_required is valid (between 0 and 1)
        if prob_B_required < 0.0 or prob_B_required > 1.0:
            continue  # Skip invalid combinations
        
        # Find CPM B using inverse sigmoid
        try:
            cpm_B = s_B.inverse(prob_B_required)
            
            # Only include valid CPM B values (non-negative and reasonable)
            if cpm_B < 0.0 or cpm_B > MAX_CPM * 2:
                continue
                
            prob_B = s_B.get_probability(cpm_B)
            
            l_cpm_A.append(cpm_A)
            l_cpm_B.append(cpm_B)
            l_prob_A.append(prob_A)
            l_prob_B.append(prob_B)
            
            # Calculate total impressions bought (should be 1.0)
            total_impressions = prob_A + prob_B
            l_total_impressions.append(total_impressions)
            
            # Calculate weighted sum of values divided by sum of probabilities
            # weighted_sum = (value_A * prob_A + value_B * prob_B) / (prob_A + prob_B)
            sum_probs = prob_A + prob_B
            if sum_probs > 0.0001:  # Avoid division by zero
                weighted_sum = (s_A.value * prob_A + s_B.value * prob_B) / sum_probs
            else:
                weighted_sum = 0.0
            l_weighted_sum_values.append(weighted_sum)
        except:
            # Skip if inverse calculation fails
            continue

    return {
        's_A': s_A,
        's_B': s_B,
        'l_prob_A': l_prob_A,
        'l_prob_B': l_prob_B,
        'l_cpm_A': l_cpm_A,
        'l_cpm_B': l_cpm_B,
        'l_weighted_sum_values': l_weighted_sum_values,
        'l_total_impressions': l_total_impressions,
    }


def update_axis1_win_probability(chart, data):
    """Update axis 1 with win probability curves and value dots."""
    a = chart.axis1
    a.clear()
    a.set_xlim(right=MAX_CPM)
    a.set_ylim(bottom=0, top=1.0)
    
    # Generate full probability curves for visualization
    prob_range = []
    prob_A_full = []
    prob_B_full = []
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        prob_range.append(cpm)
        prob_A_full.append(data['s_A'].get_probability(cpm))
        prob_B_full.append(data['s_B'].get_probability(cpm))
    
    # Plot win probability curves
    line1 = a.plot(prob_range, prob_A_full, 'C0-', label='Win probability A')
    line2 = a.plot(prob_range, prob_B_full, 'C1-', label='Win probability B')
    
    # Plot the actual pairs where prob_A + prob_B = 1.0
    if len(data['l_cpm_A']) > 0:
        line3 = a.plot(data['l_cpm_A'], data['l_prob_A'], 'C0o', markersize=4, label='A pairs', alpha=0.6)
        line4 = a.plot(data['l_cpm_B'], data['l_prob_B'], 'C1o', markersize=4, label='B pairs', alpha=0.6)
    
    # Draw value for each impression as a dot on the vertical axis (x=0)
    a.plot(0, data['s_A'].value, 'C0s', markersize=10, label='Value A')
    a.plot(0, data['s_B'].value, 'C1s', markersize=10, label='Value B')
    
    a.set_xlabel('CPM')
    a.set_ylabel('Probability / Value')
    a.legend(loc='lower right')
    a.grid(True, alpha=0.3)


def update_axis3_bottom_left(chart, data):
    """Update axis 3 with weighted sum curve."""
    a = chart.axis3
    a.clear()
    
    if len(data['l_cpm_A']) == 0:
        a.text(0.5, 0.5, 'No valid CPM pairs found\n(prob_A + prob_B = 1.0)', 
               transform=a.transAxes, ha='center', va='center', fontsize=12)
        return
    
    # Use CPM A as x-axis
    max_cpm = max(max(data['l_cpm_A']) if data['l_cpm_A'] else MAX_CPM, 
                  max(data['l_cpm_B']) if data['l_cpm_B'] else MAX_CPM)
    a.set_xlim(right=max(MAX_CPM, max_cpm * 1.1))
    a.set_ylim(bottom=0, top=1.0)
    
    # Plot win probability curves on left y-axis (using CPM A as x-axis)
    line1 = a.plot(data['l_cpm_A'], data['l_prob_A'], 'C0-', label='Win probability A', alpha=0.5)
    line2 = a.plot(data['l_cpm_A'], data['l_prob_B'], 'C1-', label='Win probability B', alpha=0.5)
    
    # Plot weighted sum of values divided by sum of probabilities on left y-axis
    line4 = a.plot(data['l_cpm_A'], data['l_weighted_sum_values'], 'g-', 
                   label='Weighted sum / Sum of probabilities', linewidth=2)
    
    a.set_xlabel('CPM A')
    a.set_ylabel('Probability / Weighted Value', color='black')
    a.tick_params(axis='y', labelcolor='black')
    
    # Create right y-axis for total impressions bought
    ax3_right = a.twinx()
    max_impressions = max(data['l_total_impressions']) if data['l_total_impressions'] else 2.0
    ax3_right.set_ylim(bottom=0, top=max(2.0, max_impressions * 1.1))
    
    # Plot total impressions bought on right y-axis (should be 1.0)
    line3 = ax3_right.plot(data['l_cpm_A'], data['l_total_impressions'], 'r-', 
                           label='Total impressions bought', linewidth=2)
    ax3_right.set_ylabel('Total impressions bought', color='r')
    ax3_right.tick_params(axis='y', labelcolor='r')
    
    # Combine legends from both axes
    lines = line1 + line2 + line3 + line4
    labels = [l.get_label() for l in lines]
    a.legend(lines, labels, loc='lower right')
    a.grid(True, alpha=0.3)


def update_axis(chart, p: Parameters):
    # Setup data
    data = setup_data(p)
    
    # Update axis 1 (top left - win probability curves with value dots)
    update_axis1_win_probability(chart, data)
    
    # Update axis 3 (bottom left - weighted sum curve)
    update_axis3_bottom_left(chart, data)


class Chart(FigureCanvas):
    def __init__(self):
        # Create figures for charts
        self.fig1 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig3 = Figure(figsize=(6, 4.2), dpi=100)
        
        # Create individual axes for each chart
        self.axis1 = self.fig1.add_subplot(111)
        self.axis3 = self.fig3.add_subplot(111)
        
        # Adjust layout
        self.fig1.tight_layout()
        self.fig3.tight_layout()
        
        self.p = Parameters()
        update_axis(self, self.p)
        
        # Use the first figure as the main canvas
        super().__init__(self.fig1)
        self.set_size_request(600, 400)
        plt.ion()

    def update(self):
        # Clear all axes
        self.axis1.clear()
        
        # Remove any existing twin axes for axis3
        for ax in self.axis3.figure.axes:
            if ax != self.axis3 and ax.bbox.bounds == self.axis3.bbox.bounds:
                self.axis3.figure.delaxes(ax)
        self.axis3.clear()
        
        update_axis(self, self.p)

        # Adjust layout
        self.fig1.tight_layout()
        self.fig3.tight_layout()

        # Redraw all figures
        self.fig1.canvas.draw()
        self.fig3.canvas.draw()
        
        # Redraw the main canvas
        self.draw()


class MainWindow(Gtk.ApplicationWindow):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.set_default_size(1400, 850)
        self.set_title("Viewability Simulation")

        self.box1 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.box2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.box3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        
        # Set fixed width for the left panel (box2)
        self.box2.set_size_request(350, -1)
        self.box2.set_hexpand(False)

        self.set_child(self.box1)
        self.box1.append(self.box2)
        self.box1.append(self.box3)
        
        # Create charts
        self.chart = Chart()
        
        # Create grid for charts
        self.charts_grid = Gtk.Grid()
        self.charts_grid.set_row_spacing(10)
        self.charts_grid.set_column_spacing(10)
        
        # Create separate canvases for each chart
        self.canvas1 = FigureCanvas(self.chart.fig1)
        self.canvas3 = FigureCanvas(self.chart.fig3)
        
        # Add canvases to grid
        self.charts_grid.attach(self.canvas1, 0, 0, 1, 1)
        self.charts_grid.attach(self.canvas3, 0, 1, 1, 1)
        
        self.box3.append(self.charts_grid)
        self.check = Gtk.Label(label="Parameters")
        self.box2.append(self.check)
        
        # Store references to controls for updating later
        self.controls = {}
        self.box2.append(self.slider_box("Sigma A value", 0.0, 1.0, 'sigmoid_A_value'))
        self.box2.append(self.slider_box("Sigma A scale", 0.001, 2.0, 'sigmoid_A_scale'))
        self.box2.append(self.slider_box("Sigma A offset", 0.0, 20.0, 'sigmoid_A_offset'))
        self.box2.append(self.slider_box("Sigma B value", 0.0, 1.0, 'sigmoid_B_value'))
        self.box2.append(self.slider_box("Sigma B scale", 0.001, 2.0, 'sigmoid_B_scale'))
        self.box2.append(self.slider_box("Sigma B offset", 0.0, 20.0, 'sigmoid_B_offset'))

    def slider_box(self, label, start_range, end_range, parameter_name):
        assert parameter_name in self.chart.p.__dict__.keys()
        start_value = self.chart.p.__dict__[parameter_name]
        slider = Gtk.Scale()
        slider.set_digits(2)
        slider.set_range(start_range, end_range)
        slider.set_draw_value(True)
        slider.set_value(start_value)
        
        def change_function(self):
            new_value = float(slider.get_value())
            # Clamp value parameters to [0, 1.0]
            if parameter_name in ['sigmoid_A_value', 'sigmoid_B_value']:
                new_value = max(0.0, min(1.0, new_value))
            self.chart.p.__dict__[parameter_name] = new_value
            self.chart.update()
        
        slider.chart = self.chart
        signal_id = slider.connect('value-changed', change_function)
        slider.set_hexpand(True)
        
        b = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        b.append(Gtk.Label(label=label))
        b.append(slider)
        
        # Store reference to control and signal ID for updating later
        self.controls[parameter_name] = {'widget': slider, 'signal_id': signal_id}
        
        return b


class MyApp(Adw.Application):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.connect('activate', self.on_activate)
        
        # Set up signal handler for Ctrl+C
        signal.signal(signal.SIGINT, self.signal_handler)

    def signal_handler(self, signum, frame):
        """Handle Ctrl+C gracefully"""
        print("\nReceived Ctrl+C, shutting down gracefully...")
        self.quit()

    def on_activate(self, app):
        self.win = MainWindow(application=app)
        self.win.present()


if __name__ == '__main__':
    app = MyApp(application_id="com.example.ViewabilitySimulation")
    app.run(sys.argv)

