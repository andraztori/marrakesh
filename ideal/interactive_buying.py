import gi,sys
gi.require_version('Adw', '1')
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, Adw

import math
from matplotlib.backends.backend_gtk4agg import \
    FigureCanvasGTK4Agg as FigureCanvas
from matplotlib.figure import Figure
import matplotlib.pyplot as plt

EPSILON = 0.00001
MAX_CPM = 20
STEP = 0.2


class Sigmoid:
    def __init__(self, scale, offset):
        self.scale = scale
        self.offset = offset
    
    def value(self, x):
        return 1.0/(1+math.exp(-(x-self.offset)*self.scale))

    def inverse(self, y):
#        return math.log(y / (1 - y)) / self.scale + self.offset
        if y<EPSILON/10:
            y = EPSILON/10
        if 1.0 - y <= EPSILON/10:
            y = 1.0 -EPSILON/10
        return (math.log(y) - math.log(1 - y)) / self.scale + self.offset

#    def derivative(self, x):
#        return self.value(x) * (1.0 - self.value(x))

class Parameters:
    def __init__(self):
        self.imp_to_buy = 1.0
        self.sigmoid_A_scale = 0.4
        self.sigmoid_A_offset = 8.0
        self.sigmoid_B_scale = 0.9
        self.sigmoid_B_offset = 9.0

    
def update_axis(axis, p: Parameters):
    s_A = Sigmoid(p.sigmoid_A_scale, p.sigmoid_A_offset)
    s_B = Sigmoid(p.sigmoid_B_scale, p.sigmoid_B_offset) 

    l1 = []
    l_cpm_B = []
    l_imp_bought_A = []
    l_imp_bought_B = []
    l_total_cost = []
    l_prob_A = []
    l_prob_B = []
    l_prob_Binv = []
    l_prob_range = []
    l_inv_1 = []
    l_inv_2 = []
    l_inv_3 = []

    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm = 1.0 * cpm_input * STEP
        l_prob_range.append(cpm)
        l_prob_A.append(s_A.value(cpm))
        l_prob_B.append(s_B.value(cpm))

    min_cost = 1000.0
    min_cost_cpm_A = 0
    min_cost_cpm_B = 0
    for cpm_input in range(0, int(MAX_CPM / STEP)):
        cpm_A = 1.0 * cpm_input * STEP
        imp_bought_A = s_A.value(cpm_A)
        if p.imp_to_buy - imp_bought_A > 1.0: # impossible 
            continue
        cpm_B = s_B.inverse(p.imp_to_buy - imp_bought_A)
        if cpm_B <= 0.0:
            continue
        imp_bought_B = s_B.value(cpm_B)
        
#        assert math.fabs((imp_bought_A + imp_bought_B) - imp_to_buy) < EPSILON	# make sure we've bought enough impressions
        if math.fabs((imp_bought_A + imp_bought_B) - p.imp_to_buy) > EPSILON:	# make sure we've bought enough impressions
            # it was impossible to buy, just fill in with empty values
            continue
 
        total_cost = cpm_A * imp_bought_A + cpm_B * imp_bought_B # this is the function we are trying to find zero-derivate of
        if total_cost < min_cost:
            min_cost = total_cost
            min_cost_cpm_A = cpm_A
            min_cost_cpm_B = cpm_B
        l1.append(cpm_A)
        l_total_cost.append(total_cost)
        l_cpm_B.append(cpm_B)
        l_imp_bought_A.append(imp_bought_A)
        l_imp_bought_B.append(imp_bought_B)
        
    

    a = axis[0, 0]
    a.set_xlim(right = MAX_CPM)
    a.plot(l_prob_range, l_prob_A, label='Imp A probability')#, l3)#, l4, l5)
    a.plot(l_prob_range, l_prob_B, label='Imp B probability')#, l3)#, l4, l5)
    a.vlines(min_cost_cpm_A, 0, 1.0, colors='C0')
    a.vlines(min_cost_cpm_B, 0, 1.0, colors='C1')
    
    a.legend() 

    a = axis[1, 0]
    
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_total_cost, label='Total cost')#, l3)#, l4, l5)
    a.plot(l1, l_cpm_B, label='Cpm B')#, l3)#, l4, l5)
    a.vlines(min_cost_cpm_A, 0, MAX_CPM, colors='C0')
    a.hlines(min_cost, 0, MAX_CPM, colors='C0')
    a.legend()
   
    a = axis[0, 1]
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_imp_bought_A, label='Imp A bought')#, l3)#, l4, l5)
    a.plot(l1, l_imp_bought_B, label='Imp B bought')#, l3)#, l4, l5)
    a.vlines(min_cost_cpm_A, 0, p.imp_to_buy, colors='C0')
    a.legend()



class Chart(FigureCanvas):
    def __init__(self):
        self.fig = Figure(figsize=(5, 4), dpi=100)
        self.axis = self.fig.subplots(2, 2)
        self.p = Parameters()
        update_axis(self.axis, self.p)

#        self.ax = self.fig.add_subplot()
#        t = np.arange(0.0, 3.0, 0.01)
#        s = np.sin(2*np.pi*t)
        super().__init__(self.fig)
        
        #canvas = FigureCanvas(fig)  # a Gtk.DrawingArea
        self.set_size_request(600, 600)
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
        self.set_default_size(1000, 650)
        self.set_title("BestBuy")

        # A scrolled margin goes outside the scrollbars and viewport.
        #sw = Gtk.ScrolledWindow(margin_top=10, margin_bottom=10,
        #                        margin_start=10, margin_end=10)
        #self.set_child(sw)
        self.box1 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing = 10)
        self.box2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.box3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        
        
#        self.button = Gtk.Button(label="Hello")
#        self.button.connect('clicked', self.hello)

        self.set_child(self.box1)  # Horizontal box to window
        self.box1.append(self.box2)  # Put vert box in that box
        self.box1.append(self.box3)  # And another one, empty for now

 #       self.box2.append(self.button) # Put button in the first of the two vertical boxes
        
        
        self.chart = Chart()
        self.box3.append(self.chart)
        self.check = Gtk.Label(label="Parameters")
        self.box2.append(self.check)
        self.box2.append(self.slider_box("Imp to buy", 0.0, 2.0, 'imp_to_buy'))
        self.box2.append(self.slider_box("Sigma A scale", 0.0, 2.0, 'sigmoid_A_scale'))
        self.box2.append(self.slider_box("Sigma A offset", 0.0, 10.0, 'sigmoid_A_offset'))
        self.box2.append(self.slider_box("Sigma B scale", 0.0, 2.0, 'sigmoid_B_scale'))
        self.box2.append(self.slider_box("Sigma B offset", 0.0, 10.0, 'sigmoid_B_offset'))
        

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
        slider.connect('value-changed', change_function)
        slider.set_hexpand(True)
        b = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        b.append(Gtk.Label(label=label))
        b.append(slider)
        return b



    def slider_changed(self, slider):
        self.chart.p.imp_to_buy = float(slider.get_value())
        self.chart.update()

#    def hello(self, button):
#        print("Hello world")
#        if self.check.get_active():
#            print("Goodbye world!")
#            self.close()



class MyApp(Adw.Application):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.connect('activate', self.on_activate)

    def on_activate(self, app):
        self.win = MainWindow(application=app)
        self.win.present()





if __name__ == '__main__':
    app = MyApp(application_id="com.example.GtkApplication")
    app.run(sys.argv)

