import random

from impression import ImpressionOnOffer
from statistics import FullStat

class Campaign:
    def __init__(self, tags = [], daily_budget = None):
        self.daily_budget = daily_budget
        self.cid = None
        self.tags = tags
        self.type = None
        self.stat = FullStat()

    def set_cid(self, cid):
        self.cid = cid

    def set_config(self, CONFIG):
        self.CONFIG = CONFIG

    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        return 0.0

    def register_impression(self, ioo: ImpressionOnOffer, price: float):
        self.stat.register_impression(ioo, price)
        
    def register_click(self, ioo: ImpressionOnOffer, ):
        self.stat.register_click(ioo)
        
    def print_line_stat(self):
        print("CID: %i, Type: %s, Tags: %s, impressions: %i, clicks: %i, spend: %2.2f, avg cpc: %2.2f " % 
            (self.cid, 
            self.type, 
            str(self.tags), 
            self.stat.single.impressions, 
            self.stat.single.clicks, 
            self.stat.single.spend, 
            self.stat.single.spend / self.stat.single.clicks))
        
        
        
        
        
        
class CampaignFixedCPC(Campaign):
    def __init__(self, tags = [], cpc = None, daily_budget = None):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.fixed_cpc = cpc
        self.daily_budget = daily_budget
        self.type = "FixedCPC"
        pass
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        #campaign_pctr = ioo.impression_pctr * (1.0 + random.uniform(0.0, self.CONFIG.BASE_PCTR_CAMPAIGN_JITTER_PERCENT))
        #return self.fixed_cpc * campaign_pctr
        return self.fixed_cpc * ioo.impression_pctr


class CampaignTargetCPC(Campaign):
    def __init__(self, tags = [], cpc = None, daily_budget = None, pctr_miscalibration = 1.0):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.target_cpc = cpc
        self.daily_budget = daily_budget
        self.pctr_miscalibration = pctr_miscalibration
        self.type = "TargetCPC"
        pass
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        campaign_pctr = ioo.impression_pctr * (self.pctr_miscalibration + random.uniform(0.0, self.CONFIG.BASE_PCTR_CAMPAIGN_JITTER_PERCENT))
        #return self.fixed_cpc * campaign_pctr
        
        
        
        return self.target_cpc * campaign_pctr

