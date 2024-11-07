import math
import random
import collections
import functools

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
        print("CID: %i, Type: %s, Tags: %s, impressions: %i, clicks: %i, spend: %2.2f, cpc: %2.3f, cpm: %2.3f" % 
            (self.cid, 
            self.type, 
            str(self.tags), 
            self.stat.single.impressions, 
            self.stat.single.clicks, 
            self.stat.single.spend, 
            self.stat.single.spend / self.stat.single.clicks if self.stat.single.clicks else 0.0,
            1000.0 * self.stat.single.spend / self.stat.single.impressions if self.stat.single.impressions else 0.0))
        
        
        
        
class CampaignFixedCPC(Campaign):
    def __init__(self, tags = [], cpc = None, daily_budget = None):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.fixed_cpc = cpc
        self.daily_budget = daily_budget
        self.type = "CPC"
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        #campaign_pctr = ioo.impression_pctr * (1.0 + random.uniform(0.0, self.CONFIG.BASE_PCTR_CAMPAIGN_JITTER_PERCENT))
        #return self.fixed_cpc * campaign_pctr
        return self.fixed_cpc * ioo.impression_pctr


class CampaignTargetCPC(Campaign):
    # TODO
    def __init__(self, tags = [], cpc = None, daily_budget = None, pctr_miscalibration = 1.0):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.target_cpc = cpc
        self.daily_budget = daily_budget
        self.pctr_miscalibration = pctr_miscalibration
        self.type = "TargetCPC"
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        campaign_pctr = ioo.impression_pctr * (self.pctr_miscalibration + random.uniform(0.0, self.CONFIG.BASE_PCTR_CAMPAIGN_JITTER_PERCENT))
        #return self.fixed_cpc * campaign_pctr
        
        
        
        return self.target_cpc * campaign_pctr

class CampaignPacedFixedCPC(Campaign):
    def __init__(self, tags = [], cpc = None, daily_budget = None):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(cpc != None)
        assert(daily_budget != None)
        self.fixed_cpc = cpc
        self.daily_budget = daily_budget
        self.type = "PacedCPC"
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        
        expected_spend = self.daily_budget * ioo.time_s / 86400.0
#        print(expected_spend)
        if self.stat.single.spend >= expected_spend:
            return None
        
        
        return self.fixed_cpc * ioo.impression_pctr

SLOW_T = 1000
FAST_T = 100
class CampaignPacedBudget(Campaign):
    def __init__(self, tags = [], daily_budget = None):
        Campaign.__init__(self, tags = tags, daily_budget = daily_budget)
        assert(daily_budget != None)
        self.daily_budget = daily_budget
        self.type = "PacedSpend"
        self.output_price = 0.1
        self.exp_avg_pace_slow = 0
        self.exp_avg_pace_fast = 0
        self.last_win_time = -1
        self.last_bid_time = -1
        
        
    def get_bid(self, ioo: ImpressionOnOffer) -> float:
        if self.stat.single.spend >= self.daily_budget:
            return None
        if self.last_bid_time == -1:
            self.last_bid_time = ioo.time_s
            return self.output_price        
        expected_spend = self.daily_budget * ioo.time_s / 86400.0
        remaining_time = 86400 - ioo.time_s
        remaining_spend = self.daily_budget - self.stat.single.spend
        assert(remaining_time > 0.0)
        remaining_desired_pace = remaining_spend / remaining_time
        
        
#        print("Span_time: %f, span_spend: %f" % (span_time, span_spend))
#        print("A iid: %i, span_pace: %2.5f desired pace: %2.5f, price: %2.5f" % (ioo.iid, span_pace, remaining_desired_pace, self.output_price))
        p_time_diff = ioo.time_s - self.last_win_time

        exp_avg_pace_slow = self.exp_avg_pace_slow + (1.0 - math.exp(-p_time_diff/SLOW_T)) * (- self.exp_avg_pace_slow)
        exp_avg_pace_fast = self.exp_avg_pace_fast + (1.0 - math.exp(-p_time_diff/FAST_T)) * (- self.exp_avg_pace_fast)

        if exp_avg_pace_slow < remaining_desired_pace and exp_avg_pace_fast < remaining_desired_pace:
            proportional = (remaining_desired_pace / exp_avg_pace_slow - 1.0) / 1 + 1.0
            self.output_price = self.output_price * proportional
#            print("IMP UP iid: %i, Expected avg: %2.5f Slow avg: %2.5f, fast avg: %2.4f, price: %2.4f" % (ioo.iid, remaining_desired_pace, exp_avg_pace_slow, exp_avg_pace_fast, self.output_price))
        else:
#            print("IMP CO iid: %i, Expected avg: %2.5f Slow avg: %2.5f, fast avg: %2.4f, price: %2.4f" % (ioo.iid, remaining_desired_pace, exp_avg_pace_slow, exp_avg_pace_fast, self.output_price))
            pass

        self.last_bid_time = ioo.time_s
        return self.output_price
        
        

    def register_impression(self, ioo: ImpressionOnOffer, price: float):
        Campaign.register_impression(self, ioo, price)
        remaining_time = 86400 - ioo.time_s
        remaining_spend = self.daily_budget - self.stat.single.spend
        assert(remaining_time > 0.0)
        remaining_desired_pace = remaining_spend / remaining_time

        if self.last_win_time < 0.0:
            # assume one impression at desired pace happened at the start of time
            print (ioo.time_s)
#            self.exp_avg_pace_slow = (1.0 - math.exp(-(ioo.time_s-1)/SLOW_T)) * (- remaining_desired_pace * 1.1)
#            self.exp_avg_pace_fast = (1.0 - math.exp(-(ioo.time_s-1)/FAST_T)) * (- remaining_desired_pace * 1.1)
            self.exp_avg_pace_fast = remaining_desired_pace
            self.exp_avg_pace_slow = remaining_desired_pace
            #print("INITIAL Exp avg pace: %2.5f" % (self.exp_avg_pace * 1000))
            
        # on win, we possibly want to decrease output_price
        p_time_diff = ioo.time_s - self.last_win_time
        self.exp_avg_pace_slow += (1.0 - math.exp(-p_time_diff/SLOW_T)) * ((price/p_time_diff) - self.exp_avg_pace_slow)
        self.exp_avg_pace_fast += (1.0 - math.exp(-p_time_diff/FAST_T)) * ((price/p_time_diff) - self.exp_avg_pace_fast)
        #print("Exp avg pace: %2.5f" % (self.exp_avg_pace * 1000))
        expected_spend = self.daily_budget * ioo.time_s / 86400.0
        
#        print(self.last_time_price_pairs)
#        if ioo.iid > 90000 :
#            print("Span_time: %f, span_spend: %f" % (span_time, span_spend))
        proportional_base = remaining_desired_pace / self.exp_avg_pace_fast
#        print("F:", self.exp_avg_pace_fast, self.exp_avg_pace_slow)
        if self.exp_avg_pace_fast > remaining_desired_pace:
            proportional = (proportional_base - 1.0) / 1.0 + 1.0
            proportional = max(0.1, proportional)
            self.output_price = self.output_price * proportional
  #          print("WIN DROP iid: %i, Expected avg: %2.4f Slow avg: %2.4f, fast avg: %2.4f, proportional: %2.4f, next price: %2.4f " % (ioo.iid, remaining_desired_pace, self.exp_avg_pace_slow, self.exp_avg_pace_fast, proportional, self.output_price))
        else:
 #           print("WIN CONSTANT iid: %i, Expected avg: %2.4f Slow avg: %2.4f, fast avg: %2.4f, next price: %2.4f" % (ioo.iid, remaining_desired_pace,  self.exp_avg_pace_slow, self.exp_avg_pace_fast, self.output_price))
            pass

        self.last_win_time = ioo.time_s

        
        
        
