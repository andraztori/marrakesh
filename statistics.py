import asciichartpy as acp
import random
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
        
    def register_impression(self, ioo: ImpressionOnOffer, price: float): 
        hour = int(ioo.time_s / 60 / 60)
        self.single.register_impression(price)
        self.hours[hour].register_impression(price)
        if ioo._clicked: 
            self.single.register_click()
            self.hours[hour].register_click()

    def draw_hourly_spend(self):
        x =  [stat.spend for stat in self.hours]
        print(acp.plot(x, {'height': 10}))

    def draw_hourly_cpm(self):
        x =  [1000.0*stat.spend/ stat.impressions if stat.clicks else 0 for stat in self.hours]
#        print (x)
        print(acp.plot(x, {'height': 10}))

    def draw_hourly_cpc(self):
        
        x =  [stat.spend / stat.clicks if stat.clicks else 0 for stat in self.hours]
#        print (x)
        print(acp.plot(x, {'height': 10}))
