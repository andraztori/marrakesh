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
        


def update_axis(chart, p: Parameters, save_to_png=False, show_value_vs_budget=False):
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset, p.sigmoid_A_value)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset, p.sigmoid_B_value) 

    l1 = []
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

    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        l_prob_range.append(cpm)
        l_prob_A.append(s_A.get_probability(cpm))
        l_prob_B.append(s_B.get_probability(cpm))

    l2 = []
    l22 = []

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
        l1.append(cpm_A) 
        l_total_value.append(total_value)
        l_cpm_B.append(cpm_B)
        l_imp_bought_A.append(imp_bought_A) 
        l_imp_bought_B.append(imp_bought_B)
    
        
    marginal_utility_of_spend_A = s_A.marginal_utility_of_spend(min_cost_cpm_A)
    marginal_utility_of_spend_B = s_B.marginal_utility_of_spend(min_cost_cpm_B)


    # Display marginal utility values as text on axis5
    a = chart.axis5
    a.axis('off')  # Hide axes for text display
    a.text(0.5, 0.8, f'Marginal Utility of Spend A: {marginal_utility_of_spend_A:.3f}', 
           transform=a.transAxes, fontsize=14, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightblue', alpha=0.8))
    a.text(0.5, 0.6, f'Marginal Utility of Spend B: {marginal_utility_of_spend_B:.3f}', 
           transform=a.transAxes, fontsize=14, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightgreen', alpha=0.8))
    a.text(0.5, 0.3, f'Optimal CPM A: {min_cost_cpm_A:.2f}  |  Optimal CPM B: {min_cost_cpm_B:.2f}', 
           transform=a.transAxes, fontsize=12, ha='center', va='center',
           bbox=dict(boxstyle='round', facecolor='lightyellow', alpha=0.8))

    a = chart.axis1
    a.set_xlim(right = MAX_CPM)
    line1 = a.plot(l_prob_range, l_prob_A, 'C0-', label='Win probability A')
    line2 = a.plot(l_prob_range, l_prob_B, 'C1-', label='Win probability B')
    # Add both vertical lines and points using same colors
    a.vlines(min_cost_cpm_A, 0, s_A.get_probability(min_cost_cpm_A), 'C0', linestyles='--', alpha=0.5)
    a.vlines(min_cost_cpm_B, 0, s_B.get_probability(min_cost_cpm_B), 'C1', linestyles='--', alpha=0.5)
    a.plot(min_cost_cpm_A, s_A.get_probability(min_cost_cpm_A), 'C0o', label='Optimal bid A')
    a.plot(min_cost_cpm_B, s_B.get_probability(min_cost_cpm_B), 'C1o', label='Optimal bid B')
    
    a.legend(loc='lower right') 

    a = chart.axis3
    
    a.set_ylim(top = max(l_cpm_B))
    a.set_xlim(right = MAX_CPM)
    a.set_xlabel('Cpm A', color='C0')
    a.tick_params(axis='x', labelcolor='C0')
    
    # Plot Cpm B on left y-axis
    a.set_ylabel('Cpm B', color='orange')
    a.tick_params(axis='y', labelcolor='orange')
    line1 = a.plot(l1, l_cpm_B, label='Cpm B', color='C1')
    
    # Create right y-axis for Total value
    ax3_right = a.twinx()
    ax3_right.set_ylim(bottom = 0.0)
    ax3_right.set_ylim(top = max_value)
    line2 = ax3_right.plot(l1, l_total_value, 'g-', label='Total value')
    ax3_right.set_ylabel('Total value', color='g')
    ax3_right.tick_params(axis='y', labelcolor='g')
    
    # Add green dot at intersection of green curve with vertical line
    line5 = ax3_right.plot(min_cost_cpm_A, max_value, 'go', label='Maximum value')
    
    a.vlines(min_cost_cpm_A, 0, min_cost_cpm_B, 'C0', linestyles='--', alpha=0.5)
    line3 = a.plot(min_cost_cpm_A, 0, 'C0o', label='Optimal bid A')
#    a.hlines(max_value, 0, MAX_CPM, colors='C0')
    # Add horizontal orange line where vertical intersects orange curve, with dot
    a.hlines(min_cost_cpm_B, 0, min_cost_cpm_A, 'C1', linestyles='--', alpha=0.5)
    line4 = a.plot(0, min_cost_cpm_B, 'C1o', label='Optimal bid B')
    
    # Combine legends from all plot elements
    lines = line1 + line2 + line3 + line4 + line5
    labels = [l.get_label() for l in lines]
    a.legend(lines, labels, loc='upper right')
    
    # Adjust layout for axis3 to ensure x-axis label is visible
    chart.axis3.figure.subplots_adjust(bottom=0.15)

    a = chart.axis2
    
    # Set up axes for marginal utility of spend vs budget
    marginal_utility_start = s_A.marginal_utility_of_spend(0.01)
    marginal_utility_end = s_A.marginal_utility_of_spend(20)
    a.set_xlim(left=marginal_utility_start, right=marginal_utility_end)  # Decreasing toward the right (20 to 0.01)
    a.set_xlabel('Marginal Utility of Spend')
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
            inverse_A = s_A.marginal_utility_of_spend_inverse(mu_value)
            inverse_B = s_B.marginal_utility_of_spend_inverse(mu_value)
            inverse_values_A.append(inverse_A if inverse_A is not None else 0)
            inverse_values_B.append(inverse_B if inverse_B is not None else 0)
            
            # Calculate budget used for both curves
            cpm_A = inverse_A if inverse_A is not None else 0
            cpm_B = inverse_B if inverse_B is not None else 0
            
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
    line_budget = a.plot(marginal_utility_range, budget_used_values, 'g-', label='Budget Used', linewidth=2)
    a.set_ylabel('Budget Used', color='g')
    a.tick_params(axis='y', labelcolor='g')
    
    # Plot inverse functions on right y-axis
    line1 = ax2_right.plot(marginal_utility_range, inverse_values_A, 'C0-', label='bid A', linewidth=2)
    line2 = ax2_right.plot(marginal_utility_range, inverse_values_B, 'C1-', label='bid B', linewidth=2)
    
    # Add vertical line at marginal_utility_of_spend_A
    a.axvline(x=marginal_utility_of_spend_A, color='g', alpha=0.7, label='Optimal Marginal utility of spend')

    # Find intersection point for budget used curve
    # Find the closest point in marginal_utility_range to marginal_utility_of_spend_A
    closest_index = 0
    min_diff = abs(marginal_utility_range[0] - marginal_utility_of_spend_A)
    for i, mu_val in enumerate(marginal_utility_range):
        diff = abs(mu_val - marginal_utility_of_spend_A)
        if diff < min_diff:
            min_diff = diff
            closest_index = i
    
    budget_intersection = budget_used_values[closest_index]
    
    # Add horizontal line to the left for budget used curve
    a.hlines(budget_intersection, marginal_utility_start, marginal_utility_of_spend_A, 'g', linestyles='--', alpha=0.5)
    
    # Add intersection dot for budget used curve
    a.plot(marginal_utility_of_spend_A, budget_intersection, 'go', markersize=8, label='Budget intersection')

    ax2_right.hlines(min_cost_cpm_B, 0, marginal_utility_of_spend_A, 'C1', linestyles='--', alpha=0.5)
    ax2_right.hlines(min_cost_cpm_A, 0, marginal_utility_of_spend_A, 'C0', linestyles='--', alpha=0.5)


    # Mark intersection points with dots
    ax2_right.plot(marginal_utility_of_spend_A, min_cost_cpm_A, 'C0o', markersize=8, label='Intersection A dot')
    ax2_right.plot(marginal_utility_of_spend_A, min_cost_cpm_B, 'C1o', markersize=8, label='Intersection B dot')
    
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
   
   
    a = chart.axis4
    a.set_xlim(right = 1.0)
    a.plot(l2, l22, label='Value for budget')
#    a.vlines(min_cost_cpm_A, 0, p.percent_to_buy, colors='C0')
    a.legend()

    # Calculate value vs budget curve
    if show_value_vs_budget: 
        for budget_x in range(0, int(20.0/BUDGET_STEP)):
            budget = budget_x * BUDGET_STEP
            l_budget_range.append(budget)
            
            # Find maximum value achievable with this budget
            max_value_for_budget = 0.0
            optimal_cpm_A_for_budget = 0.0
            optimal_cpm_B_for_budget = 0.0
            
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
                if total_value > max_value_for_budget:
                    max_value_for_budget = total_value
                    optimal_cpm_A_for_budget = cpm_A
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
        a = chart.axis4
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
        line2 = ax2.plot(l_budget_range, l_value_per_cost, 'r-', label='Value / cost')
        ax2.set_ylabel('Value / budget', color='r')
        ax2.tick_params(axis='y', labelcolor='r')
        
        # Create third y-axis for marginal utility
        # Offset the third y-axis to the right
        #ax2.spines['right'].set_position(('outward', 60))
        line3 = ax2.plot(l_budget_range, l_marginal_utility_A_budget, 'g-', label='Marginal Utility A')
        line4 = ax2.plot(l_budget_range, l_marginal_utility_B_budget, 'orange', linestyle='-', label='Marginal Utility B')
        #ax2.set_ylabel('Marginal Utility', color='g')
        #ax2.tick_params(axis='y', labelcolor='g')
        
        # Mark current budget point on all curves
        current_value = max_value if max_value is not None else 0
        current_efficiency = current_value/p.total_budget if p.total_budget > 0 else 0
        a.plot(p.total_budget, current_value, 'bo', label='Current value')
        ax2.plot(p.total_budget, current_efficiency, 'ro', label='Current efficiency')
        
        # Mark current marginal utility points
        current_mu_A = marginal_utility_of_spend_A if marginal_utility_of_spend_A is not None else 0
        current_mu_B = marginal_utility_of_spend_B if marginal_utility_of_spend_B is not None else 0
        ax2.plot(p.total_budget, current_mu_A, 'go', label='Current MU A')
        ax2.plot(p.total_budget, current_mu_B, 'o', color='orange', label='Current MU B')
        
        # Add combined legend
        lines = line1 + line2 + line3 + line4
        labels = [l.get_label() for l in lines]
        a.legend(lines, labels, loc='upper right')
    else:
        # Clear the bottom-right pane when toggle is disabled
        a = chart.axis4
        a.clear()
        # Remove any existing second and third y-axes
        for ax in a.figure.axes:
            if ax != a and ax.bbox.bounds == a.bbox.bounds:
                a.figure.delaxes(ax)
        
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
        update_axis(self, self.p, show_value_vs_budget=self.show_value_vs_budget)
        
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
        self.axis4.clear()
        self.axis5.clear()
        update_axis(self, self.p, save_to_png, show_value_vs_budget=self.show_value_vs_budget)

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
        
        # Store references to sliders and numeric entries for updating later
        self.sliders = {}
        self.numeric_entries = {}
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
        
        # Store reference to numeric entry and signal ID for updating later
        self.numeric_entries[parameter_name] = {'entry': spin_button, 'signal_id': signal_id}
        
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
        
        # Update all numeric entries to reflect the new parameter values
        for parameter_name, entry_info in self.numeric_entries.items():
            if parameter_name in self.chart.p.__dict__:
                entry = entry_info['entry']
                signal_id = entry_info['signal_id']
                # Temporarily block the signal to avoid triggering updates
                entry.handler_block(signal_id)
                entry.set_value(self.chart.p.__dict__[parameter_name])
                entry.handler_unblock(signal_id)
        
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

