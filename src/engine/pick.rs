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
    movegen::{get_moves, is_legal, GenMode},
    Board, Move,
};

use super::evaluate::{material, tag_move, Eval, EvalCookie, Score};

/// Create an estimate for how good a move is.
/// `delta` is the PST difference created by this move.
/// Requires that `m` must be a legal move in `b`.
///
/// # Panics
///
/// This function may panic if the given move is illegal.
pub fn candidacy(b: &Board, m: Move, delta: Score, phase: f32) -> Eval {
    let mover_type = b.type_at_square(m.from_square()).unwrap();

    // Worst case, we don't keep the piece we captured
    let mut worst_case_delta = delta;
    let mover_value = material::value(mover_type);
    worst_case_delta -= mover_value;
    worst_case_delta.blend(phase)
}

#[derive(Clone, Debug, PartialEq)]
/// A structure which generates legal moves for searching.
pub struct MovePicker {
    /// The buffer of captures to select from, paired with their PST deltas and then their final
    /// candidacies.
    capture_buffer: Vec<(Move, (Score, Eval))>,
    /// The buffer of quiet moves to select from, paired with their PST deltas and then their final
    /// candidacies.
    quiet_buffer: Vec<(Move, (Score, Eval))>,
    /// The index in the move buffer of the capture to give (initialized to 0).
    capture_index: usize,
    /// The index in the move buffer of the quiet move to give (initialized to 0).
    quiet_index: usize,
    /// The set of moves to ignore.
    ignored: Vec<Move>,
    /// The board for which moves are being generated.
    /// In the ideal world, this would instead be an `&'a Board`, pointing to a board in game's
    /// history, but borrow checker rules prevent us from making this optimization.
    board: Board,
    /// The tagging cookie for `board`.
    /// Like `board`, this would ideally be an `&'a Cookie`.
    cookie: EvalCookie,
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
    pub fn new(
        b: Board,
        cookie: &EvalCookie,
        transposition_move: Option<Move>,
        killer_move: Option<Move>,
    ) -> MovePicker {
        MovePicker {
            capture_buffer: Vec::new(),
            quiet_buffer: Vec::new(),
            capture_index: 0,
            quiet_index: 0,
            ignored: Vec::new(),
            board: b,
            cookie: *cookie,
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
}

/// Search through `moves` until we find the best move, sorting as we go.
/// After this function terminates, `moves[idx]` will contain the best-rated move of the input moves
/// from idx to the end.
/// Requires that `0 <= idx < moves.len()`.
fn select_best(moves: &mut [(Move, (Score, Eval))], idx: usize) -> (Move, (Score, Eval)) {
    let mut best_entry = moves[idx];
    for entry in moves.iter_mut().skip(idx + 1) {
        // insertion sort to get the best move.
        // insertion sort is slower if we need to see every move,
        // but often we don't due to beta cutoff
        if entry.1 .1 > best_entry.1 .1 {
            // swap out the next-best move
            swap(entry, &mut best_entry);
        }
    }

    best_entry
}

impl Iterator for MovePicker {
    type Item = (Move, (Score, Eval));

    /// Get the next move which the move picker wants.
    fn next(&mut self) -> Option<Self::Item> {
        match self.phase {
            PickPhase::Transposition => {
                self.phase = PickPhase::PreGoodCapture;
                match self.transposition_move {
                    None => self.next(),
                    Some(m) => {
                        // we assume that m was checked for legality before
                        self.ignore(m);
                        Some((m, tag_move(m, &self.board, &self.cookie)))
                    }
                }
            }
            PickPhase::PreGoodCapture => {
                // generate moves, and then move along
                self.phase = PickPhase::GoodCapture;
                get_moves::<{ GenMode::Captures }>(&self.board, |m| {
                    self.capture_buffer
                        .push((m, tag_move(m, &self.board, &self.cookie)));
                });
                self.next()
            }
            PickPhase::GoodCapture => {
                /// The cutoff for good captures.
                ///
                /// Captures with a score below this value will be sent to the back of the list.
                const GOOD_CAPTURE_CUTOFF: Eval = Eval::centipawns(-300);

                if self.capture_index >= self.capture_buffer.len() {
                    // out of captures
                    self.phase = PickPhase::Killer;
                    return self.next();
                }
                let capture_entry = select_best(&mut self.capture_buffer, self.capture_index);
                if capture_entry.1 .1 < GOOD_CAPTURE_CUTOFF {
                    // we are now in bad captures, move on
                    self.phase = PickPhase::Killer;
                    // make sure to leave this move in place
                    self.capture_buffer[self.capture_index] = capture_entry;
                    return self.next();
                }
                // make sure to get a new capture next time
                self.capture_index += 1;
                if self.ignored.contains(&capture_entry.0) {
                    // don't bother with ignored moves
                    return self.next();
                }
                Some(capture_entry)
            }
            PickPhase::Killer => {
                self.phase = PickPhase::PreQuiet;
                match self.killer_move {
                    None => self.next(),
                    Some(m) => {
                        if is_legal(m, &self.board) {
                            self.ignore(m);
                            Some((m, tag_move(m, &self.board, &self.cookie)))
                        } else {
                            self.next()
                        }
                    }
                }
            }
            PickPhase::PreQuiet => {
                // generate quiet moves
                self.phase = PickPhase::Quiet;
                get_moves::<{ GenMode::Quiets }>(&self.board, |m| {
                    self.quiet_buffer
                        .push((m, tag_move(m, &self.board, &self.cookie)));
                });
                self.next()
            }
            PickPhase::Quiet => {
                if self.quiet_index >= self.quiet_buffer.len() {
                    // out of quiets
                    self.phase = PickPhase::BadCaptures;
                    return self.next();
                }
                let quiet_entry = select_best(&mut self.quiet_buffer, self.quiet_index);
                self.quiet_index += 1;
                if self.ignored.contains(&quiet_entry.0) {
                    // don't bother with ignored moves
                    return self.next();
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
                if self.ignored.contains(&capture_entry.0) {
                    // don't bother with ignored moves
                    return self.next();
                }
                Some(capture_entry)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.phase {
            // check the size of the moves buffer
            PickPhase::GoodCapture | PickPhase::PreQuiet => {
                let n = self.capture_buffer.len() - self.capture_index;
                let n_ignored = self.ignored.len();
                if n_ignored >= n {
                    (0, None)
                } else {
                    (n - n_ignored, None)
                }
            }
            PickPhase::Quiet | PickPhase::BadCaptures => {
                // need to get through both the quiets and the bad captures
                let n = self.capture_buffer.len() - self.capture_index + self.quiet_buffer.len()
                    - self.quiet_index;
                let n_ignored = self.ignored.len();
                if n_ignored >= n {
                    (0, Some(n))
                } else {
                    (n - n_ignored, Some(n))
                }
            }
            _ => (0, None),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{base::movegen::make_move_vec, engine::evaluate::init_cookie};

    use super::*;

    #[test]
    /// Test that all moves are generated in the move picker and that there are no duplicates.
    fn generation_correctness() {
        let b = Board::from_fen("r2q1rk1/ppp2ppp/3b4/4Pb2/4Q3/2PB4/P1P2PPP/R1B1K2R w KQ - 5 12")
            .unwrap();
        let mp = MovePicker::new(b, &init_cookie(&b), None, None);

        let picker_moves = mp.map(|(m, _)| m);
        let gen_moves = make_move_vec::<{ GenMode::All }>(&b);
        for m in picker_moves.clone() {
            assert!(gen_moves.contains(&m));
            println!("{:?}", m);
        }

        for m in gen_moves {
            println!("looking for {m} in movepicker moves");
            assert!(picker_moves.clone().any(|m2| m2 == m));
        }

        for m in picker_moves.clone() {
            println!("counting {m}");
            assert_eq!(picker_moves.clone().filter(|&m2| m2 == m).count(), 1);
        }
    }
}
