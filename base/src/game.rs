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
    movegen::{get_moves, has_moves, GenMode},
    Board, Move,
};

use nohash_hasher::IntMap;

use std::{
    default::Default,
    fmt::{Display, Formatter},
};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, PartialEq)]
/// A struct containing game information, which unlike a `Board`, knows about
/// its history and can do things like repetition timing.
///
/// `T` is a *tagger*, which will apply tags to moves.
/// `T` also uses a cookie to annotate boards.
/// This allows consumers of `TaggedGame`s to annotate boards and moves
/// efficiently, saving on allocations.
pub struct TaggedGame<T: Tagger> {
    /// The last element in `history` is the current state of the board.
    /// The first element should be the starting position of the game, and in
    /// between are sequential board states from the entire game.
    /// The right half of the tuple is the number of moves since a pawn-move or
    /// capture was made, and should start at 0.
    history: Vec<(Board, T::Cookie)>,
    /// The list, in order, of all moves made in the game.
    /// They should all be valid moves.
    /// The length of `moves` should always be one less than the length of
    /// `history`.
    moves: Vec<Move>,
    /// Stores the number of times a position has been reached in the course of
    /// this game.
    /// It is used for three-move-rule draws.
    /// The keys are the Zobrist hashes of the boards previously visited.
    ///
    /// The values are a tuple of two integers: the first is the total number of
    /// repetitions, and the second is the number of repetitions since the last
    /// search start.
    repetitions: IntMap<u64, (u8, u8)>,
    /// Whether this game is currently part of a search.
    /// If it is, then the search-level repetitions will be incremented and
    /// result in draws.
    searching: bool,
}

pub type Game = TaggedGame<NoTag>;

#[derive(Debug, PartialEq, Eq)]
/// A tagger which will perform no tagging, allowing ergonomic use of tagged
/// games without metadata.
pub struct NoTag {}

impl Tagger for NoTag {
    type Tag = ();
    type Cookie = ();

    fn tag_move(_: Move, _: &Board, _: &Self::Cookie) -> Self::Tag {}

    fn update_cookie(_: Move, _: &Self::Tag, _: &Board, _: &Board, _: &Self::Cookie) {}

    fn init_cookie(_: &Board) {}
}

pub trait Tagger {
    /// The type of the metadata with which is attached to each move.
    type Tag;
    /// The type of the metadata which is persistent on boards, and which `Tag`
    /// is used to update.
    type Cookie;

    /// Add a tag to a given move, made on board `b`.
    fn tag_move(m: Move, b: &Board, cookie: &Self::Cookie) -> Self::Tag;

    /// Compute what the new cookie would be after making the move `m` on `b`.
    /// `b_after` is the resulting board after `m` is made on `b`.
    fn update_cookie(
        m: Move,
        tag: &Self::Tag,
        b: &Board,
        b_after: &Board,
        prev_cookie: &Self::Cookie,
    ) -> Self::Cookie;

    /// Initialize the cookie on a new board.
    fn init_cookie(b: &Board) -> Self::Cookie;
}

impl<T: Tagger> TaggedGame<T> {
    #[must_use]
    /// Construct a new `Game` in the conventional chess starting position. The
    /// cumulative evaluation will be initialized to zero.
    pub fn new() -> TaggedGame<T> {
        let b = Board::default();
        TaggedGame {
            history: vec![(b, T::init_cookie(&b))],
            moves: Vec::new(),
            repetitions: IntMap::from_iter([(b.hash, (1, 0))]),
            searching: false,
        }
    }

    /// Construct a new `TaggedGame` using the Forsyth-Edwards notation
    /// description of its position.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if the FEN string is invalid.
    pub fn from_fen(fen: &str) -> Result<TaggedGame<T>, String> {
        let b = Board::from_fen(fen)?;
        // TODO extract 50 move rule from the FEN
        Ok(TaggedGame {
            history: vec![(b, T::init_cookie(&b))],
            moves: Vec::new(),
            repetitions: IntMap::from_iter([(b.hash, (1, 0))]),
            searching: false,
        })
    }

    /// Empty out the history of this game completely, but leave the original
    /// start state of the board.
    /// Will also end the searching period for the game.
    pub fn clear(&mut self) {
        self.history.truncate(1);
        let start_board = self.history[0].0;
        self.moves.clear();
        self.repetitions.clear();
        // since we cleared this, or_insert will always be called
        self.repetitions.insert(start_board.hash, (1, 0));
        self.searching = false;
    }

    /// Make a move, assuming said move is legal. If the history is empty
    /// (this should never happen if normal operations occurred), the move will
    /// be made from the default state of a `Board`. `delta` is the
    /// expected gain in evaluation for the player making the move. Typically,
    /// `delta` will be positive.
    ///
    /// # Panics
    ///
    /// This function may panic if `m` is not a legal move.
    /// However, it is not guaranteed to.
    /// It is recommended to only call `make_move` with moves that were already
    /// validated.
    pub fn make_move(&mut self, m: Move, tag: &T::Tag) {
        /*
        #[cfg(debug_assertions)]
        if !is_legal(m, self.board()) {
            println!("an illegal move {m} is being attempted. History: {self}");
            panic!();
        }
        */
        let previous_state = self.history.last().unwrap();
        let mut new_board = previous_state.0;

        new_board.make_move(m);
        let num_reps = self.repetitions.entry(new_board.hash).or_insert((0, 0));
        num_reps.0 += 1;
        if self.searching {
            num_reps.1 += 1;
        }
        self.history.push((
            new_board,
            T::update_cookie(m, tag, &previous_state.0, &new_board, &previous_state.1),
        ));
        self.moves.push(m);
    }

    /// Attempt to play a move, which may or may not be legal. Will return
    /// `Ok(())` if `m` was a legal move.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` describing the source of the problem
    /// if `m` is illegal.
    pub fn try_move(&mut self, m: Move, tag: &T::Tag) -> Result<(), &str> {
        if is_legal(m, self.board()) {
            self.make_move(m, tag);
            Ok(())
        } else {
            Err("illegal move given!")
        }
    }

    /// Undo the most recent move. This function will return `Ok()` if there was
    /// history to undo.
    /// The move inside the `Ok` variant will be the most recent move played.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if the history of this game has no
    /// more positions left to undo.
    pub fn undo(&mut self) -> Result<Move, &'static str> {
        let m_removed = self.moves.pop().ok_or("no moves to remove")?;
        let b_removed = self.history.pop().ok_or("no boards in history")?.0;
        let num_reps = self
            .repetitions
            .entry(b_removed.hash)
            .or_insert((1, u8::from(self.searching)));
        num_reps.0 -= 1;
        if self.searching && num_reps.1 > 0 {
            num_reps.1 -= 1;
        }
        if num_reps.0 == 0 {
            self.repetitions.remove(&b_removed.hash);
        }

        Ok(m_removed)
    }

    #[inline(always)]
    #[must_use]
    /// Get the position representing the current state of the game.
    ///
    /// # Panics
    ///
    /// This function might panic due to an internal error eliminating all
    /// history from the internal board. However, this is very unlikely.
    pub fn board(&self) -> &Board {
        &self.history.last().unwrap().0
    }

    #[inline(always)]
    #[must_use]
    /// Get the cookie of the current state of the game.
    ///
    /// # Panics
    ///
    /// This function might panic due to an internal error eliminating all
    /// history from the internal board. However, this is very unlikely.
    pub fn cookie(&self) -> &T::Cookie {
        &self.history.last().unwrap().1
    }

    #[must_use]
    /// Detect how the game has ended.
    /// There are three possible return values:
    ///
    /// * `None`: the game is not over.
    /// * `Some(false)`: the game is over and is drawn.
    /// * `Some(true)`: the game is over by checkmate.
    pub fn end_state(&self) -> Option<bool> {
        let b = self.board();
        if self.drawn_by_repetition() || b.is_drawn() {
            return Some(false);
        }

        if has_moves(b) {
            return None;
        }

        Some(!b.checkers.is_empty())
    }

    #[must_use]
    /// Has this game been drawn due to history (i.e. repetition or the 50 move
    /// rule)?
    pub fn drawn_by_repetition(&self) -> bool {
        let num_reps = self.repetitions.get(&self.board().hash).unwrap_or(&(0, 0));
        if num_reps.0 >= 3 || num_reps.1 >= 2 {
            // draw by repetition
            return true;
        }

        false
    }

    #[must_use]
    /// Get the legal moves in this position. Will be empty if the position is
    /// drawn or the game is over.
    pub fn get_moves<const M: GenMode>(&self) -> Vec<(Move, T::Tag)> {
        if self.drawn_by_repetition() {
            return Vec::new();
        }

        get_moves::<M, T>(self.board(), self.cookie())
    }

    // no need for `is_empty` since history should always be nonempty
    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    /// Get the number of total positions in this history of this game.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Begin searching.
    /// During a search, repetitions of positions that were seen as the search
    /// went on will be immediately marked as draws.
    /// The current position of the board will also be marked as a possible
    /// repetition.
    ///
    /// # Examples
    ///
    /// `g1` is not over because a position must be reached 3 times to reach a
    /// draw.
    /// However, `g2` is over because it repeated the same position twice in a
    /// search.
    ///
    /// ```
    /// use fiddler_base::{game::Game, Move, Square};
    ///
    /// let mut g1 = Game::new();
    /// let mut g2 = Game::new();
    /// g2.start_search();
    ///
    /// let moves = [
    ///     Move::normal(Square::B1, Square::C3),
    ///     Move::normal(Square::B8, Square::C6),
    ///     Move::normal(Square::C3, Square::B1),
    ///     Move::normal(Square::C6, Square::B8),
    /// ];
    ///
    /// for m in moves {
    ///     g1.make_move(m, &());
    ///     g2.make_move(m, &());
    /// }
    ///
    /// assert!(!g1.end_state().is_some());
    /// assert!(g2.end_state().is_some());
    /// ```
    pub fn start_search(&mut self) {
        self.searching = true;
        for val in self.repetitions.values_mut() {
            val.1 = 0;
        }
        // mark the current position as visited during search
        self.repetitions
            .entry(self.board().hash)
            .and_modify(|v| v.1 = 1);
    }

    pub fn stop_search(&mut self) {
        self.searching = false;
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
            let board = &self.history[i].0;
            let m = self.moves[i];
            write!(f, "{} ", m.to_algebraic(board).unwrap())?;
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
        let mut old_board = *g.board();
        g.make_move(m, &());
        let new_board = g.board();

        old_board.make_move(m);
        assert_eq!(old_board, *new_board);
    }

    #[test]
    /// Test that a single move can be undone correctly.
    fn undo_move() {
        let mut g = Game::new();
        let m = Move::normal(Square::E2, Square::E4);
        g.make_move(m, &());
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
        g.make_move(m0, &());
        g.make_move(m1, &());
        g.undo().unwrap();
        g.undo().unwrap();
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that a `Game` becomes exactly the same as what it started as if a
    /// move is undone.
    fn undo_equality() {
        let mut g = Game::new();
        g.make_move(Move::normal(Square::E2, Square::E4), &());
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
        g.make_move(m, &());
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
        assert_eq!(g.end_state(), Some(true));
    }

    #[test]
    fn is_mate_over_2() {
        let g =
            Game::from_fen("r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11").unwrap();
        let moves = g.get_moves::<ALL>();
        assert!(moves.is_empty());
        assert!(!has_moves(g.board()));
        assert_eq!(g.end_state(), Some(true));
    }

    #[test]
    fn startpos_not_over() {
        assert!(Game::default().end_state().is_none());
    }

    #[test]
    /// Test that making a mate found in testing results in the game being over.
    fn mate_in_1() {
        // Rb8# is the winning move
        let mut g = Game::from_fen("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1").unwrap();
        let m = Move::normal(Square::B6, Square::B8);
        assert!(g.get_moves::<ALL>().contains(&(m, ())));
        g.make_move(m, &());
        assert_eq!(g.end_state(), Some(true));
    }

    #[test]
    /// Test that clearing a board has the same effect of replacing it with a
    /// default board, if the initial state was the initial board state.
    fn clear_board() {
        let mut g = Game::new();
        g.make_move(Move::normal(Square::E2, Square::E4), &());
        g.clear();
        assert_eq!(g, Game::new());
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn king_escape_without_capture() {
        let g = Game::from_fen("r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10")
            .unwrap();
        let moves = g.get_moves::<ALL>();
        let expected_moves = [
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in &moves {
            assert!(expected_moves.contains(&m.0));
        }
        for em in &expected_moves {
            assert!(moves.contains(&(*em, ())));
        }
    }
}
