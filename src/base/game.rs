use crate::base::algebraic::algebraic_from_move;
use crate::base::Board;
use crate::base::Color;
use crate::base::Move;
use crate::base::Piece;
use crate::base::Square;

use std::collections::HashMap;
use std::default::Default;
use std::fmt::{Display, Formatter};

use super::Score;
use super::movegen::get_loud_moves;
use super::movegen::get_moves;
use super::movegen::has_moves;
use super::movegen::is_square_attacked_by;
use super::position::PSTEvaluator;
use super::Position;

#[derive(Clone, Debug, Eq, PartialEq)]
/// A struct containing game information, which unlike a `Board`, knows about
/// its history and can do things like repetition timing.
pub struct Game {
    /// The last element in `history` is the current state of the board. The
    /// first element should be the starting position of the game, and in
    /// between are sequential board states from the entire game. The right
    /// half of the tuple is the number of moves since a pawn-move or capture
    /// was made, and should start at 0.
    history: Vec<(Position, u8)>,

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
    pub fn from_fen(fen: &str, evaluator: PSTEvaluator) -> Result<Game, String> {
        let pos = Position::from_fen(fen, evaluator)?;
        // TODO extract 50 move rule from the FEN
        Ok(Game {
            history: vec![(pos, 0)],
            moves: Vec::new(),
            repetitions: HashMap::from([(pos.board.hash, 1)]),
        })
    }

    /// Empty out the history of this game completely, but leave the original
    /// start state of the board.
    pub fn clear(&mut self) {
        self.history.truncate(1);
        let start_pos = self.history[0].0;
        self.moves.clear();
        self.repetitions.clear();
        //since we cleared this, or_insert will always be called
        self.repetitions.entry(start_pos.board.hash).or_insert(1);
    }

    /// Make a move, assuming said move is illegal. If the history is empty
    /// (this should never happen if normal operations occurred), the move will
    /// be made from the default state of a `Board`. `pst_delta` is the
    /// expected gain in evaluation for the player making the move. Typically,
    /// `pst_delta` wil always be positive.
    pub fn make_move(&mut self, m: Move, pst_delta: Score) {
        let previous_state = self.history.last().unwrap();
        let mut new_pos = previous_state.0;

        let move_timeout = match new_pos.board.is_move_capture(m)
            || new_pos.board[Piece::Pawn].contains(m.from_square())
        {
            true => 0,
            false => previous_state.1 + 1,
        };
        new_pos.make_move(m, pst_delta);

        let num_reps = self.repetitions.entry(new_pos.board.hash).or_insert(0);
        *num_reps += 1;
        self.history.push((new_pos, move_timeout));
        self.moves.push(m);
    }

    /// Attempt to play a move, which may or may not be legal. If the move is
    /// legal, the move will be executed and the state will change, then
    /// `Ok(())` will be returned. If not, an `Err` will be returned to inform
    /// you that the move is illegal, and no state will be changed.
    pub fn try_move(&mut self, m: Move, pst_delta: Score) -> Result<(), &'static str> {
        if self.get_moves().contains(&m) {
            self.make_move(m, pst_delta);
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
        let pos_removed = match self.history.pop() {
            Some(p) => p.0,
            None => return Err("no boards in history"),
        };
        let num_reps = self.repetitions.entry(pos_removed.board.hash).or_insert(1);
        *num_reps -= 1;
        if *num_reps == 0 {
            self.repetitions.remove(&pos_removed.board.hash);
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
        &self.position().board
    }

    #[inline]
    /// Get the position representing the current state of the game. Will panic
    /// if there is no history, but this should never happen.
    pub fn position(&self) -> &Position {
        &self.history.last().unwrap().0
    }

    /// In the current state, is the game complete (i.e. is there no way the
    /// game can continue)? The return type has the first type as whether the
    /// game is over, and the second is the player which has won if the game is
    /// over. It will be `None` for a draw.
    pub fn is_over(&self) -> (bool, Option<Color>) {
        if self.is_drawn_historically() {
            return (true, None);
        }
        let pos = self.position();
        let b = self.board();

        if has_moves(pos) {
            return (false, None);
        }

        // we trust that the board is valid here and this will not lead to UB
        let king_sq = unsafe { Square::unsafe_from(b[Piece::King] & b[b.player_to_move]) };
        match is_square_attacked_by(b, king_sq, !b.player_to_move) {
            true => (true, Some(!b.player_to_move)),
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

        if self.history.last().unwrap().1 >= 50 {
            // draw by 50 move rule
            return true;
        }
        false
    }

    /// Get the legal moves in this position. Will be empty if the position is
    /// drawn or the game is over.
    pub fn get_moves(&self) -> Vec<Move> {
        if self.is_drawn_historically() {
            return Vec::new();
        }

        get_moves(self.position())
    }

    /// Get the "loud" moves, such as captures and promotions, which are legal
    /// in this position. The definition of "loud" is relatively fluid at the
    /// moment, and may change.
    pub fn get_loud_moves(&self) -> Vec<Move> {
        if self.is_drawn_historically() {
            return Vec::new();
        }

        get_loud_moves(self.position())
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
            history: vec![(Position::default(), 0)],
            moves: Vec::new(),
            repetitions: HashMap::from([(Board::default().hash, 1)]),
        }
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.moves.len() {
            let pos = &self.history[i].0;
            let m = self.moves[i];
            write!(f, "{} ", algebraic_from_move(m, pos))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{board, Eval};
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
        g.make_move(
            Move::normal(Square::E2, Square::E4),
            (Eval::DRAW, Eval::DRAW),
        );
        let new_board = g.board();
        board::tests::test_move_result_helper(old_board, *new_board, m);
    }

    #[test]
    /// Test that a single move can be undone correctly.
    fn test_undo_move() {
        let mut g = Game::default();
        let m = Move::normal(Square::E2, Square::E4);
        g.make_move(m, (Eval::DRAW, Eval::DRAW));
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
        g.make_move(m0, (Eval::DRAW, Eval::DRAW));
        g.make_move(m1, (Eval::DRAW, Eval::DRAW));
        assert_eq!(g.undo_n(2), Ok(()));
        assert_eq!(*g.board(), Board::default());
    }

    #[test]
    /// Test that a `Game` becomes exactly the same as what it started as if a
    /// move is undone.
    fn test_undo_equality() {
        let mut g = Game::default();
        g.make_move(
            Move::normal(Square::E2, Square::E4),
            (Eval::DRAW, Eval::DRAW),
        );
        assert!(g.undo().is_ok());
        assert_eq!(g, Game::default());
    }

    #[test]
    /// Test that undoing a move results in the previous position.
    fn test_undo_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN, Position::no_eval).unwrap();
        let m = Move::normal(Square::D1, Square::F3);
        g.make_move(m, (Eval::DRAW, Eval::DRAW));
        assert_eq!(g.undo(), Ok(m));
        assert_eq!(
            g,
            Game::from_fen(FRIED_LIVER_FEN, Position::no_eval).unwrap()
        );
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
        let g = Game::from_fen(SCHOLARS_MATE_FEN, Position::no_eval).unwrap();
        let moves = get_moves(g.position());
        for m in moves {
            println!("{m}");
        }
        assert!(!has_moves(g.position()));
        assert_eq!(g.is_over(), (true, Some(Color::White)));
    }

    #[test]
    fn test_is_mate_over_2() {
        let g: Game = Game::from_fen(WHITE_MATED_FEN, Position::no_eval).unwrap();
        let moves = get_moves(g.position());
        println!("moves: ");
        for m in moves {
            println!("{m}");
        }
        assert!(!has_moves(g.position()));
        assert_eq!(g.is_over(), (true, Some(Color::Black)));
    }

    #[test]
    /// Test that making a mate found in testing results in the game being over.
    fn test_mate_in_1() {
        let mut g = Game::from_fen(MATE_IN_1_FEN, Position::no_eval).unwrap();
        let m = Move::normal(Square::B6, Square::B8);
        assert!(g.get_moves().contains(&m));
        g.make_move(m, (Eval::DRAW, Eval::DRAW));
        for m2 in g.get_moves() {
            println!("{m2}");
        }
        assert_eq!(g.is_over(), (true, Some(Color::White)));
    }

    #[test]
    /// Test that clearing a board has the same effect of replacing it with a
    /// default board, if the initial state was the initial board state.
    fn test_clear_board() {
        let mut g = Game::default();
        g.make_move(
            Move::normal(Square::E2, Square::E4),
            (Eval::DRAW, Eval::DRAW),
        );
        g.clear();
        assert_eq!(g, Game::default());
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn test_king_escape_without_capture() {
        let g = Game::from_fen(KING_MUST_ESCAPE_FEN, Position::no_eval).unwrap();
        let moves = g.get_moves();
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
