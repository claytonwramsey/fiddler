use super::{Board, movegen::CheckInfo, Eval, Move};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub board: Board,
    pub check_info: CheckInfo,
    pub pst_val: (Eval, Eval),
}

impl Position {
    /// 
    pub fn from_fen(fen: &str) -> Result<Position, &str> {
        let board = Board::from_fen(fen)?;
        Ok(Position {
            board,
            check_info: CheckInfo::about(&b),
            pst_val: todo!("refactor to use PST"),
        })
    }

    #[inline]
    /// Make a move on this position, updating 
    pub fn make_move(&mut self, m: Move) {
        self.board.make_move(m);
        self.check_info = CheckInfo::about(&self.board);
        todo!("refactor to use PST");
    }
}