import tkinter as tk
from tkinter import ttk
import sys
import math
import signal
import numpy as np
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
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


class Chart:
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


class MainWindow:
    def __init__(self, root):
        self.root = root
        self.root.title("Viewability Simulation")
        self.root.geometry("1400x850")
        
        # Create main horizontal container
        main_frame = tk.Frame(root)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Left panel for parameters
        left_panel = tk.Frame(main_frame, width=350)
        left_panel.pack(side=tk.LEFT, fill=tk.Y, padx=(0, 10))
        left_panel.pack_propagate(False)
        
        # Right panel for charts
        right_panel = tk.Frame(main_frame)
        right_panel.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        # Parameters label
        params_label = tk.Label(left_panel, text="Parameters", font=('Arial', 12, 'bold'))
        params_label.pack(pady=10)
        
        # Create charts
        self.chart = Chart()
        
        # Create separate canvases for each chart
        self.canvas1 = FigureCanvasTkAgg(self.chart.fig1, master=right_panel)
        self.canvas3 = FigureCanvasTkAgg(self.chart.fig3, master=right_panel)
        self.canvas4 = FigureCanvasTkAgg(self.chart.fig4, master=right_panel)
        
        # Pack canvases in grid layout
        self.canvas1.get_tk_widget().grid(row=0, column=0, columnspan=2, padx=5, pady=5)
        self.canvas3.get_tk_widget().grid(row=1, column=0, padx=5, pady=5)
        self.canvas4.get_tk_widget().grid(row=1, column=1, padx=5, pady=5)
        
        # Store references to controls
        self.controls = {}
        
        # Create sliders
        self.create_slider(left_panel, "Sigma A value", 0.0, 1.0, 'sigmoid_A_value')
        self.create_slider(left_panel, "Sigma A scale", 0.001, 2.0, 'sigmoid_A_scale')
        self.create_slider(left_panel, "Sigma A offset", 0.0, 20.0, 'sigmoid_A_offset')
        self.create_slider(left_panel, "Sigma B value", 0.0, 1.0, 'sigmoid_B_value')
        self.create_slider(left_panel, "Sigma B scale", 0.001, 2.0, 'sigmoid_B_scale')
        self.create_slider(left_panel, "Sigma B offset", 0.0, 20.0, 'sigmoid_B_offset')
        self.create_slider(left_panel, "Sigma C value", 0.0, 1.0, 'sigmoid_C_value')
        self.create_slider(left_panel, "Sigma C scale", 0.001, 2.0, 'sigmoid_C_scale')
        self.create_slider(left_panel, "Sigma C offset", 0.0, 20.0, 'sigmoid_C_offset')
        self.create_slider(left_panel, "Isohypsis value", 0.0, 1.0, 'isohypsis_value')
        
    def create_slider(self, parent, label, start_range, end_range, parameter_name):
        """Create a slider with label."""
        assert parameter_name in self.chart.p.__dict__.keys()
        start_value = self.chart.p.__dict__[parameter_name]
        
        # Create frame for slider
        frame = tk.Frame(parent)
        frame.pack(fill=tk.X, pady=5)
        
        # Label
        label_widget = tk.Label(frame, text=label, width=20, anchor='w')
        label_widget.pack(side=tk.LEFT, padx=5)
        
        # Scale (slider)
        scale = tk.Scale(frame, from_=start_range, to=end_range, 
                        resolution=0.01, orient=tk.HORIZONTAL,
                        length=200, command=lambda v: self.on_slider_change(parameter_name, v))
        scale.set(start_value)
        scale.pack(side=tk.LEFT, fill=tk.X, expand=True)
        
        # Value label
        value_label = tk.Label(frame, text=f"{start_value:.2f}", width=8)
        value_label.pack(side=tk.LEFT, padx=5)
        
        # Store reference
        self.controls[parameter_name] = {
            'scale': scale,
            'value_label': value_label
        }
    
    def on_slider_change(self, parameter_name, value):
        """Handle slider value change."""
        new_value = float(value)
        
        # Clamp value parameters to [0, 1.0]
        if parameter_name in ['sigmoid_A_value', 'sigmoid_B_value', 'sigmoid_C_value']:
            new_value = max(0.0, min(1.0, new_value))
        
        # Update parameter
        self.chart.p.__dict__[parameter_name] = new_value
        
        # Update value label
        self.controls[parameter_name]['value_label'].config(text=f"{new_value:.2f}")
        
        # Update charts
        self.chart.update()
        self.canvas1.draw()
        self.canvas3.draw()
        self.canvas4.draw()


def main():
    root = tk.Tk()
    app = MainWindow(root)
    
    # Handle window close
    def on_closing():
        root.quit()
        root.destroy()
    
    root.protocol("WM_DELETE_WINDOW", on_closing)
    root.mainloop()


if __name__ == '__main__':
    main()
