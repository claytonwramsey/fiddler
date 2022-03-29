use crate::base::{movegen::get_moves, Eval, Game, Move};

use super::{candidacy::candidacy, pst::pst_delta};

pub struct MovePicker<'a> {
    /// The buffer of moves to select from, paired with their PST deltas and
    /// then their final candidacies.
    move_buffer: Vec<((Move, (Eval, Eval)), Eval)>,
    /// The index in the move buffer of the next move to give
    /// (initialized to 0).
    index: usize,
    ignored: &'a [Move],
}

impl<'a> MovePicker<'a> {
    /// Construct a `MovePicker` for a given position. Will generate moves, so
    /// it should only be created at a point in the search where moves must be
    /// generated.
    pub fn new(g: &Game, ignored_moves: &'a [Move]) -> MovePicker<'a> {
        MovePicker {
            move_buffer: get_moves(g.position())
                .iter()
                .map(|&m| {
                    let delta = pst_delta(g.board(), m);
                    ((m, delta), candidacy(g, m, delta))
                })
                .collect(),
            index: 0,
            ignored: ignored_moves,
        }
    }
}

impl<'a> Iterator for MovePicker<'a> {
    type Item = (Move, (Eval, Eval));

    /// Get the next move which the move picker wants.
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.move_buffer.len() {
            // TODO phased move generation here
            return None;
        }
        let (mut best, mut best_eval) = self.move_buffer[self.index];
        for i in (self.index + 1)..self.move_buffer.len() {
            // insertion sort to get the best move.
            // insertion sort is slower if we need to see every move, but often
            // we don't due to beta cutoff
            let (mp, ev) = self.move_buffer[i];
            if ev > best_eval {
                // swap out the next-best move
                self.move_buffer[i] = (best, best_eval);
                (best, best_eval) = (mp, ev);
            }
        }
        for &ignored in self.ignored {
            // if we encounter an ignored move, skip our best move and move on.
            if ignored == best.0 {
                self.index += 1;
                return self.next();
            }
        }
        self.index += 1;
        Some(best)
    }
}
