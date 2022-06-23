# Fiddler: a chess engine

Fiddler is a chess engine developed by Clayton Ramsey as a hobby project. It's
written in Rust, with an emphasis on ergonomic usage and good performance. Right
now, I would guess that its playing quality is roughly 2400 Elo, on par with an
international master.

## Features

* Full UCI support

* Multi-threaded search

* Phased move generation

* Principal variation search (with quiescence)

* Piece-square table evaluation, plus some simple heuristics

* Integrated gradient descent tuner

## Usage

Fiddler is built like any Rust project, but has multiple binary files. To create
the main UCI executable, navigate to the root of this repository and run
`cargo build --release --bin fiddler_engine`. This will then create the
executable `target/release/fiddler_engine` (or
`target/release/fiddler_engine.exe` for Windows users).

## Future plans

Below are my plans for the future of this engine, in roughly descending order of
interest:

* Shuffle root move ordering across threads to improve parallelism speedup

* Send back pondermoves

* Actual match data to support Elo estimates

* Futility and null-move pruning

* Tablebase support

* Move away from `rusqlite` for tuner DB access so that the tuner can be run on
non-Linux platforms

* Opening book support

* Add a mate-searching thread

* Add loads of doctests to make usage more clear

* Develop intelligent time-management schemes

* PEXT sliding movegen on x86 architectures

## Known issues

* There seems to be some search instability. It's unclear exactly where this is
coming from, but searches to the same depth can sometimes yield differing
results.

## File structure

Fiddler currently consists of four crates:

* `base` contains common definitions across all of Fiddler, such as board state
and move generation.

* `engine` contains all code for running the Fiddler engine, including the main
UCI executable.

* `tuner` contains the tuner, which will automatically tune constant values for
evaluation.

* `cli` contains a command-line interface for testing the engine. We intend to
eventually retire the CLI.

## Contributing

If you are interested in contributing to Fiddler, please open a pull request.
Any help is welcome! If you do submit a pull request, make sure to add your name
to `AUTHORS.md`.
