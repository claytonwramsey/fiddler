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

## Future plans

Below are my plans for the future of this engine, in roughly descending order of
interest:

* Actual match data to support Elo estimates

* Futility and null-move pruning

* Tablebase support

* Opening book support

* PEXT sliding movegen on x86 architectures

## Known issues

* There seems to be some search instability. It's unclear exactly where this is
coming from, but searches to the same depth can sometimes yield differing
results.

* On rare occasion, sometimes the engine misses simple one-move blunders. I'm
also not sure what causes this.
