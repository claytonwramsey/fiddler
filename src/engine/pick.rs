/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Move selection and phased generation.
//!
//! In order to search effectively, all alpha-beta searches require an effective move ordering which
//! puts the best moves first.
//! This move ordering is the move picker's job.
//!
//! The move picker generates moves "lazily," that is, by only performing move generations when it
//! has no other choice.
//! Often, playing the transposition move (found by looking up the position in the transposition
//! table) can be enough to cause a beta-cutoff, ending the earch in a subtree without ever having
//! to generate moves.
//!
//! In addition, moves are generated in phases so that not all moves are created at once.
//! Captures, being usually the most likely move to cause a cutoff, are generated first, before the
//! quiet moves.
//! However, some captures are extremely bad, and lose material on the spot.
//! Accordingly, those captures tagged with negative candidacy are sent straight to the back of the
//! move ordering.

use std::mem::swap;

use crate::base::{
    game::Game,
    movegen::{get_moves, is_legal, GenMode},
    Move,
};

use super::evaluate::{calculate_phase, eval_nl_delta, material, mg_npm_delta, Eval, Score};

/// Create an estimate for how good a move is.
/// `delta` is the PST difference created by this move.
/// Requires that `m` must be a legal move in `b`.
///
/// # Panics
///
/// This function may panic if the given move is illegal.
pub fn candidacy(g: &Game, m: Move, delta: Score, phase: u8) -> Eval {
    // Worst case, we don't keep the piece we captured. Subtract off the value of the mover from
    // the difference in cumulative evaluation.
    (delta - material::value(g[m.from_square()].unwrap().0)).blend(phase)
}

#[derive(Clone, Debug, PartialEq)]
/// A move which has been tagged with its effects on a game.
pub struct TaggedMove {
    /// The move which is tagged.
    pub m: Move,
    /// The new quantity of mid-game non-pawn material in the game after this move is played.
    pub new_mg_npm: Eval,
    /// The phase of the position being evaluated after this move is played.
    pub phase: u8,
    /// The heuristic quality of this move.
    /// Will be higher for better moves.
    pub quality: Eval,
}

#[derive(Clone, Debug, PartialEq)]
/// A structure which generates legal moves for searching.
pub struct MovePicker {
    /// The buffer of captures to select from, paired with their PST deltas and then their final
    /// candidacies.
    capture_buffer: Vec<TaggedMove>,
    /// The buffer of quiet moves to select from, paired with their PST deltas and then their final
    /// candidacies.
    quiet_buffer: Vec<TaggedMove>,
    /// The index in the move buffer of the capture to give (initialized to 0).
    capture_index: usize,
    /// The index in the move buffer of the quiet move to give (initialized to 0).
    quiet_index: usize,
    /// The set of moves to ignore.
    ignored: Vec<Move>,
    /// The upcoming phase of move generation.
    phase: PickPhase,
    /// The move retreived from the transposition table.
    transposition_move: Option<Move>,
    /// The killer move.
    killer_move: Option<Move>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// The possible phases of move generation.
enum PickPhase {
    /// Pick the move from the transposition table next.
    Transposition,
    /// Prior to picking good captures, so captures must be generated.
    PreGoodCapture,
    /// Pick good captures, which have already been generated
    GoodCapture,
    /// Pick the killer move next.
    Killer,
    /// Prior to picking quiet moves, so quiet moves must be generated.
    PreQuiet,
    /// Pick quiet moves.
    Quiet,
    /// Pick the bad captures.
    BadCaptures,
}

impl MovePicker {
    /// Construct a `MovePicker` for a given position.
    /// Will generate moves, so it should only be created at a point in the search where moves must
    /// be generated.
    ///
    /// The transposition move must be legal, and should be checked as such prior to instantiation.
    pub fn new(transposition_move: Option<Move>, killer_move: Option<Move>) -> MovePicker {
        MovePicker {
            capture_buffer: Vec::new(),
            quiet_buffer: Vec::new(),
            capture_index: 0,
            quiet_index: 0,
            ignored: Vec::new(),
            phase: PickPhase::Transposition,
            transposition_move,
            killer_move,
        }
    }

    /// Add a move to the set of moves that should be ignored.
    /// Requires that `m` is not `Move::BAD_MOVE`.
    fn ignore(&mut self, m: Move) {
        if !self.ignored.contains(&m) {
            self.ignored.push(m);
        }
    }

    /// Get the next move which the move picker believes is worth playing.
    ///
    /// # Inputs
    ///
    /// - `g`: The game that moves should be generated on. This game must always be in the same
    ///   state that `next` on this move generator was first called with.
    /// - `phase`: The current calculated phase of `g`. This must be the return value of
    ///   [`fiddler::engine::evaluate::phase_of`].
    ///
    /// # Returns
    ///
    /// This function will return `Some` if there is another move to be evaluated, and `None` if no
    /// such moves exist.
    /// Inside of the `Some` variant, there are 2 elements:
    ///
    /// 1. The next move to be played.
    /// 2. A structure containing extrThe new cumulative evaluation of the position after the move
    /// is played. 3. The quantity of mid-
    pub fn next(&mut self, g: &Game, current_mg_npm: Eval) -> Option<TaggedMove> {
        match self.phase {
            PickPhase::Transposition => {
                self.phase = PickPhase::PreGoodCapture;
                match self.transposition_move {
                    None => self.next(g, current_mg_npm),
                    Some(m) => {
                        // we assume that m was checked for legality before
                        self.ignore(m);
                        Some(TaggedMove::new(g, m, current_mg_npm))
                    }
                }
            }
            PickPhase::PreGoodCapture => {
                // generate moves, and then move along
                self.phase = PickPhase::GoodCapture;
                get_moves::<{ GenMode::Captures }>(g, |m| {
                    self.capture_buffer
                        .push(TaggedMove::new(g, m, current_mg_npm));
                });
                self.next(g, current_mg_npm)
            }
            PickPhase::GoodCapture => {
                /// The cutoff for good captures.
                ///
                /// Captures with a score below this value will be sent to the back of the list.
                const GOOD_CAPTURE_CUTOFF: Eval = Eval::centipawns(-300);

                if self.capture_index >= self.capture_buffer.len() {
                    // out of captures
                    self.phase = PickPhase::Killer;
                    return self.next(g, current_mg_npm);
                }
                let capture_entry = select_best(&mut self.capture_buffer, self.capture_index);
                if capture_entry.quality < GOOD_CAPTURE_CUTOFF {
                    // we are now in bad captures, move on
                    self.phase = PickPhase::Killer;
                    // make sure to leave this move in place
                    self.capture_buffer[self.capture_index] = capture_entry;
                    return self.next(g, current_mg_npm);
                }
                // make sure to get a new capture next time
                self.capture_index += 1;
                if self.ignored.contains(&capture_entry.m) {
                    // don't bother with ignored moves
                    return self.next(g, current_mg_npm);
                }
                Some(capture_entry)
            }
            PickPhase::Killer => {
                self.phase = PickPhase::PreQuiet;
                match self.killer_move {
                    None => self.next(g, current_mg_npm),
                    Some(m) => {
                        if is_legal(m, g) {
                            self.ignore(m);
                            Some(TaggedMove::new(g, m, current_mg_npm))
                        } else {
                            self.next(g, current_mg_npm)
                        }
                    }
                }
            }
            PickPhase::PreQuiet => {
                // generate quiet moves
                self.phase = PickPhase::Quiet;
                get_moves::<{ GenMode::Quiets }>(g, |m| {
                    self.quiet_buffer
                        .push(TaggedMove::new(g, m, current_mg_npm));
                });
                self.next(g, current_mg_npm)
            }
            PickPhase::Quiet => {
                if self.quiet_index >= self.quiet_buffer.len() {
                    // out of quiets
                    self.phase = PickPhase::BadCaptures;
                    return self.next(g, current_mg_npm);
                }
                let quiet_entry = select_best(&mut self.quiet_buffer, self.quiet_index);
                self.quiet_index += 1;
                if self.ignored.contains(&quiet_entry.m) {
                    // don't bother with ignored moves
                    return self.next(g, current_mg_npm);
                }
                Some(quiet_entry)
            }
            PickPhase::BadCaptures => {
                if self.capture_index >= self.capture_buffer.len() {
                    // all out of moves!
                    return None;
                }
                let capture_entry = select_best(&mut self.capture_buffer, self.capture_index);
                self.capture_index += 1;
                if self.ignored.contains(&capture_entry.m) {
                    // don't bother with ignored moves
                    return self.next(g, current_mg_npm);
                }
                Some(capture_entry)
            }
        }
    }
}

impl TaggedMove {
    /// Construct a new [`TaggedMove`] by tagging `m`, which is about to be played on `g`, while
    /// the current quantity of midgame non-pawn material is `current_mg_npm`.
    pub fn new(g: &Game, m: Move, current_mg_npm: Eval) -> TaggedMove {
        let new_mg_npm = current_mg_npm + mg_npm_delta(m, g);
        let phase = calculate_phase(new_mg_npm);
        TaggedMove {
            m,
            new_mg_npm,
            phase,
            quality: candidacy(g, m, eval_nl_delta(m, g), phase),
        }
    }
}

/// Search through `moves` until we find the best move, sorting as we go.
/// After this function terminates, `moves[idx]` will contain the best-rated move of the input moves
/// from idx to the end.
/// Requires that `0 <= idx < moves.len()`.
///
/// The astute reader will realize that this search implementation is actually _O(n^2)_.
/// However, we trust that a beta cutoff will be quick under most circumstances, meaning that
/// the "true" runtime is _Ω(n)_.
/// Empirically, this performance improvement yields ~55 Elo.
fn select_best(moves: &mut [TaggedMove], idx: usize) -> TaggedMove {
    let mut best_entry = moves[idx].clone();
    for entry in moves.iter_mut().skip(idx + 1) {
        // insertion sort to get the best move.
        // insertion sort is slower if we need to see every move,
        // but often we don't due to beta cutoff
        if entry.quality > best_entry.quality {
            // swap out the next-best move
            swap(entry, &mut best_entry);
        }
    }

    best_entry
}

#[cfg(test)]
mod tests {

    use crate::base::{game::Game, movegen::make_move_vec};

    use super::*;

    #[test]
    /// Test that all moves are generated in the move picker and that there are no duplicates.
    fn generation_correctness() {
        let g = Game::from_fen("r2q1rk1/ppp2ppp/3b4/4Pb2/4Q3/2PB4/P1P2PPP/R1B1K2R w KQ - 5 12")
            .unwrap();
        let mut mp = MovePicker::new(None, None);

        let gen_moves = make_move_vec::<{ GenMode::All }>(&g);

        {
            let mut z = mp.clone();
            while let Some(tm) = z.next(&g, Eval::DRAW) {
                let m = tm.m;
                assert!(gen_moves.contains(&m));
                println!("{m:?}");
            }
        }

        for &m in &gen_moves {
            let mut z = mp.clone();

            'found: {
                while let Some(tm) = z.next(&g, Eval::DRAW) {
                    if tm.m == m {
                        break 'found;
                    }
                }

                panic!("did not find move {m}");
            }
        }

        // check that the count is equal to make sure there are no repetitions
        let mut total = 0;
        while mp.next(&g, Eval::DRAW).is_some() {
            total += 1;
        }
        assert_eq!(total, gen_moves.len());
    }
}
