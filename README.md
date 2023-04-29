# Fiddler: a chess engine

Fiddler is a chess engine developed by Clayton Ramsey as a hobby project.
It's written in Rust, with an emphasis on ergonomic usage and good performance.
Right now, I would guess that its playing quality is roughly 2100 Elo, on par with a master-level
player.

## Features

- Full UCI support

- Multi-threaded search

- Phased move generation

- Principal variation search (with quiescence)

- Evaluation with piece-square tables, mobility, and some handwritten rules

- Integrated gradient descent tuner

## Usage

Fiddler uses nightly, unstable Rust, mostly to gain access to LLVM's prefetch intrinsic.
As a result, you must use the nightly compiler to compile this code.
Because [`rust-toolchain.toml`](rust-toolchain.toml) specifies the Rust channel, you do not
need to do anything special to make this work.

To create the main UCI executable, navigate to the root of this repository and run
`cargo build --release --bin fiddler`.
This will then create the executable `target/release/fiddler` (or `target/release/fiddler.exe` for
Windows users).

You can also create a tuner executable.
To do so, run `cargo build --release --bin tune`.

Fiddler uses features from relatively new versions of Rust, so you may need to update your
installation of Rust to compile Fiddler. To do so, you can simply invoke
`rustup update && rustup upgrade`.

## UCI options supported

- `Thread Count`: Set the number of worker threads for searching.
  _Warning_: since there are currently no heuristics for differentiating search threads, increasing
  `Thread Count` to more than 1 will likely reduce perfomance.

- `Hash`: Set the transposition table size, in megabytes.

## Future plans

Below are my plans for the future of this engine, in roughly descending order of
interest:

- Fix up UCI behavior to be more spec compliant
  - Negative time remaining
  - Better setup for `GoOption`

- Shrink the mailbox representation by using colored pieces

- Add methods to differentiate search threads

- Support ponderhit and other UCI commands

- Actual match data to support Elo estimates

- Futility and null-move pruning

- Tablebase support

- Opening book support

- Develop intelligent time-management schemes

- PEXT sliding movegen on x86 architectures

## Contributing

If you are interested in contributing to Fiddler, please open a pull request.
Any help is welcome!

## License

This code is licensed under the GNU GPLv3. For mor information, refer to [LICENSE.md](LICENSE.md).
