"""
A training script for NNUEs (Efficiently Updatable Neural Networks) for this engine.
"""

# TODO: actually implement the training script :)

from fire import Fire
import torch
import csv
import tqdm


def main(train_path: str):
    X, y = load_training_data(train_path)
    # problem: cannot do this with full size dataset
    X = X.to_dense()
    model = Nnue()
    optimizer = torch.optim.Adam(model.parameters(), lr=1e-3)
    loss_fn = torch.nn.MSELoss()

    print("Training model...")
    for i in tqdm.trange(10):
        pred = model(X)
        loss = loss_fn(pred, y)

        loss.backward()
        optimizer.step()
        optimizer.zero_grad()

        print(f"{loss=:>7f}")


def load_training_data(train_path: str) -> tuple[torch.Tensor, torch.Tensor]:

    print("Loading training data....")
    x_indices = []
    y = []
    with open(train_path) as f:
        for i, (x1_s, x2_s, y_s) in tqdm.tqdm(
            enumerate(csv.reader(f, delimiter=";", lineterminator="\n"))
        ):
            x_indices.extend(parse_vector(i, 0, x1_s))
            x_indices.extend(parse_vector(i, 1, x2_s))
            y.append(float(y_s))
    nrows = len(y)
    xi = torch.tensor(x_indices, dtype=torch.int32).T
    nnz = xi.shape[1]
    xv = torch.ones(nnz, dtype=torch.int8)
    x = torch.sparse_coo_tensor(xi, xv, (nrows, 2, 64 * 64 * 11), dtype=torch.float32)
    print(f"Loaded {nrows} rows with {nnz} observed features")
    return (x, torch.tensor(y))


def parse_vector(row_id: int, side_id: int, row: str) -> list[int]:
    return [(row_id, side_id, int(x)) for x in row.split(",")]


class Nnue(torch.nn.Module):
    def __init__(self):
        super().__init__()
        self.front_seq = torch.nn.Sequential(torch.nn.Linear(64 * 64 * 11, 256), torch.nn.ReLU())
        self.out_seq = torch.nn.Sequential(
            torch.nn.Linear(512, 32),
            torch.nn.ReLU(),
            torch.nn.Linear(32, 32),
            torch.nn.ReLU(),
            torch.nn.Linear(32, 1),
            torch.nn.Sigmoid(),
        )

    def forward(self, x):
        # problem: cannot do this with sparse matrix
        a1 = self.front_seq(torch.select(x, -2, 0))
        a2 = self.front_seq(torch.select(x, -2, 1))
        a = torch.concatenate((a1, a2), dim=-1)
        return self.out_seq(a)


if __name__ == "__main__":
    Fire(main)
