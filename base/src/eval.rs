use std::{
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::Color;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
/// A wrapper for the evaluation of a position.
/// The higher an evaluation is, the better the position is for White. An
/// evaluation of 0 is a draw.
/// Internally, the i32 represents an integer. The integer value is 1/1000 of a
/// pawn (so if the internal value is +2000, the position is +2 pawns for White)
/// .
///
/// Values > 29,000 are reserved for mates. 30,000 is White to mate in
/// 0 (i.e. White has won the game), 29,999 is White to mate in 1 (White will
/// play their move and mate), 29,998 is White to mate in 1, with Black to
/// move (Black will play their move, then White will play their move to mate)
/// and so on. Values of < -29,000 are reserved for black mates, likewise.
///
/// # Examples
///
/// ```
/// use fiddler_base::Eval;
/// let mate_eval = Eval::mate_in(3);
/// let draw_eval = Eval::DRAW;
/// assert!(mate_eval > draw_eval);
/// ```
pub struct Eval(i16);

/// A `Score` is a pair of `Eval`s. The first element in the score is the
/// midgame evaluation, and the second is the endgame evaluation. At evaluation
/// time, the two evaluations are blended to produce a final evaluation.
pub type Score = (Eval, Eval);

impl Eval {
    /// An evaluation which is smaller than every other "normal" evaluation.
    pub const MIN: Eval = Eval(-Eval::MATE_0_VAL - 1000);

    /// An evaluation which is larger than every other "normal" evaluation.
    pub const MAX: Eval = Eval(Eval::MATE_0_VAL + 1000);

    /// An evaluation where Black has won the game by mate.
    pub const BLACK_MATE: Eval = Eval(-Eval::MATE_0_VAL);

    /// An evaluation where White has won the game by mate.
    pub const WHITE_MATE: Eval = Eval(Eval::MATE_0_VAL);

    /// The evaluation of a drawn position.
    pub const DRAW: Eval = Eval(0);

    /// The internal evaluation of a mate in 0 for White (i.e. White made the
    /// mating move on the previous ply).
    const MATE_0_VAL: i16 = 30_000;

    /// The highest value of a position which is not a mate.
    const MATE_CUTOFF: i16 = 29_000;

    /// The value of one pawn.
    const PAWN_VALUE: i16 = 100;

    #[inline(always)]
    /// Get an evaluation equivalent to the given pawn value.
    pub fn pawns(x: f64) -> Eval {
        Eval((x * Eval::PAWN_VALUE as f64) as i16)
    }

    #[inline(always)]
    /// Construct an `Eval` with the given value in centipawns.
    pub const fn centipawns(x: i16) -> Eval {
        Eval(x)
    }

    #[inline(always)]
    /// Convert a pair of centipawn values into a `Score` containing two
    /// `Evals`.
    pub const fn score(midgame_val: i16, endgame_val: i16) -> Score {
        (Eval::centipawns(midgame_val), Eval::centipawns(endgame_val))
    }

    #[inline(always)]
    /// Create an `Eval` based on the number of half-moves required for White to
    /// mate. `-Eval::mate_in(n)` will give Black to mate in the number of
    /// plies.
    pub const fn mate_in(nplies: u16) -> Eval {
        Eval(Eval::MATE_0_VAL - (nplies as i16))
    }

    #[inline(always)]
    /// Step this evaluation back in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one closer to 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF+1)`, this will result in undefined
    /// behavior.
    /// # Examples
    /// ```
    /// use fiddler_base::Eval;
    /// let current_eval = Eval::mate_in(0);
    /// let previous_ply_eval = current_eval.step_back();
    /// assert_eq!(previous_ply_eval, Eval::mate_in(1));
    /// ```
    pub const fn step_back(&self) -> Eval {
        Eval(self.0 - self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline(always)]
    /// Step this evaluation forward in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one further from 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF)`, this will result in undefined
    /// behavior.
    pub const fn step_forward(&self) -> Eval {
        Eval(self.0 + self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline(always)]
    /// Is this evaluation a mate (i.e. a non-normal evaluation)?
    pub const fn is_mate(&self) -> bool {
        self.0 > Eval::MATE_CUTOFF || self.0 < -Eval::MATE_CUTOFF
    }

    /// Get the number of moves until a mated position, assuming perfect play.
    /// # Examples
    /// ```
    /// use fiddler_base::Eval;
    /// let ev1 = Eval::pawns(2.5);
    /// let ev2 = Eval::mate_in(3);
    /// assert_eq!(ev1.moves_to_mate(), None);
    /// assert_eq!(ev2.moves_to_mate(), Some(2));
    /// ```
    pub const fn moves_to_mate(&self) -> Option<u8> {
        match self.is_mate() {
            true => {
                if self.0 > 0 {
                    // white to mate
                    Some(((Eval::MATE_0_VAL - self.0 + 1) / 2) as u8)
                } else {
                    // black to mate
                    Some(((Eval::MATE_0_VAL + self.0 + 1) / 2) as u8)
                }
            }
            false => None,
        }
    }

    #[inline(always)]
    /// Get the value in centipawns of this evaluation. Will return a number
    /// with magnitude greater than 29000 for mates.
    pub const fn centipawn_val(&self) -> i16 {
        self.0
    }

    #[inline(always)]
    /// Get the value in floating-point pawns of this evaluation.
    pub fn float_val(&self) -> f32 {
        (self.0 as f32) / 100.
    }

    #[inline(always)]
    /// Put this evaluation into the perspective of the given player. 
    /// In essence, if the player is Black, the evaluation will be inverted, but 
    /// if the player is White, the evaluation will remain the same. This 
    /// function is an involution, meaning that calling it twice with the same 
    /// player will yield the original evaluation.
    pub const fn in_perspective(&self, player: Color) -> Eval {
        match player {
            Color::White => *self,
            Color::Black => Eval(-self.0)
        }
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
    #[inline(always)]
    fn mul(self, rhs: u32) -> Self::Output {
        Eval(self.0 * rhs as i16)
    }
}

impl Mul<i16> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: i16) -> Self::Output {
        Eval(self.0 * rhs)
    }
}

impl Mul<i8> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: i8) -> Self::Output {
        Eval(self.0 * (rhs as i16))
    }
}

impl Mul<f32> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Eval((self.0 as f32 * rhs) as i16)
    }
}

impl MulAssign<i16> for Eval {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: i16) {
        self.0 *= rhs;
    }
}

impl AddAssign<Eval> for Eval {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Eval) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Eval> for Eval {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Eval) {
        self.0 -= rhs.0;
    }
}

impl Add<Eval> for Eval {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Eval) -> Eval {
        Eval(self.0 + rhs.0)
    }
}

impl Sub<Eval> for Eval {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Eval) -> Eval {
        Eval(self.0 - rhs.0)
    }
}

impl Neg for Eval {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Eval {
        Eval(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that stepping forward a normal evaluation will make no changes.
    fn test_step_forward_draw() {
        assert_eq!(Eval(0), Eval(0).step_forward());
    }

    #[test]
    /// Test that stepping backward a normal evaluation will make no changes.
    fn test_step_backward_draw() {
        assert_eq!(Eval(0), Eval(0).step_back());
    }

    #[test]
    /// Test that stepping forward the highest non-mate will make no change.
    fn test_step_forward_highest_non_mate() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF),
            Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    /// Test that stepping backward the highest non-mate will make no change.
    fn test_step_bacwkard_highest_non_mate() {
        assert_eq!(Eval(Eval::MATE_CUTOFF), Eval(Eval::MATE_CUTOFF).step_back());
    }

    #[test]
    /// Test that stepping forward the lowest non-mate will make no change.
    fn test_step_forward_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    /// Test that stepping backward the lowest non-mate will make no change.
    fn test_step_bacwkard_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_back()
        );
    }

    #[test]
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
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
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
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
