use crate::base::algebraic::algebraic_from_move;
use crate::base::Board;
use crate::base::Color;
use crate::base::Move;
use crate::base::Piece;
use crate::base::Square;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::default::Default;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
/// A struct containing game information, which unlike a `Board`, knows about
/// its history and can do things like repetition timing.
pub struct Game {
    /// The last element in `history` is the current state of the board. The
    /// first element should be the starting position of the game, and in
    /// between are sequential board states from the entire game. The right
    /// half of the tuple is the number of moves since a pawn-move or capture
    /// was made, and should start at 0.
    history: Vec<(Board, u8)>,

    /// The list, in order, of all moves made in the game. They should all be
    /// valid moves. The length of `moves` should always be one less than the
    /// length of `history`.
    moves: Vec<Move>,

    /// Stores the number of times a position has been reached in the course of
    /// this game. It is used for three-move-rule draws. The keys are the
    /// Zobrist hashes of the boards previously visited.
    repetitions: HashMap<u64, u64>,
}

impl Game {
    pub fn from_fen(fen: &str) -> Result<Game, &'static str> {
        let b = Board::from_fen(fen)?;
        // TODO extract 50 move rule from the FEN
        Ok(Game {
            history: vec![(b, 0)],
            moves: Vec::new(),
            repetitions: HashMap::from([(b.hash, 1)]),
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

    /// Make a move, assuming said move is illegal. If the history is empty
    /// (this should never happen if normal operations occurred), the move will
    /// be made from the default state of a `Board`.
    pub fn make_move(&mut self, m: Move) {
        let previous_state = self.history.last().unwrap();
        let mut newboard = previous_state.0;

        let move_timeout =
            match newboard.is_move_capture(m) || newboard[Piece::Pawn].contains(m.from_square()) {
                true => 0,
                false => previous_state.1 + 1,
            };
        newboard.make_move(m);

        let num_reps = self.repetitions.entry(newboard.hash).or_insert(0);
        *num_reps += 1;
        self.history.push((newboard, move_timeout));
        self.moves.push(m);
    }

    /// Attempt to play a move, which may or may not be legal. If the move is
    /// legal, the move will be executed and the state will change, then
    /// `Ok(())` will be returned. If not, an `Err` will be returned to inform
    /// you that the move is illegal, and no state will be changed.
    pub fn try_move(&mut self, mgen: &MoveGenerator, m: Move) -> Result<(), &'static str> {
        if self.get_moves(mgen).contains(&m) {
            self.make_move(m);
            Ok(())
        } else {
            Err("illegal move given!")
        }
    }

    /// Undo the most recent move. The return will be `Ok` if there are moves
    /// left to undo, with the internal value being the move that was undone,

    /// and `Err` if there are no moves to undo.
    pub fn undo(&mut self) -> Result<Move, &'static str> {
        let move_removed = match self.moves.pop() {
            Some(m) => m,
            None => return Err("no moves to remove"),
        };
        let state_removed = match self.history.pop() {
            Some(p) => p.0,
            None => return Err("no boards in history"),
        };
        let num_reps = self.repetitions.entry(state_removed.hash).or_insert(1);
        *num_reps -= 1;
        if *num_reps == 0 {
            self.repetitions.remove(&state_removed.hash);
        }

        Ok(move_removed)
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

    #[inline]
    /// Get the current state of the game as a board. Will panic if there is no
    /// history (but this should never happen if the game was initialized
    /// correctly)
    pub fn board(&self) -> &Board {
        &self.history.last().unwrap().0
    }

    /// In the current state, is the game complete (i.e. is there no way the
    /// game can continue)? The return type has the first type as whether the
    /// game is over, and the second is the player which has won if the game is
    /// over. It will be `None` for a draw.
    pub fn is_game_over(&self, mgen: &MoveGenerator) -> (bool, Option<Color>) {
        if self.is_drawn_historically() {
            return (true, None);
        }
        let b = self.board();

        if has_moves(b) {
            return (false, None);
        }

        let king_sq = Square::try_from(b[Piece::King] & b[b.player_to_move]).unwrap();
        match is_square_attacked_by(b, king_sq, !b.player_to_move) {
            true => (true, Some(!b.player_to_move)),
            false => (true, None), // stalemate
        }
    }

    /// Has this game been drawn due to its move history (i.e. due to the 50
    /// move rule or due to repetition)?
    fn is_drawn_historically(&self) -> bool {
        let num_reps = *self.repetitions.get(&self.board().hash).unwrap_or(&0);
        if num_reps >= 3 {
            // draw by repetition
            return true;
        }

        if self.history.last().unwrap().1 >= 50 {
            // draw by 50 move rule
            return true;
        }
        false
    }

    /// Get the legal moves in this position. Will be empty if the position is
    /// drawn or the game is over.
    pub fn get_moves(&self, mgen: &MoveGenerator) -> Vec<Move> {
        if self.is_drawn_historically() {
            return Vec::new();
        }

        get_moves(self.board())
    }

    /// Get the "loud" moves, such as captures and promotions, which are legal
    /// in this position. The definition of "loud" is relatively fluid at the
    /// moment, and may change.
    pub fn get_loud_moves(&self, mgen: &MoveGenerator) -> Vec<Move> {
        if self.is_drawn_historically() {
            return Vec::new();
        }

        get_loud_moves(self.board())
    }

    // no need for `is_empty` since history should always be nonempty
    #[allow(clippy::len_without_is_empty)]
    /// Get the number of total positions in this history of this game.
    pub fn len(&self) -> usize {
        self.history.len()
    }
}

impl Default for Game {
    fn default() -> Self {
        Game {
            history: vec![(Board::default(), 0)],
            moves: Vec::new(),
            repetitions: HashMap::from([(Board::default().hash, 1)]),
        }
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.moves.len() {
            let b = self.history[i].0;
            let m = self.moves[i];
            write!(
                f,
                "{} ",
                algebraic_from_move(m, &b, &MoveGenerator::default())
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::board;
    use crate::base::moves::Move;
    use crate::base::Square;
    use crate::fens::*;

    #[test]
    /// Test that we can play a simple move on a `Game` and have the board
    /// states update accordingly.
    fn test_play_e4() {
        let mut g = Game::default();
        let m = Move::normal(Square::E2, Square::E4);
        let old_board = *g.board();
        g.make_move(Move::normal(Square::E2, Square::E4));
        let new_board = g.board();
        board::tests::test_move_result_helper(old_board, *new_board, m);
    }

    #[test]
    /// Test that a single move can be undone correctly.
    fn test_undo_move() {
        let mut g = Game::default();
        let m = Move::normal(Square::E2, Square::E4);
        g.make_move(m);
        assert_eq!(g.undo(), Ok(m));
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that an undo will fail if there is no history to undo.
    fn test_illegal_undo() {
        let mut g = Game::default();
        assert!(g.undo().is_err());
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that we can undo multiple moves in a row.
    fn test_undo_multiple_moves() {
        let mut g = Game::default();
        let m0 = Move::normal(Square::E2, Square::E4);
        let m1 = Move::normal(Square::E7, Square::E5);
        g.make_move(m0);
        g.make_move(m1);
        assert_eq!(g.undo_n(2), Ok(()));
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that a `Game` becomes exactly the same as what it started as if a
    /// move is undone.
    fn test_undo_equality() {
        let mut g = Game::default();
        g.make_move(Move::normal(Square::E2, Square::E4));
        assert!(g.undo().is_ok());
        assert_eq!(g, Game::default());
    }

    #[test]
    /// Test that undoing a move results in the previous position.
    fn test_undo_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let m = Move::normal(Square::D1, Square::F3);
        g.make_move(m);
        assert_eq!(g.undo(), Ok(m));
        assert_eq!(g, Game::from_fen(FRIED_LIVER_FEN).unwrap());
        assert_eq!(g.board(), &Board::from_fen(FRIED_LIVER_FEN).unwrap());
    }

    #[test]
    /// Test that undoing with no history results in an error.
    fn test_undo_fail() {
        let mut g = Game::default();
        assert!(g.undo().is_err());
    }

    #[test]
    /// Test that a mated position is in fact over.
    fn test_is_mate_over() {
        let g = Game::from_fen(SCHOLARS_MATE_FEN).unwrap();

        let moves = get_moves(g.board());
        for m in moves {
            println!("{m}");
        }
        assert!(!has_moves(g.board()));
        assert_eq!(g.is_game_over(&mgen), (true, Some(Color::White)));
    }

    #[test]
    fn test_is_mate_over_2() {
        let g: Game = Game::from_fen(WHITE_MATED_FEN).unwrap();

        let moves = get_moves(g.board());
        println!("moves: ");
        for m in moves {
            println!("{m}");
        }
        assert!(!has_moves(g.board()));
        assert_eq!(g.is_game_over(&mgen), (true, Some(Color::Black)));
    }

    #[test]
    /// Test that making a mate found in testing results in the game being over.
    fn test_mate_in_1() {
        let mut g = Game::from_fen(MATE_IN_1_FEN).unwrap();

        let m = Move::normal(Square::B6, Square::B8);
        assert!(g.get_moves(&mgen).contains(&m));
        g.make_move(m);
        for m2 in g.get_moves(&mgen) {
            println!("{m2}");
        }
        assert_eq!(g.is_game_over(&mgen), (true, Some(Color::White)));
    }

    #[test]
    /// Test that clearing a board has the same effect of replacing it with a
    /// default board, if the initial state was the initial board state.
    fn test_clear_board() {
        let mut g = Game::default();
        g.make_move(Move::normal(Square::E2, Square::E4));
        g.clear();
        assert_eq!(g, Game::default());
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn test_king_escape_without_capture() {
        let g = Game::from_fen(KING_MUST_ESCAPE_FEN).unwrap();

        let moves = g.get_moves(&mgen);
        let expected_moves = vec![
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in moves.iter() {
            assert!(expected_moves.contains(m));
        }
        for em in expected_moves.iter() {
            assert!(moves.contains(em));
        }
    }
}
