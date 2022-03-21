use super::{movegen::CheckInfo, Board, Color, Eval, Move};

/// A function which can get the PST value of a position.
pub type PSTEvaluator = fn(&Board) -> (Eval, Eval);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub board: Board,
    pub check_info: CheckInfo,
    pub pst_val: (Eval, Eval),
}

impl Position {
    /// Construct a position from a FEN.
    pub fn from_fen(fen: &str, pst_evaluator: PSTEvaluator) -> Result<Position, String> {
        let board = Board::from_fen(fen)?;
        Ok(Position {
            board,
            check_info: CheckInfo::about(&board),
            pst_val: pst_evaluator(&board),
        })
    }

    #[inline]
    /// Make a move on this position, updating the check info and PST values as
    /// needed. `pst_delta` is the expected gain in PST evaluation that will
    /// occur from this move. It will be higher for moves which are better for
    /// the player.
    pub fn make_move(&mut self, m: Move, pst_delta: (Eval, Eval)) {
        self.check_info = CheckInfo::about(&self.board);
        // reduce evaluation for goot moves for Black
        match self.board.player_to_move {
            Color::White => {
                self.pst_val = (self.pst_val.0 + pst_delta.0, self.pst_val.1 + pst_delta.1)
            }
            Color::Black => {
                self.pst_val = (self.pst_val.0 - pst_delta.0, self.pst_val.1 - pst_delta.1)
            }
        }
        self.board.make_move(m);
    }
}

impl Default for Position {
    fn default() -> Position {
        let b = Board::default();
        Position {
            board: b,
            check_info: CheckInfo::about(&b),
            pst_val: (Eval::DRAW, Eval::DRAW),
        }
    }
}
