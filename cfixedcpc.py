
import random
from campaign import Campaign
from impression import ImpressionOnOffer


class CampaignFixedCPC(Campaign):
    def __init__(self, tags = [], cpc = None, daily_budget = None):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.fixed_cpc = cpc
        self.daily_budget = daily_budget
        print("D: %2.2f" % daily_budget)
        self.type = "FixedCPC"
        pass
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        #campaign_pctr = ioo.impression_pctr * (1.0 + random.uniform(0.0, self.CONFIG.BASE_PCTR_CAMPAIGN_JITTER_PERCENT))
        #return self.fixed_cpc * campaign_pctr
        return self.fixed_cpc * ioo.impression_pctr

