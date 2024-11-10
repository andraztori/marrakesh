    
import random
from helpers import sigmoid
        
class ImpressionOnOffer:
    def __init__(self, iid: int, time_s: float, config):
        self.iid = iid
        self.time_s = time_s 
        self.embedding = config.rng.normal(loc = 0.0, scale = 0.2, size = len(config.campaign_base_embedding))
        self.impression_ctr = sigmoid(self.embedding.dot(config.campaign_base_embedding) + config.base_intercept)
#        self.impression_ctr = config.BASE_CTR * (1 + - config.BASE_CTR_JITTER_PERCENT + random.uniform(0.0, config.BASE_CTR_JITTER_PERCENT * 2))          # HERE IT IS PCTR, not CTR
        
        # we just make it up if there was a click or not - ahead of time
        self._clicked = random.uniform(0.0, 1.0) < self.impression_ctr * (1.0 - config.BASE_CTR_JITTER_PERCENT + random.uniform(0.0, config.BASE_CTR_JITTER_PERCENT * 2))          # HERE IT IS PCTR, not CTR
        

    def has_been_clicked(self) -> bool:
        return self._clicked        

