use std::fmt::{Display, Formatter};


const MATE_VAL: i32 = 1_000_000;

const PAWN_VALUE: i32 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
/**
 * A wrapper for the evaluation of a position. 
 * The higher an evaluation is, the better the position is for White. An 
 * evaluation of 0 is a draw.
 * 
 * Internally, the i32 represents an integer. The integer value is 1/1000 of a 
 * pawn (so if the internal value is +2000, the position is +2 pawns for White).
 * 
 * Values >= 1,000,000 are reserved for mates. 1,000,000 is White to mate in 
 * 0 (i.e. White has won the game), 1,000,001 is White to mate in 1 (White will 
 * play their move and mate), 1,000,002 is White to mate in 1, with Black to 
 * move (Black will play their move, then White will play their move to mate) 
 * and so on. Values of <= -1,000,000 are reserved for black mates, likewise.
 */
pub struct Eval(i32);

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 >= MATE_VAL {
            // white to mate
            write!(f, "+M{:2.0}", self.0 - MATE_VAL)?;
        } else if self.0 <= -MATE_VAL {
            // black to mate
            write!(f, "-M{:2.0}", -self.0 - MATE_VAL)?;
        }
        else if self.0 == 0 {
            // draw
            write!(f, "00.00")?;
        } else {
            // normal eval
            write!(f, "{:+2.2}", self.0 as f32 / PAWN_VALUE as f32)?;
        }
        Ok(())
    }
}