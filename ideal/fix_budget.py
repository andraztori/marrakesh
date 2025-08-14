import gi,sys
gi.require_version('Adw', '1')
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, Adw, GLib

import math
import signal
from matplotlib.backends.backend_gtk4agg import \
    FigureCanvasGTK4Agg as FigureCanvas
from matplotlib.figure import Figure
import matplotlib.pyplot as plt


EPSILON = 0.0001
MAX_CPM = 20
STEP = 0.05
BUDGET_STEP = 0.25

class Sigmoid:
    def __init__(self, scale, offset, value):
        self.scale = scale
        self.offset = offset
        self.value = value
    
    def get_probability(self, x):
        return 1.0/(1+math.exp(-(x-self.offset)*self.scale)) 

    def bisect_spend_inverse(self, y):
        min_x = 0.0
        max_x = 100.0
        a = -100
        steps = 0
        #print("Looking for y=%f" % y)
        while math.fabs(a - y) > 0.000001:
            steps += 1
#            print("step: %i, min_x: %f, max_x: %f, a: %f" % (steps, min_x, max_x, a))
            x = (min_x + max_x) / 2.0
            a = self.get_probability(x) * x
            if a > y:
                max_x = x
            else:
                min_x = x
                
            if steps > 50:
                raise ("Didn't find the inverse of %f" % y) 
        return x
        

    def inverse(self, y):
#        return math.log(y / (1 - y)) / self.scale + self.offset
#        y = y / self.amplitude
        if y<EPSILON/10:
            y = EPSILON/10
        if 1.0 - y <= EPSILON/10:
            y = 1.0 -EPSILON/10
        return (math.log(y) - math.log(1 - y)) / self.scale + self.offset

    def numeric_derivative(self, x):
        E = 0.00001
        a1 = self.get_probability(x-E)
        a2 = self.get_probability(x+E)
        return (a2-a1) / (2 * E)

    def numeric_derivative_mul_x(self, x):
        E = 0.00001
        x1 = x-E
        x2 = x+E
        a1 = self.get_probability(x1)*x1*.5
        a2 = self.get_probability(x2)*x2*.5
        return (a2-a1) / (2 * E)


class Parameters:
    def __init__(self):
        self.total_budget = 2
        self.total_volume = 2
        self.sigmoid_A_value = 1.0
        self.sigmoid_A_scale = 0.4
        self.sigmoid_A_offset = 8.0
        self.sigmoid_B_value = 1.0
        self.sigmoid_B_scale = 0.9
        self.sigmoid_B_offset = 9.0
        self.percent_A = 0.5
        

    def interesting_curves(self):
        self.total_budget = 14
        self.total_volume = 2
        self.sigmoid_A_value = 1.0
        self.sigmoid_B_value = 1.0
        self.percent_A = 0.5
        self.sigmoid_A_scale=0.5
        self.sigmoid_A_offset=9.0
        self.sigmoid_B_scale=1.0
        self.sigmoid_B_offset=7.0

    def total_volume_A(self):
        return self.percent_A * self.total_volume	# total volume = 2 impressions
    
    def total_volume_B(self):
        return (1.0 -self.percent_A) * self.total_volume
        


def update_axis(axis, p: Parameters, save_to_png=False, show_value_vs_budget=False):
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, p.sigmoid_A_value)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset, p.sigmoid_B_value) 

    l1 = []
    l_cpm_B = []
    l_imp_bought_A = []
    l_imp_bought_B = []
    l_total_value = []
    l_prob_A = []
    l_prob_B = []
    l_prob_Binv = []
    l_prob_range = []
    l_inv_1 = []
    l_inv_2 = []
    l_inv_3 = []
    l_budget_range = []  # For x-axis of value vs budget curve
    l_max_value = []     # For y-axis of value vs budget curve
    l_value_per_cost = [] # For efficiency curve

    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        l_prob_range.append(cpm)
        l_prob_A.append(s_A.get_probability(cpm))
        l_prob_B.append(s_B.get_probability(cpm))

    l2 = []
    l22 = []
    '''
    for percent_to_buy_x in range(0, 20):
        percent_to_buy = percent_to_buy_x / 10.0
        min_cost = 1000.0
        min_cost_cpm_A = 0
        min_cost_cpm_B = 0

        for cpm_input in range(0, int(MAX_CPM / STEP)):
            cpm_A = 1.0 * cpm_input * STEP
            imp_bought_A = s_A.get_probability(cpm_A) * p.total_volume_A()
            if percent_to_buy - imp_bought_A > 1.0: # impossible 
                continue
            cpm_B = s_B.inverse((percent_to_buy - imp_bought_A) / p.total_volume_B())
            if cpm_B <= 0.0:
                continue
            imp_bought_B = s_B.get_probability(cpm_B) * p.total_volume_B()
            
            if math.fabs((imp_bought_A + imp_bought_B) - percent_to_buy) > EPSILON:	# make sure we've bought enough impressions
                # it was impossible to buy, just fill in with empty values
                continue
     
            total_cost = cpm_A * imp_bought_A + cpm_B * imp_bought_B # this is the function we are trying to find zero-derivate of
            if total_cost < min_cost:
                min_cost = total_cost
                min_cost_cpm_A = cpm_A
                min_cost_cpm_B = cpm_B
        if min_cost != 1000.0:
            l2.append(percent_to_buy)
            l22.append(min_cost)

    '''
    max_value = 0.0
    min_cost_cpm_A = 0
    min_cost_cpm_B = 0
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm_A = 1.0 * cpm_input * STEP
        imp_bought_A = s_A.get_probability(cpm_A) * p.total_volume_A()
        imp_value_A = imp_bought_A * s_A.value
        imp_spend_A = imp_bought_A * cpm_A
        
        if p.total_budget - imp_spend_A < 0.0: # impossible 
            continue
        imp_spend_B = p.total_budget - imp_spend_A
        
        if imp_spend_B < 0:
            continue
        cpm_B = s_B.bisect_spend_inverse(imp_spend_B / p.total_volume_B())
        if cpm_B <= 0.0:
            continue
        imp_bought_B = s_B.get_probability(cpm_B) * p.total_volume_B()
        imp_value_B = imp_bought_B * s_B.value
#        imp_spend_B2 = imp_bought_B * cpm_B
#        print("imp spend B1: %f, B2: %f" % (imp_spend_B, imp_spend_B2))
        #print("A: bought: %f, value: %f, spend: %f" % (imp_bought_A, imp_value_A, imp_spend_A))
        #print("B: bought: %f, value: %f, spend: %f" % (imp_bought_B, imp_value_B, imp_spend_B))
        
        total_spend = imp_spend_A + imp_spend_B
      
        #print("total_spend: %f, total budget: %f" % (total_spend, p.total_budget))
        if math.fabs(total_spend - p.total_budget) > EPSILON:	# make sure we've bought enough impressions
            # it was impossible to buy, just fill in with empty values
        #    print("A")
            continue
        
 
        total_value = imp_value_A + imp_value_B
        if total_value > max_value:
            max_value = total_value
            min_cost_cpm_A = cpm_A
            min_cost_cpm_B = cpm_B
        l1.append(cpm_A) 
        l_total_value.append(total_value)
        l_cpm_B.append(cpm_B)
        l_imp_bought_A.append(imp_bought_A)
        l_imp_bought_B.append(imp_bought_B)
    
        
    invariant_A = s_A.numeric_derivative(min_cost_cpm_A) / s_A.numeric_derivative_mul_x(min_cost_cpm_A) * s_A.value
    invariant_B = s_B.numeric_derivative(min_cost_cpm_B) / s_B.numeric_derivative_mul_x(min_cost_cpm_B) * s_B.value


    print ("I1: %f, I2: %f" % (invariant_A, invariant_B))
    #print ("D1: %f, D2: %f" % (num_der_A, num_der_B))
    #print ("C1: %f, C2: %f" % (min_cost_cpm_A, min_cost_cpm_B))

    a = axis[0][0]
    a.set_xlim(right = MAX_CPM)
    line1 = a.plot(l_prob_range, l_prob_A, 'C0-', label='Win probability A')
    line2 = a.plot(l_prob_range, l_prob_B, 'C1-', label='Win probability B')
    # Add both vertical lines and points using same colors
    a.vlines(min_cost_cpm_A, 0, s_A.get_probability(min_cost_cpm_A), 'C0', linestyles='--', alpha=0.5)
    a.vlines(min_cost_cpm_B, 0, s_B.get_probability(min_cost_cpm_B), 'C1', linestyles='--', alpha=0.5)
    a.plot(min_cost_cpm_A, s_A.get_probability(min_cost_cpm_A), 'C0o', label='Optimal Bid A')
    a.plot(min_cost_cpm_B, s_B.get_probability(min_cost_cpm_B), 'C1o', label='Optimal Bid B')
    
    a.legend(loc='lower right') 

    a = axis[0][1]
    
    a.set_ylim(top = max(l_cpm_B))
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_total_value, label='Total cost')
    a.plot(l1, l_cpm_B, label='Cpm B')
    a.vlines(min_cost_cpm_A, 0, MAX_CPM, colors='C0')
    a.hlines(max_value, 0, MAX_CPM, colors='C0')
    a.legend(loc='upper right')

    a = axis[1][0]
    
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_total_value, label='Value vs. CPM A')
    a.hlines(max_value, 0, MAX_CPM, colors='C1')
    a.legend()
   
    '''
    a = axis[0][1]
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_imp_bought_A, label='Imp A bought')
    a.plot(l1, l_imp_bought_B, label='Imp B bought')
    a.vlines(min_cost_cpm_A, 0, p.percent_to_buy, colors='C0')
    a.legend()
    '''
    a = axis[1][1]
    a.set_xlim(right = 1.0)
    a.plot(l2, l22, label='Value for budget')
#    a.vlines(min_cost_cpm_A, 0, p.percent_to_buy, colors='C0')
    a.legend()

    # Calculate value vs budget curve
    if show_value_vs_budget: 
        for budget_x in range(0, int(10.0/BUDGET_STEP)):
            budget = budget_x * BUDGET_STEP
            l_budget_range.append(budget)
            
            # Find maximum value achievable with this budget
            max_value_for_budget = 0.0
            for cpm_input in range(0, int(MAX_CPM / STEP)):
                cpm_A = 1.0 * cpm_input * STEP
                imp_bought_A = s_A.get_probability(cpm_A) * p.total_volume_A() 
                imp_value_A = imp_bought_A * s_A.value
                imp_spend_A = imp_bought_A * cpm_A
                
                if budget - imp_spend_A < 0.0:  # Over budget
                    continue
                    
                imp_spend_B = budget - imp_spend_A
                if imp_spend_B < 0:
                    continue
                    
                cpm_B = s_B.bisect_spend_inverse(imp_spend_B / p.total_volume_B())
                if cpm_B <= 0.0:
                    continue
                    
                imp_bought_B = s_B.get_probability(cpm_B) * p.total_volume_B()
                imp_value_B = imp_bought_B * s_B.value
                
                total_value = imp_value_A + imp_value_B
                max_value_for_budget = max(max_value_for_budget, total_value)
            
            l_max_value.append(max_value_for_budget)
            # Calculate value per cost (efficiency), avoiding division by zero
            if budget > 0:
                l_value_per_cost.append(max_value_for_budget/budget)
            else:
                l_value_per_cost.append(0)
        
        # Plot value vs budget curve in bottom right with two y-axes
        a = axis[1][1]
        a.clear()
        # Remove any existing second y-axis before creating a new one
        for ax in a.figure.axes:
            if ax != a and ax.bbox.bounds == a.bbox.bounds:
                a.figure.delaxes(ax)
                    
        a.set_xlim(right=10.0)
        
        # Plot total value on left y-axis
        line1 = a.plot(l_budget_range, l_max_value, 'b-', label='Max value')
        a.set_xlabel('Budget')
        a.set_ylabel('Maximum achievable value', color='b')
        a.tick_params(axis='y', labelcolor='b')
        
        # Create second y-axis for value per cost
        ax2 = a.twinx()
        line2 = ax2.plot(l_budget_range, l_value_per_cost, 'r-', label='Value per cost')
        ax2.set_ylabel('Value per cost', color='r')
        ax2.tick_params(axis='y', labelcolor='r')
        
        # Mark current budget point on both curves
        current_value = max_value if max_value is not None else 0
        current_efficiency = current_value/p.total_budget if p.total_budget > 0 else 0
        a.plot(p.total_budget, current_value, 'bo', label='Current value')
        ax2.plot(p.total_budget, current_efficiency, 'ro', label='Current efficiency')
        
        # Add combined legend
        lines = line1 + line2
        labels = [l.get_label() for l in lines]
        a.legend(lines, labels, loc='upper right')
    else:
        # Clear the bottom-right pane when toggle is disabled
        a = axis[1][1]
        a.clear()
        # Remove any existing second y-axis
        for ax in a.figure.axes:
            if ax != a and ax.bbox.bounds == a.bbox.bounds:
                a.figure.delaxes(ax)
        
    # Save plots to PNG if requested
    if save_to_png:
        # Get the figures from the axes
        fig1 = axis[0][0].figure
        fig2 = axis[1][0].figure
        fig3 = axis[0][1].figure
        fig4 = axis[1][1].figure
        
        # Create parameter string for filename
        param_str = f"tb{p.total_budget}_tv{p.total_volume}_pa{p.percent_A}_sav{p.sigmoid_A_value}_sas{p.sigmoid_A_scale}_sao{p.sigmoid_A_offset}_sbv{p.sigmoid_B_value}_sbs{p.sigmoid_B_scale}_sbo{p.sigmoid_B_offset}"
        
        # Save each figure with parameter string in filename
        fig1.savefig(f'plot1_win_probability_{param_str}.png', dpi=300, bbox_inches='tight')
        fig2.savefig(f'plot2_total_value_{param_str}.png', dpi=300, bbox_inches='tight')
        fig3.savefig(f'plot3_total_cost_{param_str}.png', dpi=300, bbox_inches='tight')
        fig4.savefig(f'plot4_min_price_{param_str}.png', dpi=300, bbox_inches='tight')
        
        print("Plots saved as PNG files")
    

class Chart(FigureCanvas):
    def __init__(self):
        # Create four separate figures instead of one with subplots
        self.fig1 = Figure(figsize=(6, 4), dpi=100)
        self.fig2 = Figure(figsize=(6, 4), dpi=100)
        self.fig3 = Figure(figsize=(6, 4), dpi=100)
        self.fig4 = Figure(figsize=(6, 4), dpi=100)
        
        # Create individual axes for each chart
        self.axis1 = self.fig1.add_subplot(111)
        self.axis2 = self.fig2.add_subplot(111)
        self.axis3 = self.fig3.add_subplot(111)
        self.axis4 = self.fig4.add_subplot(111)
        
        # Store axes in a 2x2 array for compatibility with existing update_axis function
        self.axis = [[self.axis1, self.axis3], [self.axis2, self.axis4]]
        
        self.p = Parameters()
        self.show_value_vs_budget = False  # Default to off
        update_axis(self.axis, self.p, show_value_vs_budget=self.show_value_vs_budget)
        
        # Use the first figure as the main canvas
        super().__init__(self.fig1)
        self.set_size_request(600, 400)
        plt.ion()

    def update(self, save_to_png=False):
        # Clear all axes
        for i in range(0, 2):
            for j in range(0, 2):
                self.axis[i][j].clear()
        update_axis(self.axis, self.p, save_to_png, show_value_vs_budget=self.show_value_vs_budget)

        # Redraw all figures
        self.fig1.canvas.draw()
        self.fig2.canvas.draw()
        self.fig3.canvas.draw()
        self.fig4.canvas.draw()
        
        # Redraw the main canvas
        self.draw()



class ChartContainer(Gtk.Box):
    def __init__(self):
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        
        self.p = Parameters()
        
        # Create grid layout
        self.grid = Gtk.Grid()
        self.grid.set_row_spacing(10)
        self.grid.set_column_spacing(10)
        
        self.append(self.grid)
        
        # Initial update
        self.update()
        plt.ion()


    def update(self):
        #print("updating to: ", x)
        for i in range(0,2):
            for j in range(0, 2):
                self.axis[i,j].clear()
        update_axis(self.axis, self.p)

        #self.ax.draw(1)
        #self.fig.draw()
        self.draw()
        #self.ion()


class MainWindow(Gtk.ApplicationWindow):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Things will go here
        self.set_default_size(1400, 850)
        self.set_title("BestBuy")

        self.box1 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing = 10)
        self.box2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.box3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        
        # Set fixed width for the left panel (box2)
        self.box2.set_size_request(350, -1)
        self.box2.set_hexpand(False)
        

        self.set_child(self.box1)  # Horizontal box to window
        self.box1.append(self.box2)  # Put vert box in that box
        self.box1.append(self.box3)  # And another one, empty for now

 #       self.box2.append(self.button) # Put button in the first of the two vertical boxes
        
        
        # Create charts
        self.chart = Chart()
        
        # Create grid for charts
        self.charts_grid = Gtk.Grid()
        self.charts_grid.set_row_spacing(10)
        self.charts_grid.set_column_spacing(10)
        
        # Create separate canvases for each chart
        self.canvas1 = FigureCanvas(self.chart.fig1)
        self.canvas2 = FigureCanvas(self.chart.fig2)
        self.canvas3 = FigureCanvas(self.chart.fig3)
        self.canvas4 = FigureCanvas(self.chart.fig4)
        
        # Set size for each canvas
        '''self.canvas1.set_size_request(600, 400)
        self.canvas2.set_size_request(600, 400)
        self.canvas3.set_size_request(600, 400)
        self.canvas4.set_size_request(600, 400)
        '''
        # Add canvases to grid
        self.charts_grid.attach(self.canvas1, 0, 0, 1, 1)
        self.charts_grid.attach(self.canvas2, 1, 0, 1, 1)
        self.charts_grid.attach(self.canvas3, 0, 1, 1, 1)
        self.charts_grid.attach(self.canvas4, 1, 1, 1, 1)
        
        self.box3.append(self.charts_grid)
        self.check = Gtk.Label(label="Parameters")
        self.box2.append(self.check)
        
        # Store references to sliders for updating later
        self.sliders = {}
        self.box2.append(self.slider_box("Percent of A", 0.0, 1.0, 'percent_A'))
        self.box2.append(self.slider_box("Total budget", 0.0, 10.0, 'total_budget'))
        self.box2.append(self.slider_box("Sigma A value", 0.001, 5.0, 'sigmoid_A_value'))
        self.box2.append(self.slider_box("Sigma A scale", 0.001, 2.0, 'sigmoid_A_scale'))
        self.box2.append(self.slider_box("Sigma A offset", 0.0, 20.0, 'sigmoid_A_offset'))
        self.box2.append(self.slider_box("Sigma B value", 0.001, 5.0, 'sigmoid_B_value'))
        self.box2.append(self.slider_box("Sigma B scale", 0.001, 2.0, 'sigmoid_B_scale'))
        self.box2.append(self.slider_box("Sigma B offset", 0.0, 20.0, 'sigmoid_B_offset'))
        
        # Add Value vs. budget curve toggle
        self.value_budget_toggle = Gtk.CheckButton(label="Value vs. budget curve")
        self.value_budget_toggle.set_active(False)  # Default to off
        self.value_budget_toggle.connect('toggled', self.on_value_budget_toggle)
        self.box2.append(self.value_budget_toggle)
        
        # Add Save Plots button
        self.save_button = Gtk.Button(label="Save Plots to PNG")
        self.save_button.connect('clicked', self.save_plots)
        self.box2.append(self.save_button)
        
        # Add Interesting Curves button
        self.interesting_button = Gtk.Button(label="Load Interesting Curves")
        self.interesting_button.connect('clicked', self.load_interesting_curves)
        self.box2.append(self.interesting_button)
        

    def slider_box(self, label, start_range, end_range, parameter_name):
        assert parameter_name in self.chart.p.__dict__.keys()
        start_value = self.chart.p.__dict__[parameter_name]
        slider = Gtk.Scale()
        slider.set_digits(2)  # Number of decimal places to use
        slider.set_range(start_range, end_range)
        slider.set_draw_value(True)  # Show a label with current value
        slider.set_value(start_value)  # Sets the current value/position
        def change_function(self):
            self.chart.p.__dict__[parameter_name] = float(slider.get_value())
            self.chart.update()
        slider.chart = self.chart    

        signal_id = slider.connect('value-changed', change_function)
        slider.set_hexpand(True)
        b = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        b.append(Gtk.Label(label=label))
        b.append(slider)
        
        # Store reference to slider and signal ID for updating later
        self.sliders[parameter_name] = {'slider': slider, 'signal_id': signal_id}
        
        return b

    def save_plots(self, button):
        """Save all four plots as PNG files"""
        self.chart.update(save_to_png=True)

    def load_interesting_curves(self, button):
        """Load interesting curves and update charts"""
        self.chart.p.interesting_curves()
        
        # Update all sliders to reflect the new parameter values
        for parameter_name, slider_info in self.sliders.items():
            if parameter_name in self.chart.p.__dict__:
                slider = slider_info['slider']
                signal_id = slider_info['signal_id']
                # Temporarily block the signal to avoid triggering updates
                slider.handler_block(signal_id)
                slider.set_value(self.chart.p.__dict__[parameter_name])
                slider.handler_unblock(signal_id)
        
        self.chart.update()

    def on_value_budget_toggle(self, toggle_button):
        """Handle toggle for Value vs. budget curve"""
        self.chart.show_value_vs_budget = toggle_button.get_active()
        self.chart.update()




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
    app = MyApp(application_id="com.example.GtkApplication")
    app.run(sys.argv)

