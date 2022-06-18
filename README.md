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

* CLI for ergonomic usage

## Future plans

Below are my plans for the future of this engine, in roughly descending order of
interest:

* Actual match data to support Elo estimates

* Futility and null-move pruning

* Tablebase support

* Opening book support

* PEXT sliding movegen on x86 architectures
