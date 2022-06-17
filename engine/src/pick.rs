use std::mem::swap;

use fiddler_base::{
    movegen::{get_moves, is_legal, CAPTURES, QUIETS},
    Eval, Move, Position, Score,
};

use crate::candidacy::PstNominate;

use super::pst::pst_delta;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePicker {
    /// The buffer of captures to select from, paired with their PST deltas and
    /// then their final candidacies.
    capture_buffer: Vec<(Move, (Score, Eval))>,
    /// The buffer of quiet moves to select from, paired with their PST deltas
    /// and then their final candidacies.
    quiet_buffer: Vec<(Move, (Score, Eval))>,
    /// The index in the move buffer of the capture to give
    /// (initialized to 0).
    capture_index: usize,
    /// The index of t
    quiet_index: usize,
    /// The set of moves to ignore.
    ignored: Vec<Move>,
    /// The game for which moves are being generated
    pos: Position,
    /// The upcoming phase of move generation.
    phase: PickPhase,
    /// The move retreived from the transposition table.
    transposition_move: Option<Move>,
    /// The killer move.
    killer_move: Option<Move>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// The current phase of move selection.
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
    /// Construct a `MovePicker` for a given position. Will generate moves, so
    /// it should only be created at a point in the search where moves must be
    /// generated.
    pub fn new(
        pos: Position,
        transposition_move: Option<Move>,
        killer_move: Option<Move>,
    ) -> MovePicker {
        MovePicker {
            capture_buffer: Vec::new(),
            quiet_buffer: Vec::new(),
            capture_index: 0,
            quiet_index: 0,
            ignored: Vec::new(),
            pos,
            phase: PickPhase::Transposition,
            transposition_move,
            killer_move,
        }
    }

    /// Add a move to the set of moves that should be ignored. Requires that
    /// `m` is not `Move::BAD_MOVE`.
    fn ignore(&mut self, m: Move) {
        if !self.ignored.contains(&m) {
            self.ignored.push(m);
        }
    }
}

/// Search through `moves` until we find the best move, sorting as we go. After
/// this function terminates, `moves[idx]` will contain the best-rated move of
/// the input moves from idx to the end. Requires that 0 <= `idx` <
/// `moves.len()`.
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
    type Item = (Move, Score);

    /// Get the next move which the move picker wants.
    fn next(&mut self) -> Option<Self::Item> {
        match self.phase {
            PickPhase::Transposition => {
                self.phase = PickPhase::PreGoodCapture;
                match self.transposition_move {
                    None => self.next(),
                    Some(m) => {
                        self.ignore(m);
                        Some((m, pst_delta(&self.pos.board, m)))
                    }
                }
            }
            PickPhase::PreGoodCapture => {
                // generate moves, and then move along
                self.phase = PickPhase::GoodCapture;
                self.capture_buffer = get_moves::<CAPTURES, PstNominate>(&self.pos);
                self.next()
            }
            PickPhase::GoodCapture => {
                if self.capture_index >= self.capture_buffer.len() {
                    // out of captures
                    self.phase = PickPhase::Killer;
                    return self.next();
                }
                let capture_entry = select_best(&mut self.capture_buffer, self.capture_index);
                if capture_entry.1 .1 < Eval::DRAW {
                    // we are now in bad captures, move on
                    self.phase = PickPhase::PreQuiet;
                    return self.next();
                }
                // make sure to get a new capture next time
                self.capture_index += 1;
                if self.ignored.contains(&capture_entry.0) {
                    // don't bother with ignored moves
                    return self.next();
                }
                Some((capture_entry.0, capture_entry.1 .0))
            }
            PickPhase::Killer => {
                self.phase = PickPhase::PreQuiet;
                match self.killer_move {
                    None => self.next(),
                    Some(m) => match is_legal(m, &self.pos) {
                        true => {
                            self.ignore(m);
                            Some((m, pst_delta(&self.pos.board, m)))
                        }
                        false => self.next(),
                    },
                }
            }
            PickPhase::PreQuiet => {
                // generate quiet moves
                self.phase = PickPhase::Quiet;
                self.quiet_buffer = get_moves::<QUIETS, PstNominate>(&self.pos);
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
                Some((quiet_entry.0, quiet_entry.1 .0))
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
                Some((capture_entry.0, capture_entry.1 .0))
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
