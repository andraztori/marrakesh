import asciichartpy as acp
import random
import math
from impression import ImpressionOnOffer


class IndividualStat:
    def __init__(self):
        self.impressions = 0
        self.spend = 0.0
        self.clicks = 0
        
    def register_impression(self, price):
        self.impressions += 1
        self.spend += price
        
    def register_click(self):
        self.clicks += 1


class FullStat:
    def __init__(self):
        self.single = IndividualStat()
        self.hours = [IndividualStat() for x in range(24)]
        self.cpm_stat = RunningStats()
        
    def register_impression(self, ioo: ImpressionOnOffer, price: float): 
        hour = int(ioo.time_s / 60 / 60)
        self.single.register_impression(price)
        self.hours[hour].register_impression(price)
        self.cpm_stat.push(price)
        if ioo._clicked: 
            self.single.register_click()
            self.hours[hour].register_click()

    def draw_hourly_spend(self):
        x =  [stat.spend for stat in self.hours]
        print(acp.plot(x, {'height': 10}))

    def draw_hourly_cpm(self):
        print("CPM Standard Deviation: %2.5f" % self.cpm_stat.standard_deviation())
        x =  [1000.0*stat.spend/ stat.impressions if stat.clicks else 0 for stat in self.hours]
#        print (x)
        print(acp.plot(x, {'height': 10}))

    def draw_hourly_cpc(self):
        
        x =  [stat.spend / stat.clicks if stat.clicks else 0 for stat in self.hours]
#        print (x)
        print(acp.plot(x, {'height': 10}))



# from https://stackoverflow.com/questions/1174984/how-to-efficiently-calculate-a-running-standard-deviation
class RunningStats:

    def __init__(self):
        self.n = 0
        self.old_m = 0
        self.new_m = 0
        self.old_s = 0
        self.new_s = 0

    def clear(self):
        self.n = 0

    def push(self, x):
        self.n += 1

        if self.n == 1:
            self.old_m = self.new_m = x
            self.old_s = 0
        else:
            self.new_m = self.old_m + (x - self.old_m) / self.n
            self.new_s = self.old_s + (x - self.old_m) * (x - self.new_m)

            self.old_m = self.new_m
            self.old_s = self.new_s

    def mean(self):
        return self.new_m if self.n else 0.0

    def variance(self):
        return self.new_s / (self.n - 1) if self.n > 1 else 0.0

    def standard_deviation(self):
        return math.sqrt(self.variance())







