import random

from campaigns import 	Campaign, \
                        CampaignFixedCPC, \
                        CampaignTargetCPC, \
                        CampaignPacedFixedCPC, \
                        CampaignPacedBudget
from impression import ImpressionOnOffer    
from statistics import FullStat
    
        
class CONFIG:
    IMPRESSIONS = 100000
    BASE_CTR = 0.1
    BASE_CTR_JITTER_PERCENT = 0.01
    BASE_PCTR_CAMPAIGN_JITTER_PERCENT = 0.1
    BASE_PCTR_IMPRESSION_JITTER_PERCENT = 0.1
    


class Simulation:
    def __init__(self, config):
        self.stat = FullStat()
        self.cs : list[Campaign] = []
        self.CONFIG = config 
    
    def run_one_auction(self, iid, time_s):
        ioo = ImpressionOnOffer(self.CONFIG)
        ioo.init_properties(iid, time_s)
        # get highest bid
        win_bid = -1.0
        win_c = None
    #    print( "A")
        for c in self.cs:
            new_bid = c.get_bid(ioo)
    #        print (win_bid, win_c, new_bid)
            if new_bid:
              if new_bid > win_bid or (new_bid == win_bid and random.randrange(2) == 1): # tie breaking
                win_bid = new_bid
                win_c = c
        
        if win_c:
            win_c.register_impression(ioo, win_bid)
            self.stat.register_impression(ioo, win_bid)
            self.stat_per_ctype[win_c.type].register_impression(ioo, win_bid)
            return True
        else:
            return False
    
    
    def run(self):
        self.stat = FullStat()
        campaign_types = set(c.type for c in self.cs)
#        print (campaign_types)
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
#        self.stat.draw_hourly_spend()
        self.stat.draw_hourly_cpm()
        CID = 6
        print("Spend of id %i:" % CID)
        self.cs[CID].stat.draw_hourly_spend()
        self.cs[CID].stat.draw_hourly_cpm()

def main():

    random.seed(10) # get deterministic behavior
    s = Simulation(CONFIG)
    s.add_campaign(CampaignFixedCPC(cpc = 0.05, daily_budget = 100000)) # unlimited budget back-stop campaign
    s.add_campaign(CampaignFixedCPC(cpc = 0.1, daily_budget = 100))
    s.add_campaign(CampaignFixedCPC(cpc = 0.1, daily_budget = 100))
    s.add_campaign(CampaignFixedCPC(cpc = 0.1, daily_budget = 100))
    s.add_campaign(CampaignTargetCPC(cpc = 0.1, daily_budget = 100, pctr_miscalibration = 1.5))
    s.add_campaign(CampaignPacedFixedCPC(cpc = 0.3, daily_budget = 100))
    s.add_campaign(CampaignPacedBudget(daily_budget = 100))
    s.run()
    s.print_stats()
    
    
    

if __name__ == '__main__':
    main()    
    
    