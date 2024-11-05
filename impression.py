    
import random
        
class ImpressionOnOffer:
    def __init__(self, CONFIG):
        self.CONFIG = CONFIG
        self.pctr = None
        pass

    def init_properties(self, iid: int, time_s: float):
        self.impression_pctr = self.CONFIG.BASE_CTR * (1.0 + random.uniform(0.0, self.CONFIG.BASE_PCTR_IMPRESSION_JITTER_PERCENT))
        self.iid = iid
        self.time_s = time_s
        # we just make it up if there was a click or not - ahead of time
        self._clicked = random.uniform(0.0, 1.0) < self.CONFIG.BASE_CTR * (1.0 + random.uniform(0.0, self.CONFIG.BASE_CTR_JITTER_PERCENT))          # HERE IT IS PCTR, not CTR
        

    def has_been_clicked(self) -> bool:
        return self._clicked        

