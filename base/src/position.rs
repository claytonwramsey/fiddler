/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! Positions, which contain both boards and metadata about said boards.

use std::convert::TryFrom;

use crate::movegen::{NoopNominator, ALL};

use super::{
    movegen::{get_moves, CheckInfo},
    Board, Color, Move, Piece, Score, Square,
};

/// A function which can get the PST value of a position.
pub type PSTEvaluator = fn(&Board) -> Score;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A structure describing one board, plus useful metadata about that board.
///
/// TODO: genericise this structure to allow for differing amounts of metadata.
pub struct Position {
    /// The board which the position has metadata about.
    pub board: Board,
    /// Check information about `board`.
    pub check_info: CheckInfo,
    /// The location of the White and Black kings, respectively.
    pub king_sqs: [Square; 2],
    /// The PST evaluation for the middlegame and endgame, respectively.
    pub score: Score,
}

impl Position {
    /// An evaluation delta which causes no change.
    pub const NO_DELTA: Score = Score::DRAW;

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
            score: pst_evaluator(&board),
        })
    }

    /// Helper function for initializing boards if you do not care about the
    /// PST value of a board.
    pub const fn no_eval(_: &Board) -> Score {
        Score::DRAW
    }

    #[inline(always)]
    /// Make a move on this position, updating the check info and PST values as
    /// needed. `delta` is the expected gain in PST evaluation that will
    /// occur from this move. It will be higher for moves which are better for
    /// the player.
    pub fn make_move(&mut self, m: Move, delta: Score) {
        // reduce evaluation for goot moves for Black
        match self.board.player_to_move {
            Color::White => self.score += delta,
            Color::Black => self.score -= delta,
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
    pub fn try_move(&mut self, m: Move, delta: Score) -> Result<(), &str> {
        let legal_moves = get_moves::<ALL, NoopNominator>(self);
        if !legal_moves.contains(&(m, ())) {
            return Err("not contained in the set of legal moves");
        }

        self.make_move(m, delta);
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
            score: Score::DRAW,
        }
    }
}
