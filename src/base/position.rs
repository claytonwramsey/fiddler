use super::{Board, movegen::CheckInfo, Eval, Move};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub board: Board,
    pub check_info: CheckInfo,
    pub pst_val: (Eval, Eval),
}

impl Position {
    /// Construct a position from a FEN. The resulting PST value will be 
    /// uninitialized, so it must be later updated accordingly.
    pub fn from_fen(fen: &str) -> Result<Position, &str> {
        let board = Board::from_fen(fen)?;
        Ok(Position {
            board,
            check_info: CheckInfo::about(&board),
            pst_val: (Eval::DRAW, Eval::DRAW),
        })
    }

    #[inline]
    /// Make a move on this position, updating the check info and PST values as 
    /// needed.
    pub fn make_move(&mut self, m: Move, pst_delta: (Eval, Eval)) {
        self.board.make_move(m);
        self.check_info = CheckInfo::about(&self.board);
        self.pst_val = (
            self.pst_val.0 + pst_delta.0,
            self.pst_val.1 + pst_delta.1
        );
    }
}