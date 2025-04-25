
import math
import matplotlib.pyplot as plt
EPSILON = 0.00001


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

    
def main():
    MAX_CPM = 20
    STEP = 0.1
    s_A = Sigmoid(0.4, 8)
    s_B = Sigmoid(0.9, 9) 

    imp_to_buy = 0.5  
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
        if imp_to_buy - imp_bought_A > 1.0: # impossible 
            continue
        cpm_B = s_B.inverse(imp_to_buy - imp_bought_A)
        if cpm_B <= 0.0:
            continue
        imp_bought_B = s_B.value(cpm_B)
        
#        assert math.fabs((imp_bought_A + imp_bought_B) - imp_to_buy) < EPSILON	# make sure we've bought enough impressions
        if math.fabs((imp_bought_A + imp_bought_B) - imp_to_buy) > EPSILON:	# make sure we've bought enough impressions
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
        
    

    figure, axis = plt.subplots(2, 2)
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
    a.plot(l1, l_cpm_B, label='Cpm 2')#, l3)#, l4, l5)
    a.vlines(min_cost_cpm_A, 0, MAX_CPM, colors='C0')
    a.hlines(min_cost, 0, MAX_CPM, colors='C0')
    a.legend()
    
    a = axis[0, 1]
    a.set_xlim(right = MAX_CPM)
    a.plot(l1, l_imp_bought_A, label='Imp A bought')#, l3)#, l4, l5)
    a.plot(l1, l_imp_bought_B, label='Imp B bought')#, l3)#, l4, l5)
    a.vlines(min_cost_cpm_A, 0, imp_to_buy, colors='C0')
    a.legend()

    plt.show()
        
        
    
    
    
if __name__ == '__main__':
    main()    

    