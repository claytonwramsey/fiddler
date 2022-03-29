use crate::base::{movegen::get_moves, Eval, Game, Move};

use super::{candidacy::candidacy, pst::pst_delta};

pub struct MovePicker {
    /// The buffer of moves to select from, paired with their candidacies.
    move_buffer: Vec<(Move, Eval)>,
    /// The index in the move buffer of the next move to give
    /// (initialized to 0).
    index: usize,
}

impl MovePicker {
    /// Construct a `MovePicker` for a given position. Will generate moves, so
    /// it should only be created at a point in the search where moves must be
    /// generated.
    pub fn new(g: &Game) -> MovePicker {
        MovePicker {
            move_buffer: get_moves(g.position())
                .iter()
                .map(|&m| (m, candidacy(g, m, pst_delta(g.board(), m))))
                .collect(),
            index: 0,
        }
    }
}

impl Iterator for MovePicker {
    type Item = Move;

    /// Get the next move which the move picker wants.
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.move_buffer.len() {
            // TODO phased move generation here
            return None;
        }
        let (mut best_move, mut best_eval) = self.move_buffer[self.index];
        for i in (self.index + 1)..self.move_buffer.len() {
            // insertion sort to get the best move.
            // insertion sort is slower if we need to see every move, but often
            // we don't due to beta cutoff
            let (m, ev) = self.move_buffer[i];
            if ev > best_eval {
                // swap out the next-best move
                self.move_buffer[i] = (best_move, best_eval);
                (best_move, best_eval) = (m, ev);
            }
        }
        self.index += 1;
        Some(best_move)
    }
}
