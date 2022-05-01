from torch import nn
from torch.utils.data import DataLoader
import torch
import json

from .dataset import FeatureDataset
from .model import Model


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


train_set = FeatureDataset("nnue-data.csv")
train_loader = DataLoader(train_set, batch_size=10, shuffle=False)
test_set = FeatureDataset("nnue-data-test.csv")
test_loader = DataLoader(test_set, batch_size=10, shuffle=False)

model = Model()
model.train()

# training
learning_rate = 1e-3
batch_size = 10
epochs = 20

loss_fn = nn.MSELoss()
optimizer = torch.optim.Adam(model.parameters(), lr=learning_rate)

for t in range(epochs):
    print(f"Epoch {t+1} -------------------------------")
    train_loop(train_loader, model, loss_fn, optimizer)
    test_loop(test_loader, model, loss_fn)

model.eval()

# write json
write_model_params(model)

# write checkpoint
torch.save(model.state_dict(), "nnue_state_dict.pt")

print("Done!")
