import torch
from torch import nn


class Model(nn.Module):
    def __init__(self):
        super(Model, self).__init__()
        self.l0 = nn.Linear(121+121+121+121, 256)
        self.l1 = nn.Linear(256, 32)
        self.l2 = nn.Linear(32, 1)

        # for quantization aware training
        self.weight_clipping = [
            {'params' : [self.l0.weight], 'min_weight' : -127/64, 'max_weight' : 127/64 },
            {'params' : [self.l1.weight], 'min_weight' : -127/64, 'max_weight' : 127/64 },
            {'params' : [self.l2.weight], 'min_weight' : -127/64, 'max_weight' : 127/64 },
            # {'params' : [self.output.weight], 'min_weight' : -127*127/9600, 'max_weight' : 127*127/9600 },
        ]

    def forward(self, x):
        x = self.l0(x) # accum
        x = torch.clamp(x, 0.0, 1.0)
        x = self.l1(x)
        x = torch.clamp(x, 0.0, 1.0)
        x = self.l2(x)
        return x

    # for quantization aware training
    def _clip_weights(self):
        for group in self.weight_clipping:
            for p in group['params']:
                p_data_fp32 = p.data
                min_weight = group['min_weight']
                max_weight = group['max_weight']
                p_data_fp32.clamp_(min_weight, max_weight)
                p.data.copy_(p_data_fp32)

