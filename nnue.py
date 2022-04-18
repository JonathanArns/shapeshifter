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
        scaling_factor = 20
        self.y_train = torch.sigmoid(self.y_train / scaling_factor)

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

        # for quantization
        model._clip_weights()

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
    """
    Performs the quantization of the model params and then writes them to a json file.
    Currently scaling by factor of 64.
    """
    class TensorEncoder(json.JSONEncoder):
        def default(self, obj):
            if isinstance(obj, torch.Tensor):
                return obj.numpy().tolist()
            return json.JSONEncoder.default(self, obj)

    state_dict = model.state_dict()
    result = [
        {
            # ONLY The feature transformer's weights are transposed
            # This is done to enable a more efficient impl of the forward pass
            "weight": torch.transpose(state_dict["l0.weight"].mul(64).type(torch.int8), 0, 1),
            "bias": state_dict["l0.bias"].mul(64).type(torch.int16),
        },
        {
            "weight": state_dict["l1.weight"].mul(64).type(torch.int8),
            "bias": state_dict["l1.bias"].mul(64).type(torch.int16),
        },
        {
            "weight": state_dict["l2.weight"].mul(64).type(torch.int8),
            "bias": state_dict["l2.bias"].mul(64).type(torch.int16),
        },
    ]
    with open("nnue_model.json", "w") as f:
        json.dump(result, f, cls=TensorEncoder, indent=2)



train_set = FeatureDataset("nnue-data.csv")
train_loader = DataLoader(train_set, batch_size=10, shuffle=False)
test_set = FeatureDataset("nnue-data-test.csv")
test_loader = DataLoader(train_set, batch_size=10, shuffle=False)

model = Model()
model.train()

# training
learning_rate = 1e-3
batch_size = 10
epochs = 2

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
