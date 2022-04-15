import torch
from torch import nn
from torch.utils.data import Dataset, DataLoader
import pandas as pd
import json



class FeatureDataset(Dataset):
    def __init__(self, filename) -> None:
        df = pd.read_csv(filename)
        x = df.iloc[0:, 1:].values
        y = df.iloc[0:, 0:1].values
        self.x_train = torch.tensor(x, dtype=torch.float32)
        self.y_train = torch.tensor(y, dtype=torch.float32)

    def __len__(self):
        return len(self.y_train)

    def __getitem__(self, idx):
        return self.x_train[idx], self.y_train[idx]


class Model(nn.Module):
    def __init__(self):
        super(Model, self).__init__()
        self.l0 = nn.Linear(121+121+121+121, 256)
        self.l1 = nn.Linear(256, 32)
        self.l2 = nn.Linear(32, 1)

    def forward(self, x):
        accum = self.l0(x)
        l1_x = torch.clamp(accum, 0.0, 1.0)
        l2_x = torch.clamp(self.l1(l1_x), 0.0, 1.0)
        return self.l2(l2_x)


train_set = FeatureDataset("nnue-data.csv")
train_loader = DataLoader(train_set, batch_size=10, shuffle=False)
test_set = FeatureDataset("nnue-data-test.csv")
test_loader = DataLoader(train_set, batch_size=10, shuffle=False)

model = Model()


# training
learning_rate = 1e-3
batch_size = 10
epochs = 1

loss_fn = nn.MSELoss()
# optimizer = torch.optim.SGD(model.parameters(), lr=learning_rate)
optimizer = torch.optim.Adam(model.parameters(), lr=learning_rate)

def train_loop(dataloader, model, loss_fn, optimizer):
    size = len(dataloader.dataset)
    for batch, (X, y) in enumerate(dataloader):
        # Compute prediction and loss
        pred = model(X)
        loss = loss_fn(pred, y)

        # Backpropagation
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        if batch % 100 == 0:
            loss, current = loss.item(), batch * len(X)
            # print(f"loss: {loss:>7f}  [{current:>5d}/{size:>5d}]")


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


def write_model_params(model):
    class TensorEncoder(json.JSONEncoder):
        def default(self, obj):
            if isinstance(obj, torch.Tensor):
                return obj.numpy().tolist()
            return json.JSONEncoder.default(self, obj)

    state_dict = model.state_dict()
    result = [
        {
            "weight": state_dict["l0.weight"],
            "bias": state_dict["l0.bias"],
        },
        {
            "weight": state_dict["l1.weight"],
            "bias": state_dict["l1.bias"],
        },
        {
            "weight": state_dict["l2.weight"],
            "bias": state_dict["l2.bias"],
        },
    ]
    with open("nnue_model.json", "w") as f:
        json.dump(result, f, cls=TensorEncoder, indent=2)

for t in range(epochs):
    print(f"Epoch {t+1} -------------------------------")
    train_loop(train_loader, model, loss_fn, optimizer)
    test_loop(test_loader, model, loss_fn)

# write json
write_model_params(model)

# write checkpoint
torch.save(model.state_dict(), "nnue_state_dict.pt")

print("Done!")
