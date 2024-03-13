"""
A training script for NNUEs (Efficiently Updatable Neural Networks) for this engine.
"""

# TODO: actually implement the training script :)

from fire import Fire
import torch


def main():
    x = torch.rand(5, 3)
    print(x)


class Nnue(torch.nn.Module):
    def __init__(self):
        super().__init__()
        self.front_seq = torch.nn.Sequential(torch.nn.Linear(64 * 64 * 9, 256), torch.nn.ReLU())
        self.out_seq = torch.nn.Sequential(
            torch.nn.Linear(512, 32),
            torch.nn.ReLU(),
            torch.nn.Linear(32, 32),
            torch.nn.ReLU(),
            torch.nn.Linear(32, 1),
            torch.nn.Sigmoid(),
        )

    def forward(self, x):
        a1 = self.front_seq(x[0, :])
        a2 = self.front_seq(x[1, :])
        a = torch.concatenate(a1, a2)
        return self.out_seq(a)


if __name__ == "__main__":
    Fire(main)
