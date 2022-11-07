# Fiddler: a chess engine

Fiddler is a chess engine developed by Clayton Ramsey as a hobby project.
It's written in Rust, with an emphasis on ergonomic usage and good performance.
Right now, I would guess that its playing quality is roughly 2100 Elo, on par
with a master-level player.

## Features

- Full UCI support

- Multi-threaded search

- Phased move generation

- Principal variation search (with quiescence)

- Evaluation with piece-square tables, mobility, and some handwritten rules

- Integrated gradient descent tuner

## Usage

Fiddler uses nightly, unstable Rust, mostly to gain access to LLVM's prefetch
intrinsic.
As a result, you must use the nightly compiler to compile this code.
The most simple way of doing this is by running `rustup default stable` before
proceeding.

To create the main UCI executable, navigate to the root of this repository and
run `cargo build --release --bin fiddler`.
This will then create the executable `target/release/fiddler` (or
`target/release/fiddler.exe` for Windows users).

You can also create a tuner executable.
To do so, run `cargo build --release --bin tune`.

Fiddler uses features from relatively new versions of Rust, so you may need to
update your installation of Rust to compile Fiddler. To do so, you can simply
invoke `rustup update`.

### Building with a specific target in mind

If you want to have a build which is fully optimized for your machine, you can
set your machine as the target architecture.
To do this, learn your target triple by running `rustc -vV` and reading the
`host` line.
For an example of how to do this, here's the output on my machine:

```sh
$ rustc -vV
rustc 1.63.0 (4b91a6ea7 2022-08-08)
binary: rustc
commit-hash: 4b91a6ea7258a947e59c6522cd5898e7c0a6a88f
commit-date: 2022-08-08
host: x86_64-pc-windows-gnu
release: 1.63.0
LLVM version: 14.0.5
```

Once you have obtained the target triple (in my case, `x86_64-pc-windows-gnu`),
you can then build with a single target architecture.

```sh
cargo build --release --bin engine --target=<your target triple here>
```

This will then create a a new directory in the `target` folder named after your
target triple containing the target-optimized binary.
In my case, the path to the binary is
`./target/x86_64-pc-windows-gnu/release/engine.exe`.

## UCI options supported

- `Thread Count`: Set the number of worker threads for searching.
  _Warning_: since there are currently no heuristics for differentiating
  search threads, increasing `Thread Count` to more than 1 will likely reduce
  perfomance.

- `Hash`: Set the transposition table size, in megabytes.

## Future plans

Below are my plans for the future of this engine, in roughly descending order of
interest:

- Add methods to differentiate search threads

- Support ponderhit and other UCI commands

- Actual match data to support Elo estimates

- Futility and null-move pruning

- Tablebase support

- Opening book support

- Add loads of doctests to make usage more clear

- Develop intelligent time-management schemes

- PEXT sliding movegen on x86 architectures

## Contributing

If you are interested in contributing to Fiddler, please open a pull request.
Any help is welcome!

## License

This code is licensed under the GNU GPLv3. For mor information, refer to
`LICENSE.md`.
