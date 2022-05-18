use std::convert::TryFrom;

use super::{
    movegen::{get_moves, CheckInfo},
    Board, Color, Eval, Move, Piece, Score, Square,
};

/// A function which can get the PST value of a position.
pub type PSTEvaluator = fn(&Board) -> Score;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A structure describing one board, plus useful metadata about that board.
pub struct Position {
    /// The board which the position has metadata about.
    pub board: Board,
    /// Check information about `board`.
    pub check_info: CheckInfo,
    /// The location of the White and Black kings, respectively.
    pub king_sqs: [Square; 2],
    /// The PST evaluation for the middlegame and endgame, respectively.
    pub pst_val: Score,
}

impl Position {
    pub const NO_DELTA: Score = (Eval::DRAW, Eval::DRAW);

    /// Construct a position from a FEN.
    pub fn from_fen(fen: &str, pst_evaluator: PSTEvaluator) -> Result<Position, String> {
        let board = Board::from_fen(fen)?;
        Ok(Position {
            board,
            check_info: CheckInfo::about(&board),
            king_sqs: [
                Square::try_from(board[Piece::King] & board[Color::White]).unwrap(),
                Square::try_from(board[Piece::King] & board[Color::Black]).unwrap(),
            ],
            pst_val: pst_evaluator(&board),
        })
    }

    /// Helper function for initializing boards if you do not care about the
    /// PST value of a board.
    pub fn no_eval(_: &Board) -> Score {
        (Eval::DRAW, Eval::DRAW)
    }

    #[inline]
    /// Make a move on this position, updating the check info and PST values as
    /// needed. `pst_delta` is the expected gain in PST evaluation that will
    /// occur from this move. It will be higher for moves which are better for
    /// the player.
    pub fn make_move(&mut self, m: Move, pst_delta: Score) {
        // reduce evaluation for goot moves for Black
        match self.board.player_to_move {
            Color::White => {
                self.pst_val = (self.pst_val.0 + pst_delta.0, self.pst_val.1 + pst_delta.1)
            }
            Color::Black => {
                self.pst_val = (self.pst_val.0 - pst_delta.0, self.pst_val.1 - pst_delta.1)
            }
        }
        if m.from_square() == self.king_sqs[self.board.player_to_move as usize] {
            // update king locations
            self.king_sqs[self.board.player_to_move as usize] = m.to_square();
        }
        self.board.make_move(m);
        self.check_info = CheckInfo::about(&self.board);
    }

    /// Apply the given move to the board. Will *not* assume the move is legal
    /// (unlike `make_move()`). On illegal moves, will return an `Err` with a
    /// string describing the issue.
    pub fn try_move(&mut self, m: Move, pst_delta: Score) -> Result<(), &str> {
        let legal_moves = get_moves(self);
        if !legal_moves.contains(&m) {
            return Err("not contained in the set of legal moves");
        }

        self.make_move(m, pst_delta);
        Ok(())
    }
}

impl Default for Position {
    fn default() -> Position {
        let b = Board::default();
        Position {
            board: b,
            check_info: CheckInfo::about(&b),
            king_sqs: [
                Square::try_from(b[Piece::King] & b[Color::White]).unwrap(),
                Square::try_from(b[Piece::King] & b[Color::Black]).unwrap(),
            ],
            pst_val: (Eval::DRAW, Eval::DRAW),
        }
    }
}
