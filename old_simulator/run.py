import random
import numpy as np
import matplotlib.pyplot as plt

import math

from campaigns import 	Campaign, \
                        CampaignStaticCPC, \
                        CampaignThrottledStaticCPC, \
                        CampaignPacedMinCPC
from impression import ImpressionOnOffer    
from statistics import FullStat
from helpers import sigmoid, inverse_sigmoid
        
class CONFIG:
    IMPRESSIONS = 100000
    BASE_CTR = 0.1
    BASE_CTR_JITTER_PERCENT = 0.01
    BASE_PCTR_CAMPAIGN_JITTER_PERCENT = 0.1
    BASE_PCTR_IMPRESSION_JITTER_PERCENT = 0.1
    SECOND_PRICE = False
    CAMPAIGN_EMBEDDING_SIZE = 4

class Simulation:
    def __init__(self, config):
        self.stat = FullStat()
        self.cs : list[Campaign] = []
        self.CONFIG = config
        self.CONFIG.rng = np.random.default_rng(seed = 11)
        # we want campaigns to have correlated embeddings, so they fight for the same impressions
        self.CONFIG.campaign_base_embedding = self.CONFIG.rng.normal(loc = 0.0, scale =  1, size = self.CONFIG.CAMPAIGN_EMBEDDING_SIZE)
        # we want to have base CTR set. 
        #This doesn't quite work, since randomness then moves the average toward 50%, but it is in the right direction    
        self.CONFIG.base_intercept = inverse_sigmoid(CONFIG.BASE_CTR)	
        self.max_fractional_clicks = 0.0
        self.got_fractional_clicks = 0.0
            
    def run_one_auction(self, iid, time_s):
        ioo = ImpressionOnOffer(iid, time_s, self.CONFIG)
        # get highest bid
        bids = []
        max_ctr = 0.0
        for c in self.cs:
            bid = c.get_bid(ioo)
            actual_ctr = sigmoid(ioo.embedding.dot(c.embedding) + self.CONFIG.base_intercept)
            max_ctr = max(actual_ctr, max_ctr)
            if bid:
                bids.append((c, bid, bid * c.hurdle, actual_ctr))
        self.max_fractional_clicks += max_ctr
        
        bids.sort(key=lambda t: t[2], reverse = True) 		# sort by hurdle-aware bids
        if len(bids) > 0:
            win_c = bids[0][0]
            if self.CONFIG.SECOND_PRICE:
                if len(bids) > 1:
                    win_bid = bids[1][2] / win_c.hurdle		# adjust for the hurdle  
                else: # a single bidder, what do we do?
                    win_bid = bids[0][1]	# let's do the first price
            else:
                win_bid = bids[0][1]	# simple first price
            self.got_fractional_clicks += actual_ctr
            win_c.register_impression(ioo, win_bid)
            self.stat.register_impression(ioo, win_bid)
            self.stat_per_ctype[win_c.type].register_impression(ioo, win_bid)
            return True
        else:
            return False
    
    
    def run(self):
        campaign_types = set(c.type for c in self.cs)
        # We want campaigns to be quite correlated, so we start with a base embedding and add local embedding to it
        for cs in self.cs:
            cs.embedding = self.CONFIG.campaign_base_embedding + self.CONFIG.rng.normal(loc = 0.0, scale = 0.1, size = self.CONFIG.CAMPAIGN_EMBEDDING_SIZE)
#            print(cs.embedding)
        self.stat_per_ctype = {ctype:FullStat() for ctype in campaign_types}
        for iid in range(self.CONFIG.IMPRESSIONS):
            time_s = 24*60*60 *iid / self.CONFIG.IMPRESSIONS  # linear time, for now
            self.run_one_auction(iid, time_s)
    
    def add_campaign(self, c: Campaign):
        c.set_config(self.CONFIG)
        c.set_cid(len(self.cs))
        self.cs.append(c)
        
    def print_stats(self):
        for c in self.cs:
            c.print_line_stat()
        print ("TOTAL -- Impressions: %i, Clicks: %i, Spend: %2.2f" % (self.stat.single.impressions, self.stat.single.clicks, self.stat.single.spend))
        print("Total spend by hour:")
        
        
        self.stat.draw_hourly_spend()
        self.stat.draw_hourly_cpm()
        CID = 10
        c = self.cs[CID]
        print("Spend of id %i:" % CID)
        c.stat.draw_hourly_spend()
        c.stat.draw_hourly_cpm()
        print("Got clicks: %2.2f, Max clicks: %2.2f, Click regret: %2.2f (assuming unlimited budgets)" % (self.got_fractional_clicks, self.max_fractional_clicks, self.max_fractional_clicks-self.got_fractional_clicks)) 
        START=000
        END=1000000
        
        plt.plot(c.win_times[START:END], c.diffs[START:END])
        plt.show()

def main():

    random.seed(10) # get deterministic behavior
    s = Simulation(CONFIG)
    s.add_campaign(CampaignStaticCPC(cpc = 0.05, daily_budget = 100000, hurdle = 1.0)) # unlimited budget back-stop campaign
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignStaticCPC(cpc = 0.1, daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignThrottledStaticCPC(cpc = 0.3, daily_budget = 100, hurdle = 1.0))
    s.add_campaign(CampaignPacedMinCPC(daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignPacedMinCPC(daily_budget = 200, hurdle = 1.0))
    s.add_campaign(CampaignPacedMinCPC(daily_budget = 100, hurdle = 1.0, time_start = 17 * 60 * 60, time_end = 20 * 60 * 60))
    s.run()
    s.print_stats()
    
# things one can show with Corcoran in current state:
'''
    1. Single PacedBudget campaign with 2x spend works exactly the same as 2 PacedBudget campagins with half of the spend
    2. 0.5 Hurdle makes PacedBudget campagin 2x more expensive
    3. Adding budget increases total spend less than budget, since what matters is replacement cpm
    4. Effect of second price auctions (variance goes down) 
'''    
    

if __name__ == '__main__':
    main()    
    
    