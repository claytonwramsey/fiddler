use crate::base::algebraic::algebraic_from_move;
use crate::base::Color;
use crate::base::Game;
use crate::base::Move;
use crate::base::MoveGenerator;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

pub mod candidacy;
pub mod greedy;
pub mod positional;
pub mod search;
pub mod transposition;

///
/// A function which can shallowly evaluate a position. The given array of
/// moves is the set of moves in the position, but a move generator is also
/// given in case the function desires it.
///
pub type EvaluationFn = fn(&mut Game, &MoveGenerator) -> Eval;

///
/// A function which can decide how much it "likes" a move.
///
pub type MoveCandidacyFn = fn(&mut Game, &MoveGenerator, Move) -> Eval;

///
/// An `Engine` is something that can evaluate a `Game`, and give moves which
/// it thinks are good. All the public methods require it to be mutable so that
/// the engine can alter its internal state (such as with transposition tables)
/// to update its internal data.
///
pub trait Engine {
    ///
    /// Evaluate the position of the given game. `g` is only given as mutable to
    /// allow this method access to the ability to make and undo moves, but `g`
    /// should be the same before and after its use.
    ///
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval;

    ///
    /// Set the depth of the engine's search functionality. The exact effects
    /// of this method may vary from engine to engine, but it should be
    /// expected that higher depths result in longer search times and better
    /// evaluations.
    ///
    fn set_depth(&mut self, depth: usize);

    ///
    /// Get what this engine believes to be the best move in the given position.
    /// `g` is only given as mutable to allow this method access to the ability
    /// to make and undo moves, but `g` should be the same before and after its
    /// use.
    ///
    fn get_best_move(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Move {
        /*self.get_evals(g, mgen)
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(k, _)| k)
        .unwrap_or(Move::BAD_MOVE)*/
        let player = g.get_board().player_to_move;
        let evals = self.get_evals(g, mgen);
        let mut best_move = Move::BAD_MOVE;
        let mut best_eval = match player {
            Color::White => Eval::MIN,
            _ => Eval::MAX,
        };
        for (m, eval) in evals.iter() {
            if g.get_board().player_to_move == Color::White {
                if best_eval < *eval {
                    best_eval = *eval;
                    best_move = *m;
                }
            } else if best_eval > *eval {
                best_eval = *eval;
                best_move = *m;
            }
        }

        best_move
    }

    ///
    /// Get the evaluation of each move in this position. `g` is only given as
    /// mutable to allow this method access to the ability to make and undo
    /// moves, but `g` should be the same before and after its use.
    ///
    fn get_evals(&mut self, g: &mut Game, mgen: &MoveGenerator) -> HashMap<Move, Eval> {
        let moves = g.get_moves(mgen);
        let mut evals = HashMap::new();
        for m in moves {
            g.make_move(m);
            let ev = self.evaluate(g, mgen);

            //this should never fail since we just made a move, but who knows?
            g.undo().unwrap();
            evals.insert(m, ev);
            println!("{}: {ev}", algebraic_from_move(m, g.get_board(), mgen));
        }

        evals
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
///
/// A wrapper for the evaluation of a position.
/// The higher an evaluation is, the better the position is for White. An
/// evaluation of 0 is a draw.
/// Internally, the i32 represents an integer. The integer value is 1/1000 of a
/// pawn (so if the internal value is +2000, the position is +2 pawns for White)
/// .
///
/// Values > 999,000 are reserved for mates. 1,000,000 is White to mate in
/// 0 (i.e. White has won the game), 999,999 is White to mate in 1 (White will
/// play their move and mate), 999,998 is White to mate in 1, with Black to
/// move (Black will play their move, then White will play their move to mate)
/// and so on. Values of < -999,000 are reserved for black mates, likewise.
///
pub struct Eval(i32);

impl Eval {
    ///
    /// An evaluation which is smaller than every other "normal" evaluation.
    ///
    pub const MIN: Eval = Eval(-2 * Eval::MATE_0_VAL);

    ///
    /// An evaluation which is larger than every other "normal" evaluation.
    ///
    pub const MAX: Eval = Eval(2 * Eval::MATE_0_VAL);

    ///
    /// An evaluation where Black has won the game by mate.
    ///
    pub const BLACK_MATE: Eval = Eval(-Eval::MATE_0_VAL);

    ///
    /// An evaluation where White has won the game by mate.
    ///
    pub const WHITE_MATE: Eval = Eval(Eval::MATE_0_VAL);

    ///
    /// The internal evaluation of a mate in 0 for White (i.e. White made the
    /// mating move on the previous ply).
    ///
    const MATE_0_VAL: i32 = 1_000_000;

    ///
    /// The highest value of a position which is not a mate.
    ///
    const MATE_CUTOFF: i32 = 999_000;

    ///
    /// The value of one pawn.
    ///
    const PAWN_VALUE: i32 = 1_000;

    #[inline]
    ///
    /// Get an evaluation equivalent to the given pawn value.
    ///
    pub fn pawns(x: f64) -> Eval {
        Eval((x * Eval::PAWN_VALUE as f64) as i32)
    }

    #[inline]
    ///
    /// Create an `Eval` based on the number of half-moves required for White to
    /// mate. `-Eval::mate_in(n)` will give Black to mate in the number of
    /// plies.
    ///
    pub const fn mate_in(nplies: u16) -> Eval {
        Eval(Eval::MATE_0_VAL - (nplies as i32))
    }

    #[inline]
    ///
    /// Step this evaluation back in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one closer to 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF+1)`, this will result in undefined
    /// behavior.
    ///
    pub fn step_back(&self) -> Eval {
        Eval(self.0 - self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline]
    ///
    /// Step this evaluation forward in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one further from 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF)`, this will result in undefined
    /// behavior.
    ///
    pub fn step_forward(&self) -> Eval {
        Eval(self.0 + self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline]
    ///
    /// Is this evaluation a mate (i.e. a non-normal evaluation)?
    ///
    pub fn is_mate(&self) -> bool {
        self.0 > Eval::MATE_CUTOFF || self.0 < -Eval::MATE_CUTOFF
    }
}

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 > Eval::MATE_CUTOFF {
            // white to mate
            write!(f, "+M{:.0}", (Eval::MATE_0_VAL - self.0 + 1) / 2)?;
        } else if self.0 < -Eval::MATE_CUTOFF {
            // black to mate
            write!(f, "-M{:.0}", (Eval::MATE_0_VAL + self.0 + 1) / 2)?;
        } else if self.0 == 0 {
            // draw
            write!(f, "00.00")?;
        } else {
            // normal eval
            write!(f, "{:+2.2}", self.0 as f32 / Eval::PAWN_VALUE as f32)?;
        }
        Ok(())
    }
}

impl Mul<u32> for Eval {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        Eval(self.0 * rhs as i32)
    }
}

impl Mul<i32> for Eval {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self::Output {
        Eval(self.0 * rhs)
    }
}

impl AddAssign<Eval> for Eval {
    #[inline]
    fn add_assign(&mut self, rhs: Eval) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Eval> for Eval {
    #[inline]
    fn sub_assign(&mut self, rhs: Eval) {
        self.0 -= rhs.0;
    }
}

impl Add<Eval> for Eval {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Eval) -> Eval {
        Eval(self.0 + rhs.0)
    }
}

impl Sub<Eval> for Eval {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Eval) -> Eval {
        Eval(self.0 - rhs.0)
    }
}

impl Neg for Eval {
    type Output = Self;
    #[inline]
    fn neg(self) -> Eval {
        Eval(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    ///
    /// Test that stepping forward a normal evaluation will make no changes.
    ///
    fn test_step_forward_draw() {
        assert_eq!(Eval(0), Eval(0).step_forward());
    }

    #[test]
    ///
    /// Test that stepping forward a normal evaluation will make no changes.
    ///
    fn test_step_backward_draw() {
        assert_eq!(Eval(0), Eval(0).step_back());
    }

    #[test]
    ///
    /// Test that stepping forward the highest non-mate will make no change.
    ///
    fn test_step_forward_highest_non_mate() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF),
            Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    ///
    /// Test that stepping backward the highest non-mate will make no change.
    ///
    fn test_step_bacwkard_highest_non_mate() {
        assert_eq!(Eval(Eval::MATE_CUTOFF), Eval(Eval::MATE_CUTOFF).step_back());
    }

    #[test]
    ///
    /// Test that stepping forward the lowest non-mate will make no change.
    ///
    fn test_step_forward_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    ///
    /// Test that stepping forward the lowest non-mate will make no change.
    ///
    fn test_step_bacwkard_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_back()
        );
    }

    #[test]
    ///
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
    ///
    fn test_step_forward_tightest_mates() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF + 2),
            Eval(Eval::MATE_CUTOFF + 1).step_forward()
        );
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF + 2),
            -Eval(Eval::MATE_CUTOFF + 1).step_forward()
        );
    }
    #[test]
    ///
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
    ///
    fn test_step_backward_tightest_mates() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF + 1),
            Eval(Eval::MATE_CUTOFF + 2).step_back()
        );
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF + 1),
            -Eval(Eval::MATE_CUTOFF + 2).step_back()
        );
    }
}
