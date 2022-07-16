/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors&& (see AUTHORS.md file)

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

//! Full chess games, including history and metadata.

use crate::movegen::is_legal;

use super::{
    algebraic::algebraic_from_move,
    movegen::{get_moves, has_moves, is_square_attacked_by, GenMode},
    Board, Color, Move, Piece,
};

use nohash_hasher::IntMap;

use std::{
    default::Default,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// A struct containing game information, which unlike a `Board`, knows about
/// its history and can do things like repetition timing.
///
/// `T` is
pub struct TaggedGame<T: Tagger> {
    /// The last element in `history` is the current state of the board. The
    /// first element should be the starting position of the game, and in
    /// between are sequential board states from the entire game. The right
    /// half of the tuple is the number of moves since a pawn-move or capture
    /// was made, and should start at 0.
    history: Vec<(Board, u8, T::Cookie)>,
    /// The list, in order, of all moves made in the game. They should all be
    /// valid moves. The length of `moves` should always be one less than the
    /// length of `history`.
    moves: Vec<Move>,
    /// Stores the number of times a position has been reached in the course of
    /// this game. It is used for three-move-rule draws. The keys are the
    /// Zobrist hashes of the boards previously visited.
    repetitions: IntMap<u64, u8>,
}

pub type Game = TaggedGame<NoTag>;

#[derive(Debug, PartialEq, Eq)]
/// A tagger which will perform no tagging, allowing ergonomic use of tagged
/// games without metadata.
pub struct NoTag {}

impl Tagger for NoTag {
    type Tag = ();
    type Cookie = ();

    fn tag_move(_: Move, _: &Board) -> Self::Tag {}

    fn update_cookie(_: Move, _: &Self::Tag, _: &Board, _: &Self::Cookie) {}

    fn init_cookie(_: &Board) {}
}

pub trait Tagger {
    /// The type of the metadata with which is attached to each move.
    type Tag;
    /// The type of the metadata which is persistent on boards, and which `Tag`
    /// is used to update.
    type Cookie;

    /// Add a tag to a given move, made on board `b`.
    fn tag_move(m: Move, b: &Board) -> Self::Tag;

    /// Compute what the new cookie would be after making the move `m` on `b`.
    fn update_cookie(
        m: Move,
        tag: &Self::Tag,
        b: &Board,
        prev_cookie: &Self::Cookie,
    ) -> Self::Cookie;

    /// Initialize the cookie on a new board.
    fn init_cookie(b: &Board) -> Self::Cookie;
}

impl<T: Tagger> TaggedGame<T> {
    /// Construct a new `Game` in the conventional chess starting position. The
    /// cumulative evaluation will be initialized to zero.
    pub fn new() -> TaggedGame<T> {
        let b = Board::default();
        TaggedGame {
            history: vec![(b, 0, T::init_cookie(&b))],
            moves: Vec::new(),
            repetitions: {
                let mut map = IntMap::default();
                map.insert(Board::default().hash, 1);
                map
            },
        }
    }

    /// Con
    pub fn from_fen(fen: &str) -> Result<TaggedGame<T>, String> {
        let b = Board::from_fen(fen)?;
        // TODO extract 50 move rule from the FEN
        Ok(TaggedGame {
            history: vec![(b, 0, T::init_cookie(&b))],
            moves: Vec::new(),
            repetitions: {
                let mut map = IntMap::default();
                map.insert(b.hash, 1);
                map
            },
        })
    }

    /// Empty out the history of this game completely, but leave the original
    /// start state of the board.
    pub fn clear(&mut self) {
        self.history.truncate(1);
        let start_board = self.history[0].0;
        self.moves.clear();
        self.repetitions.clear();
        //since we cleared this, or_insert will always be called
        self.repetitions.entry(start_board.hash).or_insert(1);
    }

    /// Make a move, assuming said move is legal. If the history is empty
    /// (this should never happen if normal operations occurred), the move will
    /// be made from the default state of a `Board`. `delta` is the
    /// expected gain in evaluation for the player making the move. Typically,
    /// `delta` will be positive.
    pub fn make_move(&mut self, m: Move, tag: T::Tag) {
        #[cfg(debug_assertions)]
        if !is_legal(m, self.board()) {
            println!("an illegal move {m} is being attempted. History: {self}");
            panic!();
        }
        let previous_state = self.history.last().unwrap();
        let mut new_board = previous_state.0;

        let move_timeout = match new_board.is_move_capture(m)
            || new_board[Piece::Pawn].contains(m.from_square())
        {
            true => 0,
            false => previous_state.1 + 1,
        };
        new_board.make_move(m);
        let num_reps = self.repetitions.entry(new_board.hash).or_insert(0);
        *num_reps += 1;
        self.history.push((
            new_board,
            move_timeout,
            T::update_cookie(m, &tag, &previous_state.0, &previous_state.2),
        ));
        self.moves.push(m);
    }

    /// Attempt to play a move, which may or may not be legal. If the move is
    /// legal, the move will be executed and the state will change, then
    /// `Ok(())` will be returned. If not, an `Err` will be returned to inform
    /// you that the move is illegal, and no state will be changed.
    pub fn try_move(&mut self, m: Move, tag: T::Tag) -> Result<(), &'static str> {
        if is_legal(m, self.board()) {
            self.make_move(m, tag);
            Ok(())
        } else {
            Err("illegal move given!")
        }
    }

    /// Undo the most recent move. The return will be `Ok` if there are moves
    /// left to undo, with the internal value being the move that was undone,

    /// and `Err` if there are no moves to undo.
    pub fn undo(&mut self) -> Result<Move, &'static str> {
        let m_removed = match self.moves.pop() {
            Some(m) => m,
            None => return Err("no moves to remove"),
        };
        let b_removed = match self.history.pop() {
            Some(p) => p.0,
            None => return Err("no boards in history"),
        };
        let num_reps = self.repetitions.entry(b_removed.hash).or_insert(1);
        *num_reps -= 1;
        if *num_reps == 0 {
            self.repetitions.remove(&b_removed.hash);
        }

        Ok(m_removed)
    }

    /// Undo a set number of moves. Returns an Err if you attempt to remove too
    /// many moves (and will not undo anything if that is the case).
    pub fn undo_n(&mut self, nmoves: usize) -> Result<(), &'static str> {
        if nmoves > self.moves.len() {
            return Err("attempted to remove more moves than are in history");
        }
        for _ in 0..nmoves {
            self.undo()?;
        }
        Ok(())
    }

    #[inline(always)]
    /// Get the position representing the current state of the game. Will panic
    /// if there is no history, but this should never happen.
    pub fn board(&self) -> &Board {
        &self.history.last().unwrap().0
    }

    #[inline(always)]
    /// Get the cookie of the current state of the game.
    pub fn cookie(&self) -> &T::Cookie {
        &self.history.last().unwrap().2
    }

    /// In the current state, is the game complete (i.e. is there no way the
    /// game can continue)? The return type has the first type as whether the
    /// game is over, and the second is the player which has won if the game is
    /// over. It will be `None` for a draw.
    pub fn is_over(&self) -> (bool, Option<Color>) {
        if self.is_drawn_historically() {
            return (true, None);
        }
        let b = self.board();

        if has_moves(b) {
            return (false, None);
        }

        let king_sq = b.king_sqs[b.player as usize];
        match is_square_attacked_by(b, king_sq, !b.player) {
            true => (true, Some(!b.player)),
            false => (true, None), // stalemate
        }
    }

    /// Has this game been drawn due to its move history (i.e. due to the 50
    /// move rule or due to repetition)?
    pub fn is_drawn_historically(&self) -> bool {
        let num_reps = *self.repetitions.get(&self.board().hash).unwrap_or(&0);
        if num_reps >= 3 {
            // draw by repetition
            return true;
        }

        if self.history.last().unwrap().1 >= 100 {
            // 50 moves = 100 ply
            // draw by 50 move rule
            return true;
        }
        false
    }

    /// Get the legal moves in this position. Will be empty if the position is
    /// drawn or the game is over.
    pub fn get_moves<const M: GenMode>(&self) -> Vec<(Move, T::Tag)> {
        if self.is_drawn_historically() {
            return Vec::new();
        }

        get_moves::<M, T>(self.board())
    }

    // no need for `is_empty` since history should always be nonempty
    #[allow(clippy::len_without_is_empty)]
    /// Get the number of total positions in this history of this game.
    pub fn len(&self) -> usize {
        self.history.len()
    }
}

impl<T: Tagger> Default for TaggedGame<T> {
    fn default() -> Self {
        TaggedGame::new()
    }
}

impl<T: Tagger> Display for TaggedGame<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.moves.len() {
            let board: &Board = &self.history[i].0;
            let m = self.moves[i];
            write!(f, "{} ", algebraic_from_move(m, board))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{movegen::ALL, Board, Move, Square};

    #[test]
    /// Test that we can play a simple move on a `Game` and have the board
    /// states update accordingly.
    fn play_e4() {
        let mut g = Game::new();
        let m = Move::normal(Square::E2, Square::E4);
        let old_board = *g.board();
        g.make_move(Move::normal(Square::E2, Square::E4), ());
        let new_board = g.board();
        crate::board::tests::move_result_helper(old_board, *new_board, m);
    }

    #[test]
    /// Test that a single move can be undone correctly.
    fn undo_move() {
        let mut g = Game::new();
        let m = Move::normal(Square::E2, Square::E4);
        g.make_move(m, ());
        assert_eq!(g.undo(), Ok(m));
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that an undo will fail if there is no history to undo.
    fn illegal_undo() {
        let mut g = Game::new();
        assert!(g.undo().is_err());
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that we can undo multiple moves in a row.
    fn undo_multiple_moves() {
        let mut g = Game::new();
        let m0 = Move::normal(Square::E2, Square::E4);
        let m1 = Move::normal(Square::E7, Square::E5);
        g.make_move(m0, ());
        g.make_move(m1, ());
        assert_eq!(g.undo_n(2), Ok(()));
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that a `Game` becomes exactly the same as what it started as if a
    /// move is undone.
    fn undo_equality() {
        let mut g = Game::new();
        g.make_move(Move::normal(Square::E2, Square::E4), ());
        assert!(g.undo().is_ok());
        assert_eq!(g, Game::new());
    }

    #[test]
    /// Test that undoing a move results in the previous position.
    fn undo_fried_liver() {
        // the fried liver FEN
        let fen = "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7";
        let mut g = Game::from_fen(fen).unwrap();
        let m = Move::normal(Square::D1, Square::F3);
        g.make_move(m, ());
        assert_eq!(g.undo(), Ok(m));
        assert_eq!(g, Game::from_fen(fen).unwrap());
        assert_eq!(g.board(), &Board::from_fen(fen).unwrap());
    }

    #[test]
    /// Test that undoing with no history results in an error.
    fn undo_fail() {
        let mut g = Game::new();
        assert!(g.undo().is_err());
    }

    #[test]
    /// Test that a mated position is in fact over.
    fn is_mate_over() {
        // the position from the end of Scholar's mate
        let g = Game::from_fen("rnbqk2r/pppp1Qpp/5n2/2b1p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4")
            .unwrap();
        let moves = g.get_moves::<ALL>();
        assert!(moves.is_empty());
        assert!(!has_moves(g.board()));
        assert_eq!(g.is_over(), (true, Some(Color::White)));
    }

    #[test]
    fn is_mate_over_2() {
        let g =
            Game::from_fen("r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11").unwrap();
        let moves = g.get_moves::<ALL>();
        assert!(moves.is_empty());
        assert!(!has_moves(g.board()));
        assert_eq!(g.is_over(), (true, Some(Color::Black)));
    }

    #[test]
    fn startpos_not_over() {
        assert!(!Game::default().is_over().0)
    }

    #[test]
    /// Test that making a mate found in testing results in the game being over.
    fn mate_in_1() {
        // Rb8# is the winning move
        let mut g = Game::from_fen("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1").unwrap();
        let m = Move::normal(Square::B6, Square::B8);
        assert!(g.get_moves::<ALL>().contains(&(m, ())));
        g.make_move(m, ());
        assert_eq!(g.is_over(), (true, Some(Color::White)));
    }

    #[test]
    /// Test that clearing a board has the same effect of replacing it with a
    /// default board, if the initial state was the initial board state.
    fn clear_board() {
        let mut g = Game::new();
        g.make_move(Move::normal(Square::E2, Square::E4), ());
        g.clear();
        assert_eq!(g, Game::new());
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn king_escape_without_capture() {
        let g = Game::from_fen("r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10")
            .unwrap();
        let moves = g.get_moves::<ALL>();
        let expected_moves = vec![
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in moves.iter() {
            assert!(expected_moves.contains(&m.0));
        }
        for em in expected_moves.iter() {
            assert!(moves.contains(&(*em, ())));
        }
    }
}
