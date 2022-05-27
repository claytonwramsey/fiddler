use fiddler_base::{
    movegen::{get_moves, is_legal},
    Eval, Move, Position, Score,
};

use crate::pst::PstNominate;

use super::pst::pst_delta;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePicker {
    /// The buffer of moves to select from, paired with their PST deltas and
    /// then their final candidacies.
    move_buffer: Vec<(Move, (Score, Eval))>,
    /// The index in the move buffer of the next move to give
    /// (initialized to 0).
    index: usize,
    /// The set of moves to ignore.
    ignored: Vec<Move>,
    /// The game for which moves are being generated
    pos: Position,
    /// The upcoming phase of move generation.
    phase: PickPhase,
    /// The move retreived from the transposition table.
    transposition_move: Move,
    /// The killer move.
    killer_move: Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// The current phase of move selection.
enum PickPhase {
    /// Pick the move from the transposition table next.
    Transposition,
    /// Pick the killer move next.
    Killer,
    /// Prior to picking general moves, so more moves must be generated.
    PreGeneral,
    /// Pick general moves, which have already been generated
    General,
}

impl MovePicker {
    /// Construct a `MovePicker` for a given position. Will generate moves, so
    /// it should only be created at a point in the search where moves must be
    /// generated.
    pub fn new(pos: Position, transposition_move: Move, killer_move: Move) -> MovePicker {
        MovePicker {
            move_buffer: Vec::new(),
            index: 0,
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

impl Iterator for MovePicker {
    type Item = (Move, Score);

    /// Get the next move which the move picker wants.
    fn next(&mut self) -> Option<Self::Item> {
        match self.phase {
            PickPhase::Transposition => {
                self.phase = PickPhase::Killer;
                match self.transposition_move {
                    Move::BAD_MOVE => self.next(),
                    m => {
                        self.ignore(m);
                        Some((m, pst_delta(&self.pos.board, m)))
                    }
                }
            }
            PickPhase::Killer => {
                self.phase = PickPhase::PreGeneral;
                match self.killer_move {
                    Move::BAD_MOVE => self.next(),
                    m => match is_legal(m, &self.pos) {
                        true => {
                            self.ignore(m);
                            Some((m, pst_delta(&self.pos.board, m)))
                        }
                        false => self.next(),
                    },
                }
            }
            PickPhase::PreGeneral => {
                // generate moves, and then move along
                self.phase = PickPhase::General;
                self.move_buffer = get_moves::<PstNominate>(&self.pos);
                self.next()
            }
            PickPhase::General => {
                if self.index >= self.move_buffer.len() {
                    // TODO phased move generation here
                    return None;
                }
                let (mut best, mut best_eval) = self.move_buffer[self.index];
                for i in (self.index + 1)..self.move_buffer.len() {
                    // insertion sort to get the best move.
                    // insertion sort is slower if we need to see every move,
                    // but often we don't due to beta cutoff
                    let (mp, ev) = self.move_buffer[i];
                    if ev > best_eval {
                        // swap out the next-best move
                        self.move_buffer[i] = (best, best_eval);
                        (best, best_eval) = (mp, ev);
                    }
                }
                for &ignored in self.ignored.iter() {
                    // if we encounter an ignored move, skip our best move and
                    // move on.
                    if ignored == best {
                        self.index += 1;
                        return self.next();
                    }
                }
                self.index += 1;
                Some((best, best_eval.0))
            }
        }
    }
}
