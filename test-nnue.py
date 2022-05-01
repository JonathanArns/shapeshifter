from torch import nn
from torch.utils.data import DataLoader
import torch
import json

from nnue.dataset import FeatureDataset
from nnue.model import Model


def test_loop(dataloader, model, loss_fn):
    size = len(dataloader.dataset)
    num_batches = len(dataloader)
    test_loss, correct = 0, 0

    with torch.no_grad():
        for X, y in dataloader:
            pred = model(X)
            test_loss += loss_fn(pred, y).item()
            correct += torch.isclose(pred, y, 1, 0).type(torch.float).sum().item()

    test_loss /= num_batches
    correct /= size
    print(f"Test Error: \n Accuracy: {(100*correct):>0.1f}%, Avg loss: {test_loss:>8f} \n")


test_set = FeatureDataset("nnue-data-test.csv")
test_loader = DataLoader(test_set, batch_size=10, shuffle=False)

model = Model()
model.load_state_dict(torch.load("nnue_state_dict.pt"))
model.eval()


print(model.forward(test_set[0][0]).logit())
