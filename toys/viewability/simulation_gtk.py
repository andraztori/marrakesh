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
        self.sigmoid_C_value = 0.7
        self.sigmoid_C_scale = 0.6
        self.sigmoid_C_offset = 7.0
        self.isohypsis_value = 0.66


def setup_data(p: Parameters):
    """Setup sigmoid objects and compute data.
    For each CPM A and CPM B, find CPM C such that prob_A + prob_B + prob_C = 1.0"""
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, p.sigmoid_A_value)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset, p.sigmoid_B_value)
    s_C = Sigmoid(p.sigmoid_C_scale, p.sigmoid_C_offset, p.sigmoid_C_value)

    # Initialize data for 3D surface
    # We'll create a grid of CPM A and CPM B values, then find CPM C for each
    cpm_A_range = np.arange(0, MAX_CPM, STEP)
    cpm_B_range = np.arange(0, MAX_CPM, STEP)
    
    # Create meshgrid for surface plot
    CPM_A, CPM_B = np.meshgrid(cpm_A_range, cpm_B_range)
    WEIGHTED_SUM = np.zeros_like(CPM_A)
    CPM_C = np.zeros_like(CPM_A)
    VALID_MASK = np.zeros_like(CPM_A, dtype=bool)
    
    # For each combination of CPM A and CPM B, find CPM C such that prob_A + prob_B + prob_C = 1.0
    for i, cpm_A in enumerate(cpm_A_range):
        prob_A = s_A.get_probability(cpm_A)
        
        for j, cpm_B in enumerate(cpm_B_range):
            prob_B = s_B.get_probability(cpm_B)
            
            # Calculate required prob_C to make prob_A + prob_B + prob_C = 1.0
            prob_C_required = 1.0 - prob_A - prob_B
            
            # Check if prob_C_required is valid (between 0 and 1)
            if prob_C_required < 0.0 or prob_C_required > 1.0:
                WEIGHTED_SUM[j, i] = np.nan
                CPM_C[j, i] = np.nan
                continue
            
            # Find CPM C using inverse sigmoid
            try:
                cpm_C = s_C.inverse(prob_C_required)
                
                # Only include valid CPM C values (non-negative and reasonable)
                if cpm_C < 0.0 or cpm_C > MAX_CPM * 2:
                    WEIGHTED_SUM[j, i] = np.nan
                    CPM_C[j, i] = np.nan
                    continue
                    
                prob_C = s_C.get_probability(cpm_C)
                
                # Verify that prob_A + prob_B + prob_C = 1.0 (within tolerance)
                sum_probs = prob_A + prob_B + prob_C
                if abs(sum_probs - 1.0) > 0.01:  # Check constraint is satisfied
                    WEIGHTED_SUM[j, i] = np.nan
                    CPM_C[j, i] = np.nan
                    continue
                
                # Calculate weighted sum of values divided by sum of probabilities
                # Since sum_probs = 1.0, we can simplify to:
                weighted_sum = s_A.value * prob_A + s_B.value * prob_B + s_C.value * prob_C
                
                WEIGHTED_SUM[j, i] = weighted_sum
                CPM_C[j, i] = cpm_C
                VALID_MASK[j, i] = True
            except:
                # Skip if inverse calculation fails
                WEIGHTED_SUM[j, i] = np.nan
                CPM_C[j, i] = np.nan
                continue

    return {
        's_A': s_A,
        's_B': s_B,
        's_C': s_C,
        'CPM_A': CPM_A,
        'CPM_B': CPM_B,
        'CPM_C': CPM_C,
        'WEIGHTED_SUM': WEIGHTED_SUM,
        'VALID_MASK': VALID_MASK,
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
    prob_C_full = []
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        prob_range.append(cpm)
        prob_A_full.append(data['s_A'].get_probability(cpm))
        prob_B_full.append(data['s_B'].get_probability(cpm))
        prob_C_full.append(data['s_C'].get_probability(cpm))
    
    # Plot win probability curves
    line1 = a.plot(prob_range, prob_A_full, 'C0-', label='Win probability A')
    line2 = a.plot(prob_range, prob_B_full, 'C1-', label='Win probability B')
    line3 = a.plot(prob_range, prob_C_full, 'C2-', label='Win probability C')
    
    # Draw value for each impression as a dot on the vertical axis (x=0)
    a.plot(0, data['s_A'].value, 'C0s', markersize=10, label='Value A')
    a.plot(0, data['s_B'].value, 'C1s', markersize=10, label='Value B')
    a.plot(0, data['s_C'].value, 'C2s', markersize=10, label='Value C')
    
    a.set_xlabel('CPM')
    a.set_ylabel('Probability / Value')
    a.legend(loc='lower right')
    a.grid(True, alpha=0.3)


def update_axis3_cpm_c_surface(chart, data):
    """Update axis 3 (left) with CPM C 3D surface."""
    a = chart.axis3
    a.clear()
    
    # Create 3D surface plot for CPM C
    # X-axis: CPM A, Y-axis: CPM B, Z-axis: CPM C
    surf = a.plot_surface(data['CPM_A'], data['CPM_B'], data['CPM_C'], 
                          cmap='plasma', alpha=0.8, linewidth=0, antialiased=True)
    
    a.set_xlabel('CPM A')
    a.set_ylabel('CPM B')
    a.set_zlabel('CPM C')
    a.set_title('CPM C Surface')
    
    # Add colorbar and store reference
    chart.axis3_colorbar = chart.fig3.colorbar(surf, ax=a, shrink=0.5, aspect=5)
    chart.axis3_colorbar.set_label('CPM C', rotation=270, labelpad=15)


def update_axis4_weighted_sum_surface(chart, data, isohypsis_value):
    """Update axis 4 (right) with weighted sum 3D surface."""
    a = chart.axis4
    a.clear()
    
    # Create 3D surface plot for weighted sum
    # X-axis: CPM A, Y-axis: CPM B, Z-axis: Weighted sum
    surf = a.plot_surface(data['CPM_A'], data['CPM_B'], data['WEIGHTED_SUM'], 
                          cmap='viridis', alpha=0.8, linewidth=0, antialiased=True)
    
    # Draw isohypsis (contour lines) for weighted sum at the specified value
    # Draw contour lines on the surface itself
    cont = a.contour(data['CPM_A'], data['CPM_B'], data['WEIGHTED_SUM'], 
                     levels=[isohypsis_value], colors='red', linewidths=3, alpha=1.0, zdir='z')
    
    a.set_xlabel('CPM A')
    a.set_ylabel('CPM B')
    a.set_zlabel('Weighted Sum')
    a.set_title(f'Weighted Sum Surface (isohypsis at {isohypsis_value:.2f})')
    
    # Add colorbar and store reference
    chart.axis4_colorbar = chart.fig4.colorbar(surf, ax=a, shrink=0.5, aspect=5)
    chart.axis4_colorbar.set_label('Weighted Sum', rotation=270, labelpad=15)


def update_axis(chart, p: Parameters):
    # Setup data
    data = setup_data(p)
    
    # Update axis 1 (top left - win probability curves with value dots)
    update_axis1_win_probability(chart, data)
    
    # Update axis 3 (bottom left - CPM C surface)
    update_axis3_cpm_c_surface(chart, data)
    
    # Update axis 4 (bottom right - weighted sum surface)
    update_axis4_weighted_sum_surface(chart, data, p.isohypsis_value)


class Chart(FigureCanvas):
    def __init__(self):
        # Create figures for charts
        self.fig1 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig3 = Figure(figsize=(6, 4.2), dpi=100)  # Left 3D chart for CPM C
        self.fig4 = Figure(figsize=(6, 4.2), dpi=100)  # Right 3D chart for weighted sum
        
        # Create individual axes for each chart
        self.axis1 = self.fig1.add_subplot(111)
        self.axis3 = self.fig3.add_subplot(111, projection='3d')
        self.axis4 = self.fig4.add_subplot(111, projection='3d')
        
        # Store colorbar references for proper removal
        self.axis3_colorbar = None
        self.axis4_colorbar = None
        
        # Adjust layout
        self.fig1.tight_layout()
        self.fig3.tight_layout()
        self.fig4.tight_layout()
        
        self.p = Parameters()
        update_axis(self, self.p)
        
        # Use the first figure as the main canvas
        super().__init__(self.fig1)
        self.set_size_request(600, 400)
        plt.ion()

    def update(self):
        # Clear all axes
        self.axis1.clear()
        
        # Remove existing colorbars
        if self.axis3_colorbar is not None:
            self.axis3_colorbar.remove()
            self.axis3_colorbar = None
        if self.axis4_colorbar is not None:
            self.axis4_colorbar.remove()
            self.axis4_colorbar = None
        
        # Clear 3D axes
        self.axis3.clear()
        self.axis4.clear()
        
        update_axis(self, self.p)

        # Adjust layout
        self.fig1.tight_layout()
        self.fig3.tight_layout()
        self.fig4.tight_layout()

        # Redraw all figures
        self.fig1.canvas.draw()
        self.fig3.canvas.draw()
        self.fig4.canvas.draw()
        
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
        self.canvas3 = FigureCanvas(self.chart.fig3)  # Left 3D chart (CPM C)
        self.canvas4 = FigureCanvas(self.chart.fig4)  # Right 3D chart (Weighted Sum)
        
        # Add canvases to grid
        self.charts_grid.attach(self.canvas1, 0, 0, 2, 1)  # Top chart spans both columns
        self.charts_grid.attach(self.canvas3, 0, 1, 1, 1)  # Bottom left (CPM C)
        self.charts_grid.attach(self.canvas4, 1, 1, 1, 1)  # Bottom right (Weighted Sum)
        
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
        self.box2.append(self.slider_box("Sigma C value", 0.0, 1.0, 'sigmoid_C_value'))
        self.box2.append(self.slider_box("Sigma C scale", 0.001, 2.0, 'sigmoid_C_scale'))
        self.box2.append(self.slider_box("Sigma C offset", 0.0, 20.0, 'sigmoid_C_offset'))
        self.box2.append(self.slider_box("Isohypsis value", 0.0, 1.0, 'isohypsis_value'))

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
            if parameter_name in ['sigmoid_A_value', 'sigmoid_B_value', 'sigmoid_C_value']:
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

