import gi,sys
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
from mpl_toolkits.mplot3d import Axes3D


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
        a1 = self.get_probability(x1)*x1
        a2 = self.get_probability(x2)*x2
        return (a2-a1) / (2 * E)
    
    def marginal_utility_of_spend_numeric(self, x):
        if x > 0.0001:
            # numeric formula
            #return self.value * self.numeric_derivative(x) / self.numeric_derivative_mul_x(x)
            # formula from the article
            Wx = self.get_probability(x)
            Wdx = self.numeric_derivative(x)
            return self.value * Wdx / (Wdx * x + Wx)
        else:
            return 0.0

    # M() as per gemini, analytical formula...
    def M(self, x):
        s_val = self.get_probability(x) 
        if abs(1.0 - s_val) < 1e-15:
            return 0.0
        
        numerator = self.value * self.scale * (1.0 - s_val)
        denominator = self.scale * x * (1.0 - s_val) + 1.0
        
        if abs(denominator) < 1e-15:
            # This case is unlikely under normal parameters but is good practice
            return float('inf') if numerator > 0 else float('-inf')
            
        return numerator / denominator

    def marginal_utility_of_spend(self, x):
        return self.M(x)

    def M_prime(self, x):
        """The derivative of M(x)."""
        s_val = self.get_probability(x) 
        if abs(1.0 - s_val) < 1e-15:
            return 0.0

        numerator = -self.value * (self.scale**2) * (1.0 - s_val)
        denominator = (self.scale * x * (1.0 - s_val) + 1.0)**2
        
        if abs(denominator) < 1e-15:
             return float('-inf') # Derivative approaches -inf
             
        return numerator / denominator

    def marginal_utility_of_spend_inverse(self, y_target):
        # --- Newton-Raphson Iteration ---
        max_iterations = 100
        tolerance = 1e-6
        initial_guess = 10
        x = float(initial_guess)

        for i in range(max_iterations):
            # Calculate the value of M(x) and its derivative at the current x
            m_val = self.M(x)
            m_prime_val = self.M_prime(x)

            # The function whose root we are finding is f(x) = M(x) - y_target
            f_x = m_val - y_target

            # Avoid division by zero if the derivative is flat
            if abs(m_prime_val) < 1e-15:
                print(f"Warning: Derivative is close to zero at x={x}. Method cannot proceed.")
                return None

            # Newton-Raphson update step
            x_new = x - f_x / m_prime_val
            
            # Ensure x remains positive as per the problem constraint
            if x_new <= 0:
                # If we get a non-positive x, we can try halving the step
                # This is a simple modification to improve robustness
                x_new = x / 2.0

            # Check for convergence
            if abs(x_new - x) < tolerance:
                # print(f"Converged in {i+1} iterations.")
                return x_new

            x = x_new

#        print(f"Warning: Failed to converge within {max_iterations} iterations.")
        return x










class Parameters:
    def __init__(self):
        self.total_budget = 2
        self.total_volume = 1
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
        


def setup_optimization_data(p: Parameters):
    """Setup sigmoid objects and compute optimization data."""
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, p.sigmoid_A_value)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset, p.sigmoid_B_value) 

    # Initialize data lists
    l_cpm_A = []
    l_cpm_B = []
    l_imp_bought_A = []
    l_imp_bought_B = []
    l_total_value = []
    l_prob_A = []
    l_prob_B = []
    l_prob_range = []
    l_budget_range = []  # For x-axis of value vs budget curve
    l_max_value = []     # For y-axis of value vs budget curve
    l_value_per_cost = [] # For efficiency curve
    l_marginal_utility_A_budget = []  # Marginal utility A for each budget
    l_marginal_utility_B_budget = []  # Marginal utility B for each budget

    # Generate probability data for the range
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        l_prob_range.append(cpm)
        l_prob_A.append(s_A.get_probability(cpm))
        l_prob_B.append(s_B.get_probability(cpm))

    l2 = []
    l22 = []

    # Main optimization loop - find optimal CPM values
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
        
        total_spend = imp_spend_A + imp_spend_B
      
        if math.fabs(total_spend - p.total_budget) > EPSILON:	# make sure we've bought enough impressions
            continue
       
        total_value = imp_value_A + imp_value_B
        if total_value > max_value:
            max_value = total_value
            min_cost_cpm_A = cpm_A
            min_cost_cpm_B = cpm_B
        l_cpm_A.append(cpm_A) 
        l_total_value.append(total_value)
        l_cpm_B.append(cpm_B)
        l_imp_bought_A.append(imp_bought_A) 
        l_imp_bought_B.append(imp_bought_B)
    
    # Calculate marginal utility values at optimal points
    marginal_utility_of_spend_A = s_A.marginal_utility_of_spend(min_cost_cpm_A)
    marginal_utility_of_spend_B = s_B.marginal_utility_of_spend(min_cost_cpm_B)
    
    # Return all computed data as a dictionary
    return {
        's_A': s_A,
        's_B': s_B,
        'l_cpm_A': l_cpm_A,
        'l_cpm_B': l_cpm_B,
        'l_imp_bought_A': l_imp_bought_A,
        'l_imp_bought_B': l_imp_bought_B,
        'l_total_value': l_total_value,
        'l_prob_A': l_prob_A,
        'l_prob_B': l_prob_B,
        'l_prob_range': l_prob_range,
        'l_budget_range': l_budget_range,
        'l_max_value': l_max_value,
        'l_value_per_cost': l_value_per_cost,
        'l_marginal_utility_A_budget': l_marginal_utility_A_budget,
        'l_marginal_utility_B_budget': l_marginal_utility_B_budget,
        'l2': l2,
        'l22': l22,
        'max_value': max_value,
        'min_cost_cpm_A': min_cost_cpm_A,
        'min_cost_cpm_B': min_cost_cpm_B,
        'marginal_utility_of_spend_A': marginal_utility_of_spend_A,
        'marginal_utility_of_spend_B': marginal_utility_of_spend_B
    }


def update_axis5_text_display(chart, data):
    """Update axis 5 with text display of marginal utility values."""
    a = chart.axis5
    a.axis('off')  # Hide axes for text display
    a.text(0.5, 0.8, f'Marginal Effectiveness of Spend A: {data["marginal_utility_of_spend_A"]:.3f}', 
           transform=a.transAxes, fontsize=14, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightblue', alpha=0.8))
    a.text(0.5, 0.6, f'Marginal Effectiveness of Spend B: {data["marginal_utility_of_spend_B"]:.3f}', 
           transform=a.transAxes, fontsize=14, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightgreen', alpha=0.8))
    a.text(0.5, 0.3, f'Optimal CPM A: {data["min_cost_cpm_A"]:.2f}  |  Optimal CPM B: {data["min_cost_cpm_B"]:.2f}', 
           transform=a.transAxes, fontsize=12, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightyellow', alpha=0.8))


def update_axis1_win_probability(chart, data):
    """Update axis 1 with win probability curves."""
    a = chart.axis1
    a.set_xlim(right = MAX_CPM)
    line1 = a.plot(data['l_prob_range'], data['l_prob_A'], 'C0-', label='Win probability A')
    line2 = a.plot(data['l_prob_range'], data['l_prob_B'], 'C1-', label='Win probability B')
    
    # Add both vertical lines and points using same colors
    a.vlines(data['min_cost_cpm_A'], 0, data['s_A'].get_probability(data['min_cost_cpm_A']), 'C0', linestyles='--', alpha=0.5)
    a.vlines(data['min_cost_cpm_B'], 0, data['s_B'].get_probability(data['min_cost_cpm_B']), 'C1', linestyles='--', alpha=0.5)
    a.plot(data['min_cost_cpm_A'], data['s_A'].get_probability(data['min_cost_cpm_A']), 'C0o', label='Optimal bid A')
    a.plot(data['min_cost_cpm_B'], data['s_B'].get_probability(data['min_cost_cpm_B']), 'C1o', label='Optimal bid B')
    
    a.legend(loc='lower right')


def update_axis3_cpm_and_value(chart, data):
    """Update axis 3 with CPM B vs CPM A and total value."""
    a = chart.axis3
    
    a.set_ylim(top = max(data['l_cpm_B']))
    a.set_xlim(right = MAX_CPM)
    a.set_xlabel('Cpm A', color='C0')
    a.tick_params(axis='x', labelcolor='C0')
    
    # Plot Cpm B on left y-axis
    a.set_ylabel('Cpm B', color='orange')
    a.tick_params(axis='y', labelcolor='orange')
    line1 = a.plot(data['l_cpm_A'], data['l_cpm_B'], label='Cpm B', color='C1')
    
    # Create right y-axis for Total value
    ax3_right = a.twinx()
    ax3_right.set_ylim(bottom = 0.0)
    ax3_right.set_ylim(top = data['max_value'])
    line2 = ax3_right.plot(data['l_cpm_A'], data['l_total_value'], 'g-', label='Total value')
    ax3_right.set_ylabel('Total value', color='g')
    ax3_right.tick_params(axis='y', labelcolor='g')
    
    # Add green dot at intersection of green curve with vertical line
    line5 = ax3_right.plot(data['min_cost_cpm_A'], data['max_value'], 'go', label='Maximum value')
    
    a.vlines(data['min_cost_cpm_A'], 0, data['min_cost_cpm_B'], 'C0', linestyles='--', alpha=0.5)
    line3 = a.plot(data['min_cost_cpm_A'], 0, 'C0o', label='Optimal bid A')
    
    # Add horizontal orange line where vertical intersects orange curve, with dot
    a.hlines(data['min_cost_cpm_B'], 0, data['min_cost_cpm_A'], 'C1', linestyles='--', alpha=0.5)
    line4 = a.plot(0, data['min_cost_cpm_B'], 'C1o', label='Optimal bid B')
    
    # Combine legends from all plot elements
    lines = line1 + line2 + line3 + line4 + line5
    labels = [l.get_label() for l in lines]
    a.legend(lines, labels, loc='upper right')
    
    # Adjust layout for axis3 to ensure x-axis label is visible
    chart.axis3.figure.subplots_adjust(bottom=0.15)


def update_axis2_marginal_utility(chart, p: Parameters, data):
    """Update axis 2 with marginal utility plots."""
    a = chart.axis2
    s_A = data['s_A']
    s_B = data['s_B']
    
    # Set up axes for marginal effectiveness of spend vs budget
    marginal_utility_start = s_A.marginal_utility_of_spend(4)
    marginal_utility_end = s_A.marginal_utility_of_spend(20)
    a.set_xlim(left=marginal_utility_start, right=marginal_utility_end)  # Decreasing toward the right (20 to 0.01)
    a.set_xlabel('Marginal Effectivness of Spend')
    a.grid(True, alpha=0.3)  # Add grid for better readability
    
    # Create right y-axis for the inverse functions
    ax2_right = a.twinx()
    
    # Generate marginal utility values for x-axis
    marginal_utility_range = []
    inverse_values_A = []
    inverse_values_B = []
    budget_used_values = []
    
    # Create range from marginal_utility_end to marginal_utility_start (decreasing)
    num_points = 50
    for i in range(num_points):
        # Linear interpolation from end to start
        mu_value = marginal_utility_end + (marginal_utility_start - marginal_utility_end) * i / (num_points - 1)
        marginal_utility_range.append(mu_value)
        
        # Call inverse functions for both s_A and s_B
        try:
            cpm_A = s_A.marginal_utility_of_spend_inverse(mu_value)
            cpm_B = s_B.marginal_utility_of_spend_inverse(mu_value)
            # Calculate budget used for both curves
            cpm_A = cpm_A if cpm_A is not None else 0
            cpm_B = cpm_B if cpm_B is not None else 0
            inverse_values_A.append(cpm_A)
            inverse_values_B.append(cpm_B)
            
            
            # Budget = volume * probability * CPM
            budget_A = p.total_volume_A() * s_A.get_probability(cpm_A) * cpm_A
            budget_B = p.total_volume_B() * s_B.get_probability(cpm_B) * cpm_B
            total_budget = budget_A + budget_B
            
            budget_used_values.append(total_budget)
        except:
            inverse_values_A.append(0)
            inverse_values_B.append(0)
            budget_used_values.append(0)
    
    # Plot budget used curve on left y-axis
    line_budget = a.plot(marginal_utility_range, budget_used_values, 'g-', label='Budget', linewidth=2)
    a.set_ylabel('Budget', color='g')
    a.tick_params(axis='y', labelcolor='g')
    
    # Plot inverse functions on right y-axis
    line1 = ax2_right.plot(marginal_utility_range, inverse_values_A, 'C0-', label='Bid A', linewidth=2)
    line2 = ax2_right.plot(marginal_utility_range, inverse_values_B, 'C1-', label='Bid B', linewidth=2)
    
    # Add vertical line at marginal_utility_of_spend_A
    a.axvline(x=data['marginal_utility_of_spend_A'], color='g', alpha=0.7, label='Optimal Marginal effectiveness of spend')

    # Find intersection point for budget used curve
    # Budget = volume * probability * CPM
    budget_A = p.total_volume_A() * s_A.get_probability(data['min_cost_cpm_A']) * data['min_cost_cpm_A']
    budget_B = p.total_volume_B() * s_B.get_probability(data['min_cost_cpm_B']) * data['min_cost_cpm_B']
    total_budget = budget_A + budget_B

    # Add horizontal line to the left for budget used curve
    a.hlines(total_budget, marginal_utility_start, data['marginal_utility_of_spend_A'], 'g', linestyles='--', alpha=0.5)
    
    # Add intersection dot for budget used curve
    a.plot(data['marginal_utility_of_spend_A'], total_budget, 'go', markersize=8, label='')

    ax2_right.hlines(data['min_cost_cpm_B'], 0, data['marginal_utility_of_spend_A'], 'C1', linestyles='--', alpha=0.5)
    ax2_right.hlines(data['min_cost_cpm_A'], 0, data['marginal_utility_of_spend_A'], 'C0', linestyles='--', alpha=0.5)

    # Mark intersection points with dots
    ax2_right.plot(data['marginal_utility_of_spend_A'], data['min_cost_cpm_A'], 'C0o', markersize=8, label='Intersection A dot')
    ax2_right.plot(data['marginal_utility_of_spend_A'], data['min_cost_cpm_B'], 'C1o', markersize=8, label='Intersection B dot')
    
    # Set right y-axis properties
    ax2_right.set_ylabel('CPM', color='purple')
    ax2_right.tick_params(axis='y', labelcolor='purple')
    
    # Combine legends from both axes
    lines = line_budget + line1 + line2  # Include budget curve from left axis
    labels = [l.get_label() for l in lines]
    
    # Add the vertical line to the legend
    lines_main = a.get_lines()  # Get lines from main axis
    if len(lines_main) > 1:  # Check if we have more than just the budget line
        lines.extend(lines_main[-1:])  # Add the last line (vertical line)
        labels.extend([lines_main[-1].get_label()])
    
    ax2_right.legend(lines, labels, loc='upper right')


def update_axis4_bottom_right_chart(chart, p: Parameters, data, show_value_vs_budget, show_marginal_utility_heatmap):
    """Update axis 4 with bottom-right chart (heatmap, value vs budget, or default)."""
    a = chart.axis4
    s_A = data['s_A']
    s_B = data['s_B']
    
    # Handle the bottom-right chart based on toggle states
    if show_marginal_utility_heatmap:
        # Create heatmap of y=marginal_utility_of_spend_inverse()
        a.clear()
        
        # Define ranges for the heatmap
        value_range = np.linspace(0.4, 5.0, 50)  # s_A.value from 0.5 to 5
        y_target_range = np.linspace(0.02, 0.6, 50)  # y_target from 0.02 to 0.6 (reversed for display)
        
        # Create meshgrid
        Value, Y_target = np.meshgrid(value_range, y_target_range)
        
        # Initialize result matrix
        Z = np.zeros_like(Value)
        
        # Calculate marginal_utility_of_spend_inverse for each combination
        for i in range(len(value_range)):
            for j in range(len(y_target_range)):
                # Create temporary sigmoid with varied value
                temp_sigmoid = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, value_range[i])
                try:
                    result = temp_sigmoid.marginal_utility_of_spend_inverse(y_target_range[j])
                    Z[j, i] = result if result is not None else 0
                except:
                    Z[j, i] = 0
        
        # Create 3D contour lines along z-axis
        contour_lines = a.contour(Value, Y_target, Z, levels=15, cmap='viridis', alpha=0.8)
        
        # Set 3D viewing angle for better visualization
        a.view_init(elev=30, azim=45)
        
        # Set labels and title
        a.set_xlabel('v')
        a.set_ylabel('u')
        a.set_zlabel('x = M^-1(u, W, v)')
        #a.set_xlabel('s_A Value')
        #a.set_ylabel('Marginal Effectiveness of Spend')
        #a.set_zlabel('Bid A = M^-1(x, v, W)')
        a.set_title('3D Wireframe: y=M^-1(x, v, W)')
        
        # Add current parameter point in 3D
#        current_z = data['s_A'].marginal_utility_of_spend_inverse(data['marginal_utility_of_spend_A'])
#        if current_z is not None:
#            a.scatter([p.sigmoid_A_value], [data['marginal_utility_of_spend_A']], [current_z], 
#                     c='red', s=100, label=f'Current (value={p.sigmoid_A_value:.2f}, MU={data["marginal_utility_of_spend_A"]:.3f})')
#            a.legend()
        
    elif show_value_vs_budget: 
        # Calculate value vs budget curve
        l_budget_range = data['l_budget_range']
        l_max_value = data['l_max_value']
        l_value_per_cost = data['l_value_per_cost']
        l_marginal_utility_A_budget = data['l_marginal_utility_A_budget']
        l_marginal_utility_B_budget = data['l_marginal_utility_B_budget']
        
        for budget_x in range(0, int(20.0/BUDGET_STEP)):
            budget = budget_x * BUDGET_STEP
            l_budget_range.append(budget)
            
            # Find maximum value achievable with this budget
            max_value_for_budget = 0.0
            optimal_cpm_A_for_budget = 0.0
            optimal_cpm_B_for_budget = 0.0
            
            for cpm_input in range(0, int(MAX_CPM / STEP)):
                cmp_A = 1.0 * cpm_input * STEP
                imp_bought_A = s_A.get_probability(cmp_A) * p.total_volume_A() 
                imp_value_A = imp_bought_A * s_A.value
                imp_spend_A = imp_bought_A * cmp_A
                
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
                if total_value > max_value_for_budget:
                    max_value_for_budget = total_value
                    optimal_cpm_A_for_budget = cmp_A
                    optimal_cpm_B_for_budget = cpm_B
            
            l_max_value.append(max_value_for_budget)            
                
            l_marginal_utility_A_budget.append(s_A.marginal_utility_of_spend(optimal_cpm_A_for_budget))
            l_marginal_utility_B_budget.append(s_B.marginal_utility_of_spend(optimal_cpm_B_for_budget))
            
            # Calculate value per cost (efficiency), avoiding division by zero
            if budget > 0:
                l_value_per_cost.append(max_value_for_budget/budget)
            else:
                l_value_per_cost.append(0)
        
        # Plot value vs budget curve in bottom right with two y-axes
        a.clear()
        # Remove any existing second y-axis before creating a new one
        for ax in a.figure.axes:
            if ax != a and ax.bbox.bounds == a.bbox.bounds:
                a.figure.delaxes(ax)
                    
        a.set_xlim(right=20.0)
        
        # Plot total value on left y-axis
        line1 = a.plot(l_budget_range, l_max_value, 'b-', label='Max value')
        a.set_xlabel('Budget')
        a.set_ylabel('Maximum achievable value', color='b')
        a.tick_params(axis='y', labelcolor='b')
        
        # Create second y-axis for value per cost
        ax2 = a.twinx()
        line2 = ax2.plot(l_budget_range, l_value_per_cost, 'r-', label='Value / budget')
        ax2.set_ylabel('Value / budget', color='r')
        ax2.tick_params(axis='y', labelcolor='r')
        
        # Create third y-axis for marginal utility
        line3 = ax2.plot(l_budget_range, l_marginal_utility_A_budget, 'g-', label='Marginal Utility A')
        line4 = ax2.plot(l_budget_range, l_marginal_utility_B_budget, 'orange', linestyle='-', label='Marginal Utility B')
        
        # Mark current budget point on all curves
        #current_value =l data['max_value'] if data['max_value'] is not None else 0
        #current_efficiency = current_value/p.total_budget if p.total_budget > 0 else 0
        #a.plot(p.total_budget, current_value, 'bo', label='Current value')
        #ax2.plot(p.total_budget, current_efficiency, 'ro', label='Current efficiency')
        
        # Mark current marginal utility points
        current_mu_A = data['marginal_utility_of_spend_A'] if data['marginal_utility_of_spend_A'] is not None else 0
        current_mu_B = data['marginal_utility_of_spend_B'] if data['marginal_utility_of_spend_B'] is not None else 0
        ax2.plot(p.total_budget, current_mu_A, 'go', label='Current MU A')
        ax2.plot(p.total_budget, current_mu_B, 'o', color='orange', label='Current MU B')
        
        # Add combined legend
        lines = line1 + line2 + line3 + line4
        labels = [l.get_label() for l in lines]
        a.legend(lines, labels, loc='upper right')
    else:
        # Clear the bottom-right pane when both toggles are disabled
        a.set_xlim(right = 1.0)
        a.plot(data['l2'], data['l22'], label='Value for budget')
        a.legend()


def update_axis(chart, p: Parameters, save_to_png=False, show_value_vs_budget=False, show_marginal_utility_heatmap=False):
    # Setup optimization data
    data = setup_optimization_data(p)
    
    # Update axis 5 (text display)
    update_axis5_text_display(chart, data)
    
    # Update axis 1 (win probability curves)
    update_axis1_win_probability(chart, data)
    
    # Update axis 3 (CPM B vs CPM A with total value)
    update_axis3_cpm_and_value(chart, data)
    
    # Update axis 2 (marginal utility plots)
    update_axis2_marginal_utility(chart, p, data)
    
    # Update axis 4 (bottom-right chart)
    update_axis4_bottom_right_chart(chart, p, data, show_value_vs_budget, show_marginal_utility_heatmap)

    # Save plots to PNG if requested
    if save_to_png:
        # Get the figures from the axes
        fig1 = chart.axis1.figure
        fig2 = chart.axis2.figure
        fig3 = chart.axis3.figure
        fig4 = chart.axis4.figure
        fig5 = chart.axis5.figure
        
        # Create parameter string for filename
        param_str = f"tb{p.total_budget}_tv{p.total_volume}_pa{p.percent_A}_sav{p.sigmoid_A_value}_sas{p.sigmoid_A_scale}_sao{p.sigmoid_A_offset}_sbv{p.sigmoid_B_value}_sbs{p.sigmoid_B_scale}_sbo{p.sigmoid_B_offset}"
        
        # Save each figure with parameter string in filename
        fig1.savefig(f'plot1_win_probability_{param_str}.png', dpi=300, bbox_inches='tight')
        fig2.savefig(f'plot2_total_value_{param_str}.png', dpi=300, bbox_inches='tight')
        fig3.savefig(f'plot3_total_cost_{param_str}.png', dpi=300, bbox_inches='tight')
        fig4.savefig(f'plot4_min_price_{param_str}.png', dpi=300, bbox_inches='tight')
        fig5.savefig(f'plot5_marginal_utility_{param_str}.png', dpi=300, bbox_inches='tight')
        
        print("Plots saved as PNG files")
    

class Chart(FigureCanvas):
    def __init__(self):
        # Create five separate figures instead of one with subplots
        self.fig1 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig2 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig3 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig4 = Figure(figsize=(6, 4.2), dpi=100)
        self.fig5 = Figure(figsize=(12, 2), dpi=100)  # Horizontal figure for text
        
        # Create individual axes for each chart
        self.axis1 = self.fig1.add_subplot(111)
        self.axis2 = self.fig2.add_subplot(111)
        self.axis3 = self.fig3.add_subplot(111)
        self.axis4 = self.fig4.add_subplot(111)
        self.axis5 = self.fig5.add_subplot(111)
        
        # Adjust layout to ensure labels are visible
        self.fig1.tight_layout()
        self.fig2.tight_layout()
        # Set specific bottom margin for fig3 to show x-axis label
        self.fig3.subplots_adjust(bottom=0.15)
        self.fig4.tight_layout()
        self.fig5.tight_layout()
        

        
        self.p = Parameters()
        self.show_value_vs_budget = False  # Default to off
        self.show_marginal_utility_heatmap = False  # New flag for heatmap
        self.axis4_colorbar = None  # Store colorbar reference for proper removal
        update_axis(self, self.p, show_value_vs_budget=self.show_value_vs_budget, show_marginal_utility_heatmap=self.show_marginal_utility_heatmap)
        
        # Use the first figure as the main canvas
        super().__init__(self.fig1)
        self.set_size_request(600, 400)
        plt.ion()

    def update(self, save_to_png=False):
        # Clear all axes
        self.axis1.clear()
        self.axis2.clear()
        # Remove any existing twin axes for axis2
        for ax in self.axis2.figure.axes:
            if ax != self.axis2 and ax.bbox.bounds == self.axis2.bbox.bounds:
                self.axis2.figure.delaxes(ax)
        self.axis3.clear()
        # Remove any existing twin axes for axis3
        for ax in self.axis3.figure.axes:
            if ax != self.axis3 and ax.bbox.bounds == self.axis3.bbox.bounds:
                self.axis3.figure.delaxes(ax)
        
        # Enhanced clearing for axis4 to handle heatmap colorbars properly
        # Remove existing colorbar first
        if self.axis4_colorbar is not None:
            self.axis4_colorbar.remove()
            self.axis4_colorbar = None
            
        # Clear the axis
        self.axis4.clear()
        
        # Remove any remaining twin axes for axis4
        axes_to_remove = []
        for ax in self.axis4.figure.axes:
            if ax != self.axis4:
                axes_to_remove.append(ax)
        for ax in axes_to_remove:
            self.axis4.figure.delaxes(ax)
        
        # Recreate axis4 as 3D if showing marginal utility heatmap, otherwise 2D
        if self.show_marginal_utility_heatmap:
            self.fig4.clear()
            self.axis4 = self.fig4.add_subplot(111, projection='3d')
        else:
            # Ensure it's 2D for other modes
            if hasattr(self.axis4, 'zaxis'):  # Check if it's currently 3D
                self.fig4.clear()
                self.axis4 = self.fig4.add_subplot(111)
        
        # Reset the axis position to ensure proper layout
        self.axis4.set_position(self.axis4.get_position())
            
        self.axis5.clear()
        update_axis(self, self.p, save_to_png, show_value_vs_budget=self.show_value_vs_budget, show_marginal_utility_heatmap=self.show_marginal_utility_heatmap)

        # Adjust layout to ensure labels are visible
        self.fig1.tight_layout()
        self.fig2.tight_layout()
        # Don't use tight_layout for fig3 as we use subplots_adjust instead
        self.fig4.tight_layout()
        self.fig5.tight_layout()

        # Redraw all figures
        self.fig1.canvas.draw()
        self.fig2.canvas.draw()
        self.fig3.canvas.draw()
        self.fig4.canvas.draw()
        self.fig5.canvas.draw()
        
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
        self.canvas5 = FigureCanvas(self.chart.fig5)
        
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
        self.charts_grid.attach(self.canvas5, 0, 2, 2, 1)  # Horizontal figure spanning both columns
        
        self.box3.append(self.charts_grid)
        self.check = Gtk.Label(label="Parameters")
        self.box2.append(self.check)
        
        # Store references to controls for updating later
        self.controls = {}
        self.box2.append(self.numeric_entry_box("Auction volume", 1, 20, 'total_volume'))
        self.box2.append(self.slider_box("Percent of auction A", 0.0, 1.0, 'percent_A'))
        self.box2.append(self.slider_box("Total budget", 0.0, 20.0, 'total_budget'))
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
        
        # Add Marginal Utility Heatmap toggle
        self.marginal_utility_heatmap_toggle = Gtk.CheckButton(label="y=M^-1(x, v, W)")
        self.marginal_utility_heatmap_toggle.set_active(False)  # Default to off
        self.marginal_utility_heatmap_toggle.connect('toggled', self.on_marginal_utility_heatmap_toggle)
        self.box2.append(self.marginal_utility_heatmap_toggle)
        
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
        
        # Store reference to control and signal ID for updating later
        self.controls[parameter_name] = {'widget': slider, 'signal_id': signal_id}
        
        return b

    def numeric_entry_box(self, label, min_value, max_value, parameter_name):
        assert parameter_name in self.chart.p.__dict__.keys()
        start_value = self.chart.p.__dict__[parameter_name]
        
        # Create a SpinButton for numeric input
        spin_button = Gtk.SpinButton()
        spin_button.set_range(min_value, max_value)
        
        # Set integer-only for auction volume, decimals for others
        if parameter_name == 'total_volume':
            spin_button.set_increments(1.0, 1.0)  # step, page increment
            spin_button.set_digits(0)  # no decimal places
        else:
            spin_button.set_increments(0.1, 1.0)  # step, page increment
            spin_button.set_digits(2)  # 2 decimal places
            
        spin_button.set_value(start_value)
        
        def change_function(self):
            self.chart.p.__dict__[parameter_name] = float(spin_button.get_value())
            self.chart.update()
        
        spin_button.chart = self.chart    
        signal_id = spin_button.connect('value-changed', change_function)
        
        b = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        b.append(Gtk.Label(label=label))
        b.append(spin_button)
        
        # Store reference to control and signal ID for updating later
        self.controls[parameter_name] = {'widget': spin_button, 'signal_id': signal_id}
        
        return b

    def save_plots(self, button):
        """Save all four plots as PNG files"""
        self.chart.update(save_to_png=True)

    def load_interesting_curves(self, button):
        """Load interesting curves and update charts"""
        self.chart.p.interesting_curves()
        
        # Update all controls to reflect the new parameter values
        for parameter_name, control_info in self.controls.items():
            if parameter_name in self.chart.p.__dict__:
                widget = control_info['widget']
                signal_id = control_info['signal_id']
                # Temporarily block the signal to avoid triggering updates
                widget.handler_block(signal_id)
                widget.set_value(self.chart.p.__dict__[parameter_name])
                widget.handler_unblock(signal_id)
        
        self.chart.update()

    def on_value_budget_toggle(self, toggle_button):
        """Handle toggle for Value vs. budget curve"""
        self.chart.show_value_vs_budget = toggle_button.get_active()
        self.chart.update()

    def on_marginal_utility_heatmap_toggle(self, toggle_button):
        """Handle toggle for Marginal Utility Heatmap"""
        self.chart.show_marginal_utility_heatmap = toggle_button.get_active()
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

