import torch
from torch.utils.data import Dataset
import pandas as pd


class FeatureDataset(Dataset):
    def __init__(self, filename) -> None:
        df = pd.read_csv(filename)
        x = df.iloc[0:, 1:].values
        y = df.iloc[0:, 0:1].values
        self.x_train = torch.tensor(x, dtype=torch.float32)
        self.y_train = torch.tensor(y, dtype=torch.float32)
        scaling_factor = 1
        self.y_train = torch.sigmoid(self.y_train / scaling_factor)

    def __len__(self):
        return len(self.y_train)

    def __getitem__(self, idx):
        return self.x_train[idx], self.y_train[idx]

